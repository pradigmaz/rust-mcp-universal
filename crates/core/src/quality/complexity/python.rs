use super::common::{
    ComplexityCounts, function_span_location, is_return_statement, observed, update_max,
};
use crate::model::QualitySource;
use crate::quality::HotspotFacts;

pub(super) fn analyze(source: &str) -> HotspotFacts {
    let lines = source.lines().collect::<Vec<_>>();
    let mut facts = HotspotFacts::default();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len().saturating_sub(trimmed.len());
        if parse_function_header(trimmed).is_none() {
            continue;
        }
        let end_line = block_end(&lines, idx, indent);
        let location = function_span_location(&lines, idx + 1, end_line);
        let counts = scan_python_complexity(&lines, idx + 1, end_line, indent);
        record_counts(&mut facts, location, counts);
    }

    facts
}

fn record_counts(
    facts: &mut HotspotFacts,
    location: Option<crate::model::QualityLocation>,
    counts: ComplexityCounts,
) {
    update_max(
        &mut facts.max_cyclomatic_complexity,
        observed(
            counts.cyclomatic,
            location.clone(),
            QualitySource::ParserLight,
        ),
    );
    update_max(
        &mut facts.max_cognitive_complexity,
        observed(
            counts.cognitive,
            location.clone(),
            QualitySource::ParserLight,
        ),
    );
    update_max(
        &mut facts.max_branch_count,
        observed(
            counts.branch_count,
            location.clone(),
            QualitySource::ParserLight,
        ),
    );
    update_max(
        &mut facts.max_early_return_count,
        observed(
            counts.early_return_count,
            location,
            QualitySource::ParserLight,
        ),
    );
}

fn scan_python_complexity(
    lines: &[&str],
    start_idx: usize,
    end_line: usize,
    base_indent: usize,
) -> ComplexityCounts {
    let body_indent = first_body_indent(lines, start_idx, end_line, base_indent);
    let mut stack = Vec::<usize>::new();
    let mut branch_count = 0_i64;
    let mut cognitive = 0_i64;
    let mut returns = Vec::<(usize, bool)>::new();

    for (idx, line) in lines.iter().enumerate().take(end_line).skip(start_idx) {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('@') {
            continue;
        }
        let indent = line.len().saturating_sub(trimmed.len());
        while stack.last().is_some_and(|value| *value >= indent) {
            stack.pop();
        }

        if is_return_statement(trimmed) {
            returns.push((idx, indent == body_indent));
        }

        let sites = python_branch_sites(trimmed);
        if sites > 0 {
            branch_count += sites;
            cognitive += sites.saturating_mul(1 + i64::try_from(stack.len()).unwrap_or(i64::MAX));
        }

        if starts_python_block(trimmed) && indent > base_indent {
            stack.push(indent);
        }
    }

    let final_top_level_return = lines
        .iter()
        .enumerate()
        .take(end_line)
        .skip(start_idx)
        .rev()
        .find_map(|(idx, line)| {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('@') {
                return None;
            }
            let indent = line.len().saturating_sub(trimmed.len());
            (indent == body_indent && is_return_statement(trimmed)).then_some(idx)
        });
    let early_return_count = i64::try_from(
        returns
            .iter()
            .filter(|(idx, is_top_level)| !(*is_top_level && Some(*idx) == final_top_level_return))
            .count(),
    )
    .unwrap_or(i64::MAX);
    ComplexityCounts::from_parts(branch_count, cognitive, early_return_count)
}

fn parse_function_header(trimmed: &str) -> Option<String> {
    let rest = trimmed
        .strip_prefix("def ")
        .or_else(|| trimmed.strip_prefix("async def "))?;
    let name = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    (!name.is_empty() && rest.contains('(') && rest.contains(')')).then_some(name)
}

fn block_end(lines: &[&str], start_idx: usize, indent: usize) -> usize {
    let mut last_line = start_idx + 1;
    for (idx, line) in lines.iter().enumerate().skip(start_idx + 1) {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let current_indent = line.len().saturating_sub(trimmed.len());
        if current_indent <= indent {
            return last_line;
        }
        last_line = idx + 1;
    }
    last_line
}

fn first_body_indent(
    lines: &[&str],
    start_idx: usize,
    end_line: usize,
    base_indent: usize,
) -> usize {
    lines
        .iter()
        .take(end_line)
        .skip(start_idx)
        .find_map(|line| {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('@') {
                return None;
            }
            let indent = line.len().saturating_sub(trimmed.len());
            (indent > base_indent).then_some(indent)
        })
        .unwrap_or(base_indent)
}

fn python_branch_sites(trimmed: &str) -> i64 {
    if trimmed.starts_with("elif ")
        || trimmed.starts_with("if ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("except")
        || trimmed.starts_with("case ")
        || trimmed.starts_with("match ")
    {
        return 1;
    }
    0
}

fn starts_python_block(trimmed: &str) -> bool {
    trimmed.ends_with(':')
        && [
            "if ", "elif ", "else:", "for ", "while ", "try:", "except", "finally:", "with ",
            "match ", "case ",
        ]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}
