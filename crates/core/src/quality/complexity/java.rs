use super::common::{
    ComplexityCounts, count_ternary_operators, function_span_location, is_return_statement,
    observed, strip_line_comment, update_max,
};
use crate::model::QualitySource;
use crate::quality::HotspotFacts;

pub(super) fn analyze(source: &str) -> HotspotFacts {
    let lines = source.lines().collect::<Vec<_>>();
    let line_offsets = line_offsets(source);
    let mut facts = HotspotFacts::default();
    let mut idx = 0_usize;

    while idx < lines.len() {
        if let Some(method) = parse_java_method(&lines, &line_offsets, idx) {
            let location = function_span_location(&lines, method.start_line, method.end_line);
            let body_start = method.body_start_offset.saturating_add(1).min(source.len());
            let body_end = method.end_offset.saturating_sub(1).min(source.len());
            let counts = scan_java_complexity(&source[body_start..body_end]);
            record_counts(&mut facts, location, counts);
            idx = method.end_line;
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

fn scan_java_complexity(body: &str) -> ComplexityCounts {
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

        let sites = java_branch_sites(trimmed);
        if sites > 0 {
            branch_count += sites;
            cognitive += sites.saturating_mul(1 + nesting_depth);
        }

        let open_count = trimmed.matches('{').count();
        if open_count == 0 {
            continue;
        }
        if java_introduces_control_block(trimmed) {
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

fn java_branch_sites(trimmed: &str) -> i64 {
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
        || trimmed.starts_with("catch ")
        || trimmed.starts_with("switch ")
    {
        return 1;
    }
    0
}

fn java_introduces_control_block(trimmed: &str) -> bool {
    trimmed.contains('{')
        && (trimmed.starts_with("if ")
            || trimmed.starts_with("else if ")
            || trimmed.starts_with("else")
            || trimmed.starts_with("for ")
            || trimmed.starts_with("while ")
            || trimmed.starts_with("do ")
            || trimmed.starts_with("catch ")
            || trimmed.starts_with("switch ")
            || trimmed.starts_with("try "))
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

struct JavaMethod {
    start_line: usize,
    end_line: usize,
    body_start_offset: usize,
    end_offset: usize,
}

fn parse_java_method(
    lines: &[&str],
    line_offsets: &[usize],
    start_idx: usize,
) -> Option<JavaMethod> {
    let first = strip_line_comment(lines[start_idx], "//").trim_start();
    if first.starts_with('@') {
        return None;
    }
    let mut signature = String::from(first);
    let mut end_idx = start_idx;
    while !signature.contains('{') && !signature.contains(';') && end_idx + 1 < lines.len() {
        end_idx += 1;
        signature.push('\n');
        signature.push_str(strip_line_comment(lines[end_idx], "//"));
    }
    if signature.contains(';')
        || is_type_declaration(&signature)
        || is_control_signature(&signature)
    {
        return None;
    }
    let open_paren = signature.find('(')?;
    let name = method_name_before_paren(&signature[..open_paren])?;
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
                        let _start_offset =
                            line_offsets[start_idx] + lines[start_idx].find(&name)?;
                        return Some(JavaMethod {
                            start_line: start_idx + 1,
                            end_line: line_idx + 1,
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

fn method_name_before_paren(prefix: &str) -> Option<String> {
    let token = prefix
        .split_whitespace()
        .last()?
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_');
    let blocked = [
        "if",
        "for",
        "while",
        "switch",
        "catch",
        "new",
        "return",
        "throw",
        "class",
        "interface",
        "enum",
        "record",
    ];
    (!token.is_empty() && !blocked.contains(&token)).then(|| token.to_string())
}

fn is_type_declaration(signature: &str) -> bool {
    let trimmed = signature.trim_start();
    trimmed.starts_with("class ")
        || trimmed.starts_with("interface ")
        || trimmed.starts_with("enum ")
        || trimmed.starts_with("record ")
}

fn is_control_signature(signature: &str) -> bool {
    let trimmed = signature.trim_start();
    ["if ", "for ", "while ", "switch ", "catch ", "do "]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}
