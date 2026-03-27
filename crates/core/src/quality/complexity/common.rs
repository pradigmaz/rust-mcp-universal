use crate::model::{QualityLocation, QualitySource};
use crate::quality::ObservedMetric;

#[derive(Debug, Clone)]
pub(super) struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    pub(super) fn new(source: &str) -> Self {
        let mut starts = vec![0];
        for (idx, ch) in source.char_indices() {
            if ch == '\n' {
                starts.push(idx + 1);
            }
        }
        Self { starts }
    }

    pub(super) fn locate(&self, source: &str, offset: usize) -> (usize, usize) {
        let offset = offset.min(source.len());
        let line_idx = self
            .starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        let line_start = self.starts.get(line_idx).copied().unwrap_or(0);
        let column = source[line_start..offset].chars().count() + 1;
        (line_idx + 1, column)
    }

    pub(super) fn span_location(
        &self,
        source: &str,
        start: usize,
        end: usize,
    ) -> Option<QualityLocation> {
        if source.is_empty() {
            return None;
        }
        let end = end.max(start.saturating_add(1)).min(source.len());
        let (start_line, start_column) = self.locate(source, start);
        let (end_line, end_column) = self.locate(source, end.saturating_sub(1));
        Some(QualityLocation {
            start_line,
            start_column,
            end_line,
            end_column: end_column.max(1),
        })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ComplexityCounts {
    pub(super) cyclomatic: i64,
    pub(super) cognitive: i64,
    pub(super) branch_count: i64,
    pub(super) early_return_count: i64,
}

impl ComplexityCounts {
    pub(super) fn from_parts(branch_count: i64, cognitive: i64, early_return_count: i64) -> Self {
        Self {
            cyclomatic: branch_count.saturating_add(1),
            cognitive,
            branch_count,
            early_return_count,
        }
    }
}

pub(super) fn observed(
    metric_value: i64,
    location: Option<QualityLocation>,
    source: QualitySource,
) -> ObservedMetric {
    ObservedMetric {
        metric_value,
        location,
        source,
    }
}

pub(super) fn update_max(slot: &mut Option<ObservedMetric>, candidate: ObservedMetric) {
    match slot {
        Some(current) if current.metric_value >= candidate.metric_value => {}
        _ => *slot = Some(candidate),
    }
}

pub(super) fn strip_line_comment<'a>(line: &'a str, marker: &str) -> &'a str {
    line.split(marker).next().unwrap_or(line)
}

pub(super) fn function_span_location(
    lines: &[&str],
    start_line: usize,
    end_line: usize,
) -> Option<QualityLocation> {
    if start_line == 0 || end_line == 0 || start_line > end_line || end_line > lines.len() {
        return None;
    }
    let start_len = lines[start_line - 1].chars().count();
    let end_len = lines[end_line - 1].chars().count();
    Some(crate::quality::location::region_location(
        start_line, start_len, end_line, end_len,
    ))
}

pub(super) fn is_return_statement(trimmed: &str) -> bool {
    trimmed == "return"
        || trimmed.starts_with("return ")
        || trimmed.starts_with("return;")
        || trimmed.starts_with("return(")
}

pub(super) fn count_ternary_operators(source: &str) -> i64 {
    let mut count = 0_i64;
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '?' {
            continue;
        }
        if matches!(chars.peek(), Some('.') | Some('?')) {
            continue;
        }
        count += 1;
    }
    count
}
