use std::collections::BTreeSet;

use crate::text_utils::symbol_tail;

use super::{
    GraphExtraction, GraphRef, GraphSymbol,
    common::{
        column_from_byte_index, find_identifier_column, iter_call_candidates, iter_path_candidates,
        read_identifier,
    },
    java_support::{
        normalize_java_type_ref, read_trailing_identifier, starts_with_uppercase,
        strip_java_comments, strip_java_item_modifiers,
    },
};

pub(super) fn extract_java_heuristic(source: &str) -> GraphExtraction {
    let mut symbols = BTreeSet::new();
    let mut deps = BTreeSet::new();
    let mut refs = BTreeSet::new();
    let mut in_block_comment = false;

    for (line_idx, raw_line) in source.lines().enumerate() {
        let line_no = line_idx + 1;
        let line = strip_java_comments(raw_line, &mut in_block_comment);
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }

        let declared_symbol = parse_java_symbol(&line, trimmed, line_no);
        if let Some(symbol) = declared_symbol.clone() {
            symbols.insert(symbol);
        }

        if let Some(dep) = parse_java_dep(trimmed) {
            if let Some(graph_ref) = graph_ref_from_import(&line, &dep, line_no) {
                refs.insert(graph_ref);
            }
            deps.insert(dep);
        }

        for graph_ref in
            extract_java_refs_from_line(&line, trimmed, line_no, declared_symbol.as_ref())
        {
            refs.insert(graph_ref);
        }
    }

    GraphExtraction {
        symbols: symbols.into_iter().collect(),
        deps: deps.into_iter().collect(),
        refs: refs.into_iter().collect(),
    }
}

fn parse_java_symbol(line: &str, trimmed: &str, line_no: usize) -> Option<GraphSymbol> {
    let normalized = strip_java_item_modifiers(trimmed);
    for (keyword, kind) in [
        ("@interface ", "annotation"),
        ("class ", "class"),
        ("interface ", "interface"),
        ("enum ", "enum"),
        ("record ", "record"),
    ] {
        if let Some(rest) = normalized.strip_prefix(keyword)
            && let Some(name) = read_identifier(rest)
        {
            return Some(GraphSymbol {
                column: find_identifier_column(line, &name),
                kind: kind.to_string(),
                line: Some(line_no),
                name,
            });
        }
    }

    let (name, kind) = parse_java_callable_symbol(normalized)?;
    Some(GraphSymbol {
        column: find_identifier_column(line, &name),
        kind: kind.to_string(),
        line: Some(line_no),
        name,
    })
}

fn parse_java_callable_symbol(trimmed: &str) -> Option<(String, &'static str)> {
    if trimmed.starts_with("package ")
        || trimmed.starts_with("import ")
        || matches!(
            trimmed.split_whitespace().next().unwrap_or(""),
            "if" | "for" | "while" | "switch" | "catch" | "return" | "throw" | "new" | "assert"
        )
    {
        return None;
    }

    let paren_idx = trimmed.find('(')?;
    let prefix = trimmed[..paren_idx].trim_end();
    if prefix.is_empty() || prefix.contains('=') || prefix.ends_with('.') || prefix.ends_with("->")
    {
        return None;
    }

    let (start_idx, name) = read_trailing_identifier(prefix)?;
    let before_name = prefix[..start_idx].trim_end();
    if before_name.ends_with('.') || before_name.ends_with("::") {
        return None;
    }

    if matches!(
        before_name.split_whitespace().last().unwrap_or(""),
        "class"
            | "interface"
            | "enum"
            | "record"
            | "@interface"
            | "if"
            | "for"
            | "while"
            | "switch"
            | "catch"
            | "return"
            | "throw"
            | "new"
    ) {
        return None;
    }

    if before_name.is_empty() && starts_with_uppercase(&name) {
        Some((name, "constructor"))
    } else if !before_name.is_empty() {
        Some((name, "method"))
    } else {
        None
    }
}

fn parse_java_dep(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("import ")?;
    let rest = rest.strip_prefix("static ").unwrap_or(rest);
    let dep = rest.split(';').next().unwrap_or("").trim();
    if dep.is_empty() {
        None
    } else {
        Some(dep.to_string())
    }
}

fn graph_ref_from_import(line: &str, dep: &str, line_no: usize) -> Option<GraphRef> {
    let tail = dep.rsplit('.').next().unwrap_or(dep).trim();
    if tail.is_empty() || tail == "*" {
        return None;
    }
    Some(GraphRef {
        symbol: normalize_java_type_ref(dep),
        line: Some(line_no),
        column: find_identifier_column(line, tail),
    })
}

fn extract_java_refs_from_line(
    line: &str,
    trimmed: &str,
    line_no: usize,
    declared_symbol: Option<&GraphSymbol>,
) -> Vec<GraphRef> {
    let mut out = BTreeSet::new();
    for (candidate, start_idx) in iter_call_candidates(line) {
        if !is_java_call_ref_candidate(line, &candidate, start_idx, declared_symbol) {
            continue;
        }
        out.insert(GraphRef {
            symbol: candidate,
            line: Some(line_no),
            column: Some(column_from_byte_index(line, start_idx)),
        });
    }

    if !trimmed.starts_with("package ") {
        for (candidate, start_idx, end_idx) in iter_path_candidates(line) {
            if !is_java_type_ref_candidate(line, &candidate, start_idx, end_idx) {
                continue;
            }
            out.insert(GraphRef {
                symbol: normalize_java_type_ref(&candidate),
                line: Some(line_no),
                column: Some(column_from_byte_index(line, start_idx)),
            });
        }
    }

    out.into_iter().collect()
}

fn is_java_call_ref_candidate(
    line: &str,
    candidate: &str,
    start_idx: usize,
    declared_symbol: Option<&GraphSymbol>,
) -> bool {
    if candidate.contains("::") {
        return false;
    }

    let tail = symbol_tail(candidate);
    let Some(first) = tail.chars().next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }

    if matches!(
        tail,
        "if" | "for"
            | "while"
            | "switch"
            | "catch"
            | "return"
            | "throw"
            | "new"
            | "assert"
            | "super"
            | "this"
            | "class"
            | "interface"
            | "enum"
            | "record"
    ) {
        return false;
    }

    if declared_symbol.is_some_and(|symbol| {
        matches!(symbol.kind.as_str(), "method" | "constructor") && symbol.name == tail
    }) && !candidate.contains('.')
    {
        return false;
    }

    !matches!(
        line[..start_idx].split_whitespace().last().unwrap_or(""),
        "if" | "for" | "while" | "switch" | "catch" | "return" | "throw" | "new" | "assert"
    )
}

fn is_java_type_ref_candidate(
    line: &str,
    candidate: &str,
    start_idx: usize,
    end_idx: usize,
) -> bool {
    if candidate.contains("::") {
        return false;
    }

    let normalized = normalize_java_type_ref(candidate);
    let tail = symbol_tail(&normalized);
    let Some(first) = tail.chars().next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }

    if matches!(
        line[..start_idx].split_whitespace().last().unwrap_or(""),
        "package" | "class" | "interface" | "enum" | "record" | "@interface"
    ) {
        return false;
    }

    !line[end_idx..].trim_start().starts_with("->")
}
