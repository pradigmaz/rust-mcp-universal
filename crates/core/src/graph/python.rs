use std::collections::BTreeSet;

use super::{
    GraphExtraction, GraphRef, GraphSymbol,
    common::{
        column_from_byte_index, find_identifier_column, iter_call_candidates, read_identifier,
        strip_line_comment,
    },
};

pub(super) fn extract_python_heuristic(source: &str) -> GraphExtraction {
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

        if let Some(rest) = trimmed
            .strip_prefix("def ")
            .or_else(|| trimmed.strip_prefix("async def "))
            && let Some(name) = read_identifier(rest)
        {
            symbols.insert(GraphSymbol {
                column: find_identifier_column(line, &name),
                kind: "function".to_string(),
                line: Some(line_no),
                name,
            });
        }

        if let Some(rest) = trimmed.strip_prefix("class ")
            && let Some(name) = read_identifier(rest)
        {
            symbols.insert(GraphSymbol {
                column: find_identifier_column(line, &name),
                kind: "class".to_string(),
                line: Some(line_no),
                name,
            });
        }

        if let Some(rest) = trimmed.strip_prefix("import ") {
            for part in rest.split(',') {
                let name = part.split_whitespace().next().unwrap_or("");
                if !name.is_empty() {
                    deps.insert(name.to_string());
                }
            }
        }

        if let Some(rest) = trimmed.strip_prefix("from ") {
            let dep = rest.split_whitespace().next().unwrap_or("").trim();
            if !dep.is_empty() {
                deps.insert(dep.to_string());
            }
        }

        for graph_ref in extract_python_refs_from_line(line, line_no) {
            refs.insert(graph_ref);
        }
    }

    GraphExtraction {
        symbols: symbols.into_iter().collect(),
        deps: deps.into_iter().collect(),
        refs: refs.into_iter().collect(),
    }
}

fn extract_python_refs_from_line(line: &str, line_no: usize) -> Vec<GraphRef> {
    let mut out = Vec::new();
    for (candidate, start_idx) in iter_call_candidates(line) {
        if !candidate
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.')
        {
            continue;
        }
        let before_candidate = line[..start_idx].trim_end();
        let last_token = before_candidate.split_whitespace().last().unwrap_or("");
        if matches!(
            last_token,
            "def" | "class" | "if" | "for" | "while" | "return"
        ) {
            continue;
        }
        out.push(GraphRef {
            symbol: candidate,
            line: Some(line_no),
            column: Some(column_from_byte_index(line, start_idx)),
        });
    }
    out
}
