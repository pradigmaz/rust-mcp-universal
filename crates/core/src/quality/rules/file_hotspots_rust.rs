use super::file_hotspots_common::{
    LineIndex, count_top_level_parameters, observed, scan_braced_control_nesting,
    strip_line_comment, update_max,
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
            update_max(
                &mut facts.max_function_lines,
                observed(
                    i64::try_from(function.end_line.saturating_sub(function.start_line) + 1)
                        .unwrap_or(i64::MAX),
                    location.clone(),
                    QualitySource::ParserLight,
                ),
            );
            update_max(
                &mut facts.max_parameters_per_function,
                observed(
                    count_top_level_parameters(&function.params),
                    location.clone(),
                    QualitySource::ParserLight,
                ),
            );
            update_max(
                &mut facts.max_nesting_depth,
                observed(
                    scan_braced_control_nesting(
                        &source[function.body_start_offset.min(source.len())
                            ..function.end_offset.min(source.len())],
                        &["if", "for", "while", "match", "loop"],
                    ),
                    location,
                    QualitySource::ParserLight,
                ),
            );
            idx = function.end_line;
            continue;
        }
        idx += 1;
    }

    facts.max_export_count_per_file = Some(observed(
        count_rust_exports(&lines),
        None,
        QualitySource::ParserLight,
    ));
    facts
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
    start_line: usize,
    end_line: usize,
    start_offset: usize,
    body_start_offset: usize,
    end_offset: usize,
    params: String,
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
    let params = extract_params(&signature)?;
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
                            start_line: start_idx + 1,
                            end_line: line_idx + 1,
                            start_offset: line_offsets[start_idx]
                                + lines[start_idx].find(name).unwrap_or(0),
                            body_start_offset,
                            end_offset: line_offsets[line_idx] + byte_idx + 1,
                            params,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn extract_params(signature: &str) -> Option<String> {
    let open = signature.find('(')?;
    let close = signature[open + 1..].find(')')? + open + 1;
    Some(signature[open + 1..close].to_string())
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

fn count_rust_exports(lines: &[&str]) -> i64 {
    i64::try_from(
        lines
            .iter()
            .filter(|line| {
                let trimmed = strip_line_comment(line, "//").trim_start();
                trimmed.starts_with("pub ")
                    || trimmed.starts_with("pub(crate) ")
                    || trimmed.starts_with("pub(super) ")
                    || trimmed.starts_with("pub(in ")
            })
            .count(),
    )
    .unwrap_or(i64::MAX)
}
