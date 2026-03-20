use std::collections::BTreeSet;

use crate::text_utils::symbol_tail;

use super::super::{
    GraphExtraction, GraphRef, GraphSymbol,
    common::{
        column_from_byte_index, extract_javascript_quoted_argument, find_identifier_column,
        iter_call_candidates, iter_path_candidates, read_identifier,
        read_javascript_string_literal, strip_javascript_item_modifiers, strip_line_comment,
    },
};

pub(super) fn extract_javascript_heuristic(
    source: &str,
    include_type_symbols: bool,
) -> GraphExtraction {
    let mut symbols = BTreeSet::new();
    let mut deps = BTreeSet::new();
    let mut refs = BTreeSet::new();

    for (line_idx, raw_line) in source.lines().enumerate() {
        let line_no = line_idx + 1;
        let line = strip_line_comment(raw_line);
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(symbol) = parse_javascript_symbol(line, trimmed, line_no, include_type_symbols)
        {
            symbols.insert(symbol);
        }

        if let Some(dep) = parse_javascript_dep(trimmed) {
            deps.insert(dep);
        }

        for graph_ref in extract_javascript_refs_from_line(line, line_no) {
            refs.insert(graph_ref);
        }
    }

    GraphExtraction {
        symbols: symbols.into_iter().collect(),
        deps: deps.into_iter().collect(),
        refs: refs.into_iter().collect(),
    }
}

fn parse_javascript_symbol(
    line: &str,
    trimmed: &str,
    line_no: usize,
    include_type_symbols: bool,
) -> Option<GraphSymbol> {
    let normalized = strip_javascript_item_modifiers(trimmed);
    for (keyword, kind) in [("class ", "class"), ("function ", "function")] {
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

    if include_type_symbols {
        for (keyword, kind) in [
            ("interface ", "interface"),
            ("type ", "type"),
            ("enum ", "enum"),
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
    }

    None
}

fn parse_javascript_dep(trimmed: &str) -> Option<String> {
    if let Some(rest) = trimmed.strip_prefix("import ") {
        let rest = rest.trim_start();
        if let Some(dep) = read_javascript_string_literal(rest) {
            return Some(dep);
        }
    }

    if let Some((_, rest)) = trimmed.split_once(" from ")
        && let Some(dep) = read_javascript_string_literal(rest)
    {
        return Some(dep);
    }

    if let Some(dep) = extract_javascript_quoted_argument(trimmed, "require(") {
        return Some(dep);
    }

    extract_javascript_quoted_argument(trimmed, "import(")
}

fn extract_javascript_refs_from_line(line: &str, line_no: usize) -> Vec<GraphRef> {
    let mut out = BTreeSet::new();

    for (candidate, start_idx) in iter_call_candidates(line) {
        let before_candidate = line[..start_idx].trim_end();
        if is_javascript_ref_blocked(before_candidate, &candidate) {
            continue;
        }
        out.insert(GraphRef {
            symbol: candidate,
            line: Some(line_no),
            column: Some(column_from_byte_index(line, start_idx)),
        });
    }

    for (candidate, start_idx, end_idx) in iter_path_candidates(line) {
        let before_candidate = line[..start_idx].trim_end();
        let after_candidate = &line[end_idx..];
        if !is_javascript_type_ref_candidate(before_candidate, &candidate, after_candidate) {
            continue;
        }
        out.insert(GraphRef {
            symbol: candidate,
            line: Some(line_no),
            column: Some(column_from_byte_index(line, start_idx)),
        });
    }

    out.into_iter().collect()
}

fn is_javascript_ref_blocked(before_candidate: &str, candidate: &str) -> bool {
    let tail = symbol_tail(candidate);
    if tail.is_empty() {
        return true;
    }
    let Some(first) = tail.chars().next() else {
        return true;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return true;
    }

    if matches!(
        candidate,
        "if" | "for"
            | "while"
            | "switch"
            | "catch"
            | "return"
            | "throw"
            | "function"
            | "class"
            | "interface"
            | "type"
            | "enum"
            | "import"
            | "export"
            | "from"
            | "default"
            | "constructor"
    ) {
        return true;
    }

    matches!(
        before_candidate.split_whitespace().last().unwrap_or(""),
        "function"
            | "class"
            | "interface"
            | "type"
            | "enum"
            | "if"
            | "for"
            | "while"
            | "switch"
            | "catch"
            | "return"
            | "throw"
            | "import"
            | "export"
            | "from"
            | "public"
            | "private"
            | "protected"
            | "static"
            | "async"
            | "get"
            | "set"
    )
}

fn is_javascript_type_ref_candidate(
    before_candidate: &str,
    candidate: &str,
    after_candidate: &str,
) -> bool {
    let tail = symbol_tail(candidate);
    let Some(first) = tail.chars().next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }

    let last_token = before_candidate.split_whitespace().last().unwrap_or("");
    if matches!(
        last_token,
        "class" | "function" | "interface" | "type" | "enum"
    ) {
        return false;
    }

    let Some(next_char) = after_candidate.trim_start().chars().next() else {
        return false;
    };
    matches!(
        next_char,
        '{' | ',' | ')' | ']' | '}' | ';' | '>' | '=' | ':' | '|' | '&' | '?'
    )
}
