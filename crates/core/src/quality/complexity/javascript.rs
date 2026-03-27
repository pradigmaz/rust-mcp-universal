use oxc_allocator::Allocator;
use oxc_ast::ast::{ArrowFunctionExpression, Function};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_syntax::scope::ScopeFlags;

use super::common::{
    ComplexityCounts, LineIndex, count_ternary_operators, is_return_statement, observed,
    strip_line_comment, update_max,
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
    if parsed.panicked || !parsed.errors.is_empty() {
        return HotspotFacts::default();
    }

    let mut collector = JsComplexityCollector::new(source);
    collector.visit_program(&parsed.program);
    collector.facts
}

struct JsComplexityCollector<'a> {
    source: &'a str,
    line_index: LineIndex,
    facts: HotspotFacts,
}

impl<'a> JsComplexityCollector<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            line_index: LineIndex::new(source),
            facts: HotspotFacts::default(),
        }
    }

    fn record_function_body(&mut self, start: usize, body_start: usize, body_end: usize) {
        let location = self.line_index.span_location(self.source, start, body_end);
        let body = safe_function_body_slice(self.source, body_start, body_end);
        let counts = scan_javascript_complexity(body);
        update_max(
            &mut self.facts.max_cyclomatic_complexity,
            observed(
                counts.cyclomatic,
                location.clone(),
                QualitySource::ParserLight,
            ),
        );
        update_max(
            &mut self.facts.max_cognitive_complexity,
            observed(
                counts.cognitive,
                location.clone(),
                QualitySource::ParserLight,
            ),
        );
        update_max(
            &mut self.facts.max_branch_count,
            observed(
                counts.branch_count,
                location.clone(),
                QualitySource::ParserLight,
            ),
        );
        update_max(
            &mut self.facts.max_early_return_count,
            observed(
                counts.early_return_count,
                location,
                QualitySource::ParserLight,
            ),
        );
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

impl<'a> Visit<'a> for JsComplexityCollector<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if let Some(body) = &it.body {
            let start = it
                .id
                .as_ref()
                .map(|id| usize::try_from(id.span.start).unwrap_or(0))
                .unwrap_or(usize::try_from(it.span.start).unwrap_or(0));
            self.record_function_body(
                start,
                usize::try_from(body.span.start).unwrap_or(0),
                usize::try_from(body.span.end).unwrap_or(self.source.len()),
            );
        }
        walk::walk_function(self, it, flags);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.record_function_body(
            usize::try_from(it.span.start).unwrap_or(0),
            usize::try_from(it.body.span.start).unwrap_or(0),
            usize::try_from(it.body.span.end).unwrap_or(self.source.len()),
        );
        walk::walk_arrow_function_expression(self, it);
    }
}

fn scan_javascript_complexity(body: &str) -> ComplexityCounts {
    let mut branch_count = count_ternary_operators(body);
    let mut cognitive = branch_count;
    let mut nesting_depth = 0_i64;
    let mut control_stack = Vec::<bool>::new();
    let mut returns = Vec::<(usize, i64)>::new();
    let lines = body.lines().collect::<Vec<_>>();

    for (idx, raw_line) in lines.iter().enumerate() {
        let trimmed = strip_line_comment(raw_line, "//").trim();
        for _ in 0..trimmed.matches('}').count() {
            if control_stack.pop().unwrap_or(false) {
                nesting_depth = nesting_depth.saturating_sub(1);
            }
        }
        if trimmed.is_empty() {
            continue;
        }
        if is_return_statement(trimmed) {
            returns.push((idx, nesting_depth));
        }

        let sites = javascript_branch_sites(trimmed);
        if sites > 0 {
            branch_count += sites;
            cognitive += sites.saturating_mul(1 + nesting_depth);
        }

        let open_count = trimmed.matches('{').count();
        if open_count == 0 {
            continue;
        }
        if javascript_introduces_control_block(trimmed) {
            nesting_depth += 1;
            control_stack.push(true);
            control_stack.extend(std::iter::repeat_n(false, open_count.saturating_sub(1)));
        } else {
            control_stack.extend(std::iter::repeat_n(false, open_count));
        }
    }

    let final_top_level_return = last_significant_return_line(&lines).filter(|line_idx| {
        returns
            .iter()
            .any(|(idx, depth)| idx == line_idx && *depth == 0)
    });
    let early_return_count = i64::try_from(
        returns
            .iter()
            .filter(|(idx, depth)| *depth > 0 || Some(*idx) != final_top_level_return)
            .count(),
    )
    .unwrap_or(i64::MAX);
    ComplexityCounts::from_parts(branch_count, cognitive, early_return_count)
}

fn javascript_branch_sites(trimmed: &str) -> i64 {
    let case_count = i64::try_from(
        usize::from(trimmed.starts_with("case ")) + usize::from(trimmed.starts_with("default:")),
    )
    .unwrap_or(i64::MAX);
    if case_count > 0 {
        return case_count;
    }
    if trimmed.starts_with("else if ")
        || trimmed.starts_with("if ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("do ")
        || trimmed.starts_with("do{")
        || trimmed.starts_with("catch ")
        || trimmed.starts_with("catch(")
        || trimmed.starts_with("switch ")
        || trimmed.starts_with("switch(")
    {
        return 1;
    }
    0
}

fn javascript_introduces_control_block(trimmed: &str) -> bool {
    trimmed.contains('{')
        && (trimmed.starts_with("if ")
            || trimmed.starts_with("else if ")
            || trimmed.starts_with("else")
            || trimmed.starts_with("for ")
            || trimmed.starts_with("while ")
            || trimmed.starts_with("do ")
            || trimmed.starts_with("switch ")
            || trimmed.starts_with("catch ")
            || trimmed.starts_with("try ")
            || trimmed.starts_with("try{"))
}

fn last_significant_return_line(lines: &[&str]) -> Option<usize> {
    lines.iter().enumerate().rev().find_map(|(idx, line)| {
        let trimmed = strip_line_comment(line, "//")
            .trim()
            .trim_matches(|ch: char| matches!(ch, '{' | '}' | ';'));
        if trimmed.is_empty() {
            return None;
        }
        is_return_statement(trimmed).then_some(idx)
    })
}
