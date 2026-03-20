use std::collections::{BTreeSet, HashMap};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    CallExpression, Class, Declaration, ExportDefaultDeclarationKind, Expression,
    ImportDeclarationSpecifier, ImportExpression, JSXElementName, JSXMemberExpression,
    JSXMemberExpressionObject, JSXOpeningElement, NewExpression, Program, Statement,
    TSInterfaceDeclaration, TSTypeName, TSTypeReference, VariableDeclaration,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};

use super::super::{GraphExtraction, GraphRef, GraphSourceKind, GraphSymbol};
use super::heuristic::extract_javascript_heuristic;

pub(super) fn extract_javascript_ast_first(kind: GraphSourceKind, source: &str) -> GraphExtraction {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(kind.synthetic_path()).unwrap_or_else(|_| {
        SourceType::default()
            .with_javascript(true)
            .with_typescript(kind.include_type_symbols())
            .with_jsx(matches!(
                kind,
                GraphSourceKind::JavaScript { jsx: true }
                    | GraphSourceKind::TypeScript { jsx: true }
            ))
    });
    let parsed = Parser::new(&allocator, source, source_type).parse();
    let mut collector = JsGraphCollector::new(source);
    collector.collect_top_level(&parsed.program);
    collector.visit_program(&parsed.program);
    let graph = collector.finish();

    match classify_ast_result(&parsed, &graph, source) {
        AstResultPolicy::KeepAst => graph,
        AstResultPolicy::FallbackHeuristic => {
            extract_javascript_heuristic(source, kind.include_type_symbols())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AstResultPolicy {
    KeepAst,
    FallbackHeuristic,
}

fn classify_ast_result(
    parsed: &oxc_parser::ParserReturn<'_>,
    graph: &GraphExtraction,
    source: &str,
) -> AstResultPolicy {
    if parsed.panicked {
        return AstResultPolicy::FallbackHeuristic;
    }
    if parsed.errors.is_empty() {
        return AstResultPolicy::KeepAst;
    }
    if graph_has_payload(graph) || source.trim().is_empty() {
        AstResultPolicy::KeepAst
    } else {
        AstResultPolicy::FallbackHeuristic
    }
}

fn graph_has_payload(graph: &GraphExtraction) -> bool {
    !(graph.symbols.is_empty() && graph.deps.is_empty() && graph.refs.is_empty())
}

#[derive(Debug)]
struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let mut starts = vec![0];
        for (idx, ch) in source.char_indices() {
            if ch == '\n' {
                starts.push(idx + 1);
            }
        }
        Self { starts }
    }

    fn locate(&self, source: &str, offset: u32) -> (Option<usize>, Option<usize>) {
        if self.starts.is_empty() {
            return (None, None);
        }
        let offset = usize::try_from(offset)
            .unwrap_or(source.len())
            .min(source.len());
        let line_idx = self
            .starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        let Some(line_start) = self.starts.get(line_idx).copied() else {
            return (None, None);
        };
        let column = source[line_start..offset].chars().count() + 1;
        (Some(line_idx + 1), Some(column))
    }
}

#[derive(Default)]
struct ImportAliases {
    renamed: HashMap<String, String>,
    namespaces: BTreeSet<String>,
}

impl ImportAliases {
    fn insert_identity(&mut self, local: &str) {
        self.renamed
            .entry(local.to_string())
            .or_insert_with(|| local.to_string());
    }

    fn insert_renamed(&mut self, local: &str, canonical: &str) {
        self.renamed
            .insert(local.to_string(), canonical.to_string());
    }

    fn insert_namespace(&mut self, local: &str) {
        self.namespaces.insert(local.to_string());
    }

    fn normalize(&self, name: &str) -> String {
        self.renamed
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }
}

struct JsGraphCollector<'s> {
    source: &'s str,
    lines: LineIndex,
    aliases: ImportAliases,
    symbols: BTreeSet<GraphSymbol>,
    deps: BTreeSet<String>,
    refs: BTreeSet<GraphRef>,
}

impl<'s> JsGraphCollector<'s> {
    fn new(source: &'s str) -> Self {
        Self {
            source,
            lines: LineIndex::new(source),
            aliases: ImportAliases::default(),
            symbols: BTreeSet::new(),
            deps: BTreeSet::new(),
            refs: BTreeSet::new(),
        }
    }

    fn finish(self) -> GraphExtraction {
        GraphExtraction {
            symbols: self.symbols.into_iter().collect(),
            deps: self.deps.into_iter().collect(),
            refs: self.refs.into_iter().collect(),
        }
    }

    fn collect_top_level<'a>(&mut self, program: &Program<'a>) {
        for statement in &program.body {
            self.collect_statement(statement);
        }
    }

    fn collect_statement<'a>(&mut self, statement: &Statement<'a>) {
        match statement {
            Statement::FunctionDeclaration(function) => self.record_function_symbol(function),
            Statement::ClassDeclaration(class) => self.record_class_symbol(class),
            Statement::VariableDeclaration(variable) => self.record_variable_symbols(variable),
            Statement::TSTypeAliasDeclaration(alias) => {
                self.record_binding_symbol(&alias.id, "type");
            }
            Statement::TSInterfaceDeclaration(interface) => {
                self.record_binding_symbol(&interface.id, "interface");
            }
            Statement::TSEnumDeclaration(enum_decl) => {
                self.record_binding_symbol(&enum_decl.id, "enum");
            }
            Statement::ImportDeclaration(import_decl) => self.record_import(import_decl),
            Statement::ExportAllDeclaration(export_all) => {
                self.deps.insert(export_all.source.value.to_string());
            }
            Statement::ExportNamedDeclaration(export_named) => {
                if let Some(source) = &export_named.source {
                    self.deps.insert(source.value.to_string());
                }
                if let Some(declaration) = &export_named.declaration {
                    self.collect_declaration(declaration);
                }
                for specifier in &export_named.specifiers {
                    self.record_ref(
                        self.aliases.normalize(specifier.local.name().as_ref()),
                        specifier.local.span(),
                    );
                }
            }
            Statement::ExportDefaultDeclaration(export_default) => {
                self.collect_export_default(&export_default.declaration);
            }
            _ => {}
        }
    }

    fn collect_declaration<'a>(&mut self, declaration: &Declaration<'a>) {
        match declaration {
            Declaration::FunctionDeclaration(function) => self.record_function_symbol(function),
            Declaration::ClassDeclaration(class) => self.record_class_symbol(class),
            Declaration::VariableDeclaration(variable) => self.record_variable_symbols(variable),
            Declaration::TSTypeAliasDeclaration(alias) => {
                self.record_binding_symbol(&alias.id, "type");
            }
            Declaration::TSInterfaceDeclaration(interface) => {
                self.record_binding_symbol(&interface.id, "interface");
            }
            Declaration::TSEnumDeclaration(enum_decl) => {
                self.record_binding_symbol(&enum_decl.id, "enum");
            }
            _ => {}
        }
    }

    fn collect_export_default<'a>(&mut self, declaration: &ExportDefaultDeclarationKind<'a>) {
        match declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                self.record_function_symbol(function);
            }
            ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                self.record_class_symbol(class);
            }
            ExportDefaultDeclarationKind::TSInterfaceDeclaration(interface) => {
                self.record_binding_symbol(&interface.id, "interface");
            }
            _ => {}
        }
    }

    fn record_import<'a>(&mut self, import_decl: &oxc_ast::ast::ImportDeclaration<'a>) {
        self.deps.insert(import_decl.source.value.to_string());
        let Some(specifiers) = &import_decl.specifiers else {
            return;
        };
        for specifier in specifiers {
            match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                    let local = specifier.local.name.as_str();
                    let canonical = specifier.imported.name().to_string();
                    self.aliases.insert_renamed(local, &canonical);
                    self.record_ref(canonical, specifier.local.span);
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                    let local = specifier.local.name.as_str();
                    self.aliases.insert_identity(local);
                    self.record_ref(local.to_string(), specifier.local.span);
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                    let local = specifier.local.name.as_str();
                    self.aliases.insert_namespace(local);
                    self.aliases.insert_identity(local);
                }
            }
        }
    }

    fn record_variable_symbols<'a>(&mut self, variable: &VariableDeclaration<'a>) {
        for declarator in &variable.declarations {
            let Some(name) = declarator
                .id
                .get_binding_identifier()
                .map(|binding| binding.name.to_string())
            else {
                continue;
            };
            let Some(init) = &declarator.init else {
                continue;
            };
            let kind = match init {
                Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
                    "function"
                }
                Expression::ClassExpression(_) => "class",
                _ => continue,
            };
            self.record_symbol(name, kind, declarator.id.span());
        }
    }

    fn record_function_symbol<'a>(&mut self, function: &oxc_ast::ast::Function<'a>) {
        let Some(id) = &function.id else {
            return;
        };
        self.record_binding_symbol(id, "function");
    }

    fn record_class_symbol<'a>(&mut self, class: &Class<'a>) {
        let Some(id) = &class.id else {
            return;
        };
        self.record_binding_symbol(id, "class");
    }

    fn record_binding_symbol<'a>(
        &mut self,
        binding: &oxc_ast::ast::BindingIdentifier<'a>,
        kind: &str,
    ) {
        self.record_symbol(binding.name.to_string(), kind, binding.span);
    }

    fn record_symbol(&mut self, name: String, kind: &str, span: Span) {
        let (line, column) = self.lines.locate(self.source, span.start);
        self.symbols.insert(GraphSymbol {
            name,
            kind: kind.to_string(),
            line,
            column,
        });
    }

    fn record_ref(&mut self, symbol: String, span: Span) {
        if symbol.is_empty() {
            return;
        }
        let (line, column) = self.lines.locate(self.source, span.start);
        self.refs.insert(GraphRef {
            symbol,
            line,
            column,
        });
    }

    fn record_expression_ref<'a>(&mut self, expression: &Expression<'a>) {
        if let Some(symbol) = expression_to_symbol(expression, &self.aliases) {
            self.record_ref(symbol, expression.span());
        }
    }

    fn record_type_name_ref<'a>(&mut self, type_name: &TSTypeName<'a>, span: Span) {
        self.record_ref(type_name_to_symbol(type_name, &self.aliases), span);
    }
}

impl<'a> Visit<'a> for JsGraphCollector<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(dep) = extract_static_string_arg(it.arguments.first()) {
            if matches!(&it.callee, Expression::Identifier(identifier) if identifier.name == "require")
            {
                self.deps.insert(dep);
            }
        }
        self.record_expression_ref(&it.callee);
        walk::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.record_expression_ref(&it.callee);
        walk::walk_new_expression(self, it);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        if let Expression::StringLiteral(literal) = &it.source {
            self.deps.insert(literal.value.to_string());
        }
        walk::walk_import_expression(self, it);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if let Some(super_class) = &it.super_class {
            self.record_expression_ref(super_class);
        }
        for implemented in &it.implements {
            self.record_type_name_ref(&implemented.expression, implemented.span);
        }
        walk::walk_class(self, it);
    }

    fn visit_ts_interface_declaration(&mut self, it: &TSInterfaceDeclaration<'a>) {
        for heritage in &it.extends {
            self.record_expression_ref(&heritage.expression);
        }
        walk::walk_ts_interface_declaration(self, it);
    }

    fn visit_ts_type_reference(&mut self, it: &TSTypeReference<'a>) {
        self.record_type_name_ref(&it.type_name, it.span);
        walk::walk_ts_type_reference(self, it);
    }

    fn visit_jsx_opening_element(&mut self, it: &JSXOpeningElement<'a>) {
        if let Some(symbol) = jsx_name_to_symbol(&it.name, &self.aliases) {
            self.record_ref(symbol, it.span);
        }
        walk::walk_jsx_opening_element(self, it);
    }
}

fn extract_static_string_arg(argument: Option<&oxc_ast::ast::Argument<'_>>) -> Option<String> {
    let argument = argument?;
    match argument {
        oxc_ast::ast::Argument::StringLiteral(literal) => Some(literal.value.to_string()),
        _ => None,
    }
}

fn expression_to_symbol(expression: &Expression<'_>, aliases: &ImportAliases) -> Option<String> {
    match expression {
        Expression::Identifier(identifier) => Some(aliases.normalize(identifier.name.as_str())),
        Expression::StaticMemberExpression(member) => {
            let base = expression_to_symbol(&member.object, aliases)?;
            Some(format!("{base}.{}", member.property.name))
        }
        Expression::ParenthesizedExpression(inner) => {
            expression_to_symbol(&inner.expression, aliases)
        }
        Expression::ChainExpression(chain) => chain_element_to_symbol(&chain.expression, aliases),
        Expression::TSAsExpression(expr) => expression_to_symbol(&expr.expression, aliases),
        Expression::TSSatisfiesExpression(expr) => expression_to_symbol(&expr.expression, aliases),
        Expression::TSTypeAssertion(expr) => expression_to_symbol(&expr.expression, aliases),
        Expression::TSNonNullExpression(expr) => expression_to_symbol(&expr.expression, aliases),
        Expression::TSInstantiationExpression(expr) => {
            expression_to_symbol(&expr.expression, aliases)
        }
        _ => None,
    }
}

fn chain_element_to_symbol(
    element: &oxc_ast::ast::ChainElement<'_>,
    aliases: &ImportAliases,
) -> Option<String> {
    match element {
        oxc_ast::ast::ChainElement::CallExpression(call) => {
            expression_to_symbol(&call.callee, aliases)
        }
        oxc_ast::ast::ChainElement::StaticMemberExpression(member) => {
            let base = expression_to_symbol(&member.object, aliases)?;
            Some(format!("{base}.{}", member.property.name))
        }
        oxc_ast::ast::ChainElement::ComputedMemberExpression(_) => None,
        oxc_ast::ast::ChainElement::PrivateFieldExpression(_) => None,
        oxc_ast::ast::ChainElement::TSNonNullExpression(expr) => {
            expression_to_symbol(&expr.expression, aliases)
        }
    }
}

fn type_name_to_symbol(type_name: &TSTypeName<'_>, aliases: &ImportAliases) -> String {
    match type_name {
        TSTypeName::IdentifierReference(identifier) => aliases.normalize(identifier.name.as_str()),
        TSTypeName::QualifiedName(name) => format!(
            "{}.{}",
            type_name_to_symbol(&name.left, aliases),
            name.right.name
        ),
    }
}

fn jsx_name_to_symbol(name: &JSXElementName<'_>, aliases: &ImportAliases) -> Option<String> {
    match name {
        JSXElementName::IdentifierReference(identifier) => {
            Some(aliases.normalize(identifier.name.as_str()))
        }
        JSXElementName::MemberExpression(member) => {
            jsx_member_expression_to_symbol(member, aliases)
        }
        _ => None,
    }
}

fn jsx_member_expression_to_symbol(
    member: &JSXMemberExpression<'_>,
    aliases: &ImportAliases,
) -> Option<String> {
    let base = jsx_member_object_to_symbol(&member.object, aliases)?;
    Some(format!("{base}.{}", member.property.name))
}

fn jsx_member_object_to_symbol(
    object: &JSXMemberExpressionObject<'_>,
    aliases: &ImportAliases,
) -> Option<String> {
    match object {
        JSXMemberExpressionObject::IdentifierReference(identifier) => {
            Some(aliases.normalize(identifier.name.as_str()))
        }
        JSXMemberExpressionObject::MemberExpression(member) => {
            jsx_member_expression_to_symbol(member, aliases)
        }
        JSXMemberExpressionObject::ThisExpression(_) => None,
    }
}
