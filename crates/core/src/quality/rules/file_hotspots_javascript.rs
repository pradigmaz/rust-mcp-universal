use oxc_allocator::Allocator;
use oxc_ast::ast::{ArrowFunctionExpression, Class, ClassElement, Function, Statement};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_syntax::scope::ScopeFlags;

use super::file_hotspots_common::{
    LineIndex, count_top_level_parameters, observed, scan_braced_control_nesting, update_max,
};
use crate::model::QualitySource;
use crate::quality::HotspotFacts;

pub(super) fn analyze(rel_path: &str, language: &str, source: &str) -> HotspotFacts {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(rel_path).unwrap_or_else(|_| {
        SourceType::default()
            .with_typescript(matches!(language, "typescript" | "tsx"))
            .with_javascript(matches!(language, "javascript" | "jsx" | "mjs" | "cjs"))
            .with_jsx(matches!(language, "jsx" | "tsx"))
    });
    let parsed = Parser::new(&allocator, source, source_type).parse();
    let mut facts = HotspotFacts {
        max_export_count_per_file: Some(observed(
            count_exports(&parsed.program.body),
            None,
            QualitySource::Ast,
        )),
        ..HotspotFacts::default()
    };
    if parsed.panicked || !parsed.errors.is_empty() {
        return facts;
    }

    let mut collector = JsHotspotCollector::new(source);
    collector.visit_program(&parsed.program);
    facts.max_function_lines = collector.max_function_lines;
    facts.max_parameters_per_function = collector.max_parameters_per_function;
    facts.max_nesting_depth = collector.max_nesting_depth;
    facts.max_class_member_count = collector.max_class_member_count;
    facts
}

fn count_exports(body: &oxc_allocator::Vec<'_, Statement<'_>>) -> i64 {
    let mut count = 0_i64;
    for statement in body {
        count += match statement {
            Statement::ExportDefaultDeclaration(_) | Statement::ExportAllDeclaration(_) => 1,
            Statement::ExportNamedDeclaration(decl) => {
                if !decl.specifiers.is_empty() {
                    i64::try_from(decl.specifiers.len()).unwrap_or(i64::MAX)
                } else if let Some(declaration) = &decl.declaration {
                    match declaration {
                        oxc_ast::ast::Declaration::FunctionDeclaration(_)
                        | oxc_ast::ast::Declaration::ClassDeclaration(_)
                        | oxc_ast::ast::Declaration::TSTypeAliasDeclaration(_)
                        | oxc_ast::ast::Declaration::TSInterfaceDeclaration(_)
                        | oxc_ast::ast::Declaration::TSEnumDeclaration(_) => 1,
                        oxc_ast::ast::Declaration::VariableDeclaration(variable) => {
                            i64::try_from(variable.declarations.len()).unwrap_or(i64::MAX)
                        }
                        _ => 0,
                    }
                } else {
                    0
                }
            }
            _ => 0,
        };
    }
    count
}

struct JsHotspotCollector<'s> {
    source: &'s str,
    lines: LineIndex,
    max_function_lines: Option<crate::quality::ObservedMetric>,
    max_parameters_per_function: Option<crate::quality::ObservedMetric>,
    max_nesting_depth: Option<crate::quality::ObservedMetric>,
    max_class_member_count: Option<crate::quality::ObservedMetric>,
}

impl<'s> JsHotspotCollector<'s> {
    fn new(source: &'s str) -> Self {
        Self {
            source,
            lines: LineIndex::new(source),
            max_function_lines: None,
            max_parameters_per_function: None,
            max_nesting_depth: None,
            max_class_member_count: None,
        }
    }

    fn record_function_like(
        &mut self,
        span_start: usize,
        body_start: usize,
        body_end: usize,
        params_text: &str,
    ) {
        let location = self.lines.span_location(self.source, span_start, body_end);
        if let Some(location) = location.clone() {
            update_max(
                &mut self.max_function_lines,
                observed(
                    i64::try_from(location.end_line.saturating_sub(location.start_line) + 1)
                        .unwrap_or(i64::MAX),
                    Some(location.clone()),
                    QualitySource::Ast,
                ),
            );
            update_max(
                &mut self.max_parameters_per_function,
                observed(
                    count_top_level_parameters(params_text),
                    Some(location.clone()),
                    QualitySource::Ast,
                ),
            );
            let nesting_source = safe_function_body_slice(self.source, body_start, body_end);
            update_max(
                &mut self.max_nesting_depth,
                observed(
                    scan_braced_control_nesting(
                        nesting_source,
                        &["if", "for", "while", "switch", "catch", "else if"],
                    ),
                    Some(location),
                    QualitySource::ParserLight,
                ),
            );
        }
    }
}

impl<'a> Visit<'a> for JsHotspotCollector<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let start = it
            .id
            .as_ref()
            .map(|id| id.span.start)
            .unwrap_or(it.span.start);
        let end = it
            .body
            .as_ref()
            .map(|body| body.span.end)
            .unwrap_or(it.span.end);
        self.record_function_like(
            usize::try_from(start).unwrap_or(0),
            it.body
                .as_ref()
                .map(|body| usize::try_from(body.span.start).unwrap_or(0))
                .unwrap_or(usize::try_from(it.span.start).unwrap_or(0)),
            usize::try_from(end).unwrap_or(self.source.len()),
            safe_slice(
                self.source,
                usize::try_from(it.params.span.start).unwrap_or(0),
                usize::try_from(it.params.span.end).unwrap_or(self.source.len()),
            ),
        );
        walk::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.record_function_like(
            usize::try_from(it.span.start).unwrap_or(0),
            usize::try_from(it.body.span.start).unwrap_or(0),
            usize::try_from(it.body.span.end).unwrap_or(self.source.len()),
            safe_slice(
                self.source,
                usize::try_from(it.params.span.start).unwrap_or(0),
                usize::try_from(it.params.span.end).unwrap_or(self.source.len()),
            ),
        );
        walk::walk_arrow_function_expression(self, it);
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        let member_count = i64::try_from(
            it.body
                .body
                .iter()
                .filter(|element| !matches!(element, ClassElement::StaticBlock(_)))
                .count(),
        )
        .unwrap_or(i64::MAX);
        let location = self.lines.span_location(
            self.source,
            usize::try_from(
                it.id
                    .as_ref()
                    .map(|id| id.span.start)
                    .unwrap_or(it.span.start),
            )
            .unwrap_or(0),
            usize::try_from(it.span.end).unwrap_or(self.source.len()),
        );
        update_max(
            &mut self.max_class_member_count,
            observed(member_count, location, QualitySource::Ast),
        );
        walk::walk_class(self, it);
    }
}

fn safe_function_body_slice(source: &str, body_start: usize, body_end: usize) -> &str {
    let body = safe_slice(source, body_start, body_end);
    let trimmed = body.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') && body.len() >= 2 {
        let inner_start = body.find('{').map(|idx| idx + 1).unwrap_or(0);
        let inner_end = body.rfind('}').unwrap_or(body.len());
        return &body[inner_start.min(inner_end)..inner_end];
    }
    body
}

fn safe_slice(source: &str, start: usize, end: usize) -> &str {
    let clamped_start = start.min(source.len());
    let clamped_end = end.min(source.len());
    if clamped_start >= clamped_end {
        return "";
    }
    &source[clamped_start..clamped_end]
}
