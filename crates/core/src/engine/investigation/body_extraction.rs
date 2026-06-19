use crate::model::SourceSpan;

use super::common::CandidateFile;

pub(super) fn extract_rust_block(
    candidate: &CandidateFile,
    lines: &[String],
) -> Option<(String, String, SourceSpan, bool)> {
    let anchor = candidate
        .line
        .unwrap_or(1)
        .saturating_sub(1)
        .min(lines.len() - 1);
    let start = (0..=anchor).rev().find(|index| {
        let line = lines[*index].trim_start();
        looks_like_rust_declaration(line)
    })?;
    let mut brace_balance = 0_i32;
    let mut seen_open = false;
    let mut end = start;
    for (index, line) in lines.iter().enumerate().skip(start) {
        brace_balance += line.matches('{').count() as i32;
        if line.contains('{') {
            seen_open = true;
        }
        brace_balance -= line.matches('}').count() as i32;
        end = index;
        if seen_open && brace_balance <= 0 {
            break;
        }
        if !seen_open && index >= start + 12 {
            break;
        }
    }
    build_body(lines, start, end)
}

pub(super) fn extract_python_block(
    candidate: &CandidateFile,
    lines: &[String],
) -> Option<(String, String, SourceSpan, bool)> {
    let anchor = candidate
        .line
        .unwrap_or(1)
        .saturating_sub(1)
        .min(lines.len() - 1);
    let mut start = (0..=anchor).rev().find(|index| {
        let line = lines[*index].trim_start();
        line.starts_with("def ") || line.starts_with("async def ") || line.starts_with("class ")
    })?;
    while start > 0 && lines[start - 1].trim_start().starts_with('@') {
        start -= 1;
    }
    let header_end = python_header_end(lines, start)?;
    let base_indent = indentation(lines[start].as_str());
    let mut end = lines.len() - 1;
    for (index, raw) in lines.iter().enumerate().skip(header_end + 1) {
        let raw = raw.as_str();
        if raw.trim().is_empty() {
            continue;
        }
        if indentation(raw) <= base_indent && !raw.trim_start().starts_with('#') {
            end = index.saturating_sub(1);
            break;
        }
    }
    build_body(lines, start, end)
}

fn python_header_end(lines: &[String], start: usize) -> Option<usize> {
    let mut paren_balance = 0_i32;
    for (index, raw) in lines.iter().enumerate().skip(start).take(16) {
        let line = raw.as_str();
        paren_balance += line.matches('(').count() as i32;
        paren_balance -= line.matches(')').count() as i32;
        if paren_balance <= 0 && line.trim_end().ends_with(':') {
            return Some(index);
        }
    }
    None
}

pub(super) fn extract_js_ts_block(
    candidate: &CandidateFile,
    lines: &[String],
) -> Option<(String, String, SourceSpan, bool)> {
    let anchor = candidate
        .line
        .unwrap_or(1)
        .saturating_sub(1)
        .min(lines.len() - 1);
    let start = (0..=anchor)
        .rev()
        .find(|index| looks_like_js_ts_declaration(lines[*index].trim_start()))?;
    let mut brace_balance = 0_i32;
    let mut seen_open = false;
    let mut end = start;
    for (index, line) in lines.iter().enumerate().skip(start) {
        brace_balance += line.matches('{').count() as i32;
        if line.contains('{') {
            seen_open = true;
        }
        brace_balance -= line.matches('}').count() as i32;
        end = index;
        if seen_open && brace_balance <= 0 {
            break;
        }
        if !seen_open && index >= start + 12 {
            break;
        }
    }
    build_body(lines, start, end)
}

pub(super) fn build_body(
    lines: &[String],
    start: usize,
    end: usize,
) -> Option<(String, String, SourceSpan, bool)> {
    if start > end || end >= lines.len() {
        return None;
    }
    let excerpt = lines[start..=end].join("\n");
    excerpt_to_body(&excerpt, start + 1, end + 1)
}

pub(super) fn excerpt_to_body(
    excerpt: &str,
    start_line: usize,
    end_line: usize,
) -> Option<(String, String, SourceSpan, bool)> {
    let trimmed_excerpt = excerpt.trim_end();
    if trimmed_excerpt.is_empty() {
        return None;
    }
    let truncated = trimmed_excerpt.len() > 2000;
    let body = if truncated {
        trimmed_excerpt.chars().take(2000).collect::<String>()
    } else {
        trimmed_excerpt.to_string()
    };
    let signature = body
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .unwrap_or_default();
    Some((
        signature,
        body,
        SourceSpan {
            start_line,
            end_line: end_line.max(start_line),
            start_column: Some(1),
            end_column: None,
        },
        truncated,
    ))
}

fn indentation(line: &str) -> usize {
    line.chars().take_while(|ch| ch.is_whitespace()).count()
}

fn looks_like_js_ts_declaration(line: &str) -> bool {
    if line.starts_with("function ")
        || line.starts_with("export function ")
        || line.starts_with("export default function ")
        || line.starts_with("async function ")
        || line.starts_with("export async function ")
        || line.starts_with("class ")
        || line.starts_with("export class ")
        || line.starts_with("export default class ")
        || line.starts_with("const ")
        || line.starts_with("let ")
        || line.starts_with("var ")
    {
        return line.contains("=>") || line.contains('{') || line.starts_with("class ");
    }

    if line.contains('(')
        && line.contains(')')
        && line.contains('{')
        && !line.starts_with("if ")
        && !line.starts_with("for ")
        && !line.starts_with("while ")
        && !line.starts_with("switch ")
        && !line.starts_with("catch ")
    {
        return true;
    }

    false
}

fn looks_like_rust_declaration(line: &str) -> bool {
    let line = line.strip_prefix("pub ").unwrap_or(line);
    let line = line.strip_prefix("pub(crate) ").unwrap_or(line);
    let line = line.strip_prefix("pub(super) ").unwrap_or(line);
    let line = if let Some(rest) = line.strip_prefix("pub(") {
        rest.split_once(") ").map_or(line, |(_, rest)| rest)
    } else {
        line
    };
    line.starts_with("fn ")
        || line.starts_with("async fn ")
        || line.starts_with("struct ")
        || line.starts_with("enum ")
        || line.starts_with("impl ")
        || line.starts_with("trait ")
}
