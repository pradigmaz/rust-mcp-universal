use super::common::{
    ComplexityCounts, LineIndex, is_return_statement, observed, strip_line_comment, update_max,
};
use crate::model::QualitySource;
use crate::quality::HotspotFacts;

pub(super) fn analyze(source: &str) -> HotspotFacts {
    let lines = source.lines().collect::<Vec<_>>();
    let mut facts = HotspotFacts::default();
    let line_index = LineIndex::new(source);
    let line_offsets = line_offsets(source);
    let mut idx = 0_usize;

    while idx < lines.len() {
        if let Some(function) = parse_rust_function(&lines, &line_offsets, idx) {
            let location =
                line_index.span_location(source, function.start_offset, function.end_offset);
            let body_start = function
                .body_start_offset
                .saturating_add(1)
                .min(source.len());
            let body_end = function.end_offset.saturating_sub(1).min(source.len());
            let counts = scan_rust_complexity(&source[body_start..body_end]);
            record_counts(&mut facts, location, counts);
            idx = function.end_line;
            continue;
        }
        idx += 1;
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

fn scan_rust_complexity(body: &str) -> ComplexityCounts {
    let mut branch_count = 0_i64;
    let mut cognitive = 0_i64;
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

        let sites = rust_branch_sites(trimmed);
        if sites > 0 {
            branch_count += sites;
            cognitive += sites.saturating_mul(1 + nesting_depth);
        }

        let open_count = trimmed.matches('{').count();
        if open_count == 0 {
            continue;
        }
        if rust_introduces_control_block(trimmed) {
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

fn rust_branch_sites(trimmed: &str) -> i64 {
    if trimmed.starts_with("else if ") {
        return 1;
    }
    if trimmed.starts_with("if ")
        || trimmed.starts_with("if let ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("loop")
    {
        return 1;
    }
    let arm_count = i64::try_from(trimmed.matches("=>").count()).unwrap_or(i64::MAX);
    if arm_count > 0 {
        return arm_count;
    }
    if trimmed.starts_with("match ") {
        return 1;
    }
    0
}

fn rust_introduces_control_block(trimmed: &str) -> bool {
    trimmed.contains('{')
        && (trimmed.starts_with("if ")
            || trimmed.starts_with("if let ")
            || trimmed.starts_with("else if ")
            || trimmed.starts_with("else")
            || trimmed.starts_with("for ")
            || trimmed.starts_with("while ")
            || trimmed.starts_with("loop")
            || trimmed.starts_with("match ")
            || trimmed.contains("=> {"))
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

fn line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut next = 0_usize;
    for line in source.split_inclusive('\n') {
        offsets.push(next);
        next += line.len();
    }
    if !source.is_empty() && !source.ends_with('\n') {
        offsets.push(next);
    }
    offsets
}

struct RustFunction {
    end_line: usize,
    start_offset: usize,
    body_start_offset: usize,
    end_offset: usize,
}

fn parse_rust_function(
    lines: &[&str],
    line_offsets: &[usize],
    start_idx: usize,
) -> Option<RustFunction> {
    let first = strip_line_comment(lines[start_idx], "//").trim_start();
    let normalized = strip_rust_modifiers(first);
    let rest = normalized.strip_prefix("fn ")?;
    let name_end = rest.find('(')?;
    let name = &rest[..name_end];
    if name.trim().is_empty() {
        return None;
    }

    let mut signature = String::from(normalized);
    let mut end_idx = start_idx;
    while !signature.contains('{') && !signature.contains(';') && end_idx + 1 < lines.len() {
        end_idx += 1;
        signature.push('\n');
        signature.push_str(strip_line_comment(lines[end_idx], "//"));
    }
    let body_open = signature.find('{')?;
    let body_start_offset = line_offsets[start_idx] + body_open;

    let mut depth = 0_i64;
    let mut seen_open = false;
    for line_idx in start_idx..lines.len() {
        let line = strip_line_comment(lines[line_idx], "//");
        for (byte_idx, ch) in line.char_indices() {
            match ch {
                '{' => {
                    depth += 1;
                    seen_open = true;
                }
                '}' => {
                    depth -= 1;
                    if seen_open && depth == 0 {
                        return Some(RustFunction {
                            end_line: line_idx + 1,
                            start_offset: line_offsets[start_idx] + lines[start_idx].find(name)?,
                            body_start_offset,
                            end_offset: line_offsets[line_idx] + byte_idx + 1,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn strip_rust_modifiers(mut text: &str) -> &str {
    loop {
        let trimmed = text.trim_start();
        if let Some(rest) = trimmed.strip_prefix("pub(") {
            let Some(close_idx) = rest.find(')') else {
                return trimmed;
            };
            text = &rest[close_idx + 1..];
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("pub ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("async ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("const ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("unsafe ") {
            text = rest;
            continue;
        }
        return trimmed;
    }
}
