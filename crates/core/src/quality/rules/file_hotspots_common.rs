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

pub(super) fn update_max(slot: &mut Option<ObservedMetric>, candidate: ObservedMetric) {
    match slot {
        Some(current) if current.metric_value >= candidate.metric_value => {}
        _ => *slot = Some(candidate),
    }
}

pub(super) fn observed(
    value: i64,
    location: Option<QualityLocation>,
    source: QualitySource,
) -> ObservedMetric {
    ObservedMetric {
        metric_value: value,
        location,
        source,
    }
}

pub(super) fn count_todo_markers(source: &str) -> i64 {
    i64::try_from(
        source
            .lines()
            .filter(|line| {
                let upper = line.to_ascii_uppercase();
                upper.contains("TODO") || upper.contains("FIXME") || upper.contains("XXX")
            })
            .count(),
    )
    .unwrap_or(i64::MAX)
}

pub(super) fn todo_metric(source: &str) -> Option<ObservedMetric> {
    Some(observed(
        count_todo_markers(source),
        None,
        QualitySource::Heuristic,
    ))
}

pub(super) fn count_top_level_parameters(signature: &str) -> i64 {
    let mut depth = 0_i32;
    let mut count = 0_i64;
    let mut has_token = false;
    for ch in signature.chars() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                if has_token {
                    count += 1;
                    has_token = false;
                }
            }
            ch if !ch.is_whitespace() => has_token = true,
            _ => {}
        }
    }
    if has_token {
        count += 1;
    }
    count
}

pub(super) fn strip_line_comment<'a>(line: &'a str, marker: &str) -> &'a str {
    line.split(marker).next().unwrap_or(line)
}

pub(super) fn scan_braced_control_nesting(source: &str, control_keywords: &[&str]) -> i64 {
    let mut stack = Vec::<bool>::new();
    let mut depth = 0_i64;
    let mut max_depth = 0_i64;

    for raw_line in source.lines() {
        let line = strip_line_comment(raw_line, "//");
        let trimmed = line.trim();
        let close_count = trimmed.matches('}').count();
        for _ in 0..close_count {
            if stack.pop().unwrap_or(false) {
                depth = depth.saturating_sub(1);
            }
        }

        let open_count = trimmed.matches('{').count();
        if open_count == 0 {
            continue;
        }

        let is_control = control_keywords.iter().any(|keyword| {
            trimmed.starts_with(keyword)
                || trimmed.contains(&format!(" {keyword} "))
                || trimmed.contains(&format!("({keyword} "))
        });
        if is_control {
            depth += 1;
            max_depth = max_depth.max(depth);
            stack.push(true);
            stack.extend(std::iter::repeat_n(false, open_count.saturating_sub(1)));
        } else {
            stack.extend(std::iter::repeat_n(false, open_count));
        }
    }

    max_depth
}
