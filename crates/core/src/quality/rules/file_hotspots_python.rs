use super::file_hotspots_common::{count_top_level_parameters, observed, update_max};
use crate::model::{QualityLocation, QualitySource};
use crate::quality::HotspotFacts;

pub(super) fn analyze(source: &str) -> HotspotFacts {
    let lines = source.lines().collect::<Vec<_>>();
    let mut facts = HotspotFacts::default();
    let mut classes = Vec::<(usize, usize, usize, String)>::new();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len().saturating_sub(trimmed.len());
        if let Some((name, params)) = parse_function_header(trimmed) {
            let end_line = block_end(&lines, idx, indent);
            let location = declaration_location(line, idx + 1, &name);
            update_max(
                &mut facts.max_function_lines,
                observed(
                    i64::try_from(end_line.saturating_sub(idx + 1) + 1).unwrap_or(i64::MAX),
                    location.clone(),
                    QualitySource::ParserLight,
                ),
            );
            update_max(
                &mut facts.max_parameters_per_function,
                observed(
                    count_top_level_parameters(&params),
                    location.clone(),
                    QualitySource::ParserLight,
                ),
            );
            update_max(
                &mut facts.max_nesting_depth,
                observed(
                    python_nesting_depth(&lines, idx + 1, end_line, indent),
                    location,
                    QualitySource::ParserLight,
                ),
            );
        } else if let Some(name) = parse_class_header(trimmed) {
            classes.push((idx, indent, block_end(&lines, idx, indent), name));
        }
    }

    for (idx, indent, end_line, name) in classes {
        let location = declaration_location(lines[idx], idx + 1, &name);
        update_max(
            &mut facts.max_class_member_count,
            observed(
                count_python_class_members(&lines, idx + 1, end_line, indent),
                location,
                QualitySource::ParserLight,
            ),
        );
    }

    if let Some(export_count) = python_export_count(&lines) {
        facts.max_export_count_per_file =
            Some(observed(export_count, None, QualitySource::ParserLight));
    }
    facts
}

fn parse_function_header(trimmed: &str) -> Option<(String, String)> {
    let rest = trimmed
        .strip_prefix("def ")
        .or_else(|| trimmed.strip_prefix("async def "))?;
    let name = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    let start = rest.find('(')?;
    let end = rest.rfind(')')?;
    Some((name, rest[start + 1..end].to_string()))
}

fn parse_class_header(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("class ")?;
    let name = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    (!name.is_empty()).then_some(name)
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

fn declaration_location(line: &str, line_no: usize, name: &str) -> Option<QualityLocation> {
    let start_column = line.find(name).map(|idx| line[..idx].chars().count() + 1)?;
    Some(QualityLocation {
        start_line: line_no,
        start_column,
        end_line: line_no,
        end_column: start_column + name.chars().count().saturating_sub(1),
    })
}

fn python_nesting_depth(
    lines: &[&str],
    start_line: usize,
    end_line: usize,
    base_indent: usize,
) -> i64 {
    let mut stack = Vec::<usize>::new();
    let mut max_depth = 0_i64;
    for line in lines.iter().take(end_line).skip(start_line) {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len().saturating_sub(trimmed.len());
        while stack.last().is_some_and(|value| *value >= indent) {
            stack.pop();
        }
        if indent <= base_indent {
            continue;
        }
        if starts_python_control_block(trimmed) {
            stack.push(indent);
            max_depth = max_depth.max(i64::try_from(stack.len()).unwrap_or(i64::MAX));
        }
    }
    max_depth
}

fn count_python_class_members(
    lines: &[&str],
    start_line: usize,
    end_line: usize,
    class_indent: usize,
) -> i64 {
    let mut member_indent = None::<usize>;
    let mut count = 0_i64;
    for line in lines.iter().take(end_line).skip(start_line) {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('@') {
            continue;
        }
        let indent = line.len().saturating_sub(trimmed.len());
        if indent <= class_indent {
            continue;
        }
        let level = member_indent.get_or_insert(indent);
        if indent != *level {
            continue;
        }
        if trimmed.starts_with("def ")
            || trimmed.starts_with("async def ")
            || (trimmed.contains('=')
                && !trimmed.starts_with("if ")
                && !trimmed.starts_with("for "))
        {
            count += 1;
        }
    }
    count
}

fn python_export_count(lines: &[&str]) -> Option<i64> {
    for line in lines {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("__all__")
            && let Some(open_idx) = rest.find('[')
            && let Some(close_idx) = rest.rfind(']')
        {
            let inner = &rest[open_idx + 1..close_idx];
            return Some(
                i64::try_from(
                    inner
                        .split(',')
                        .filter(|item| {
                            let item = item.trim().trim_matches('"').trim_matches('\'');
                            !item.is_empty()
                        })
                        .count(),
                )
                .unwrap_or(i64::MAX),
            );
        }
    }
    None
}

fn starts_python_control_block(trimmed: &str) -> bool {
    [
        "if ", "for ", "while ", "try:", "with ", "match ", "elif ", "else:", "except", "finally:",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix))
}
