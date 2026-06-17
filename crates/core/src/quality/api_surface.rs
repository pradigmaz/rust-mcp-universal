use super::ApiSurfaceFacts;
use super::location::line_location;

pub(crate) fn analyze_api_surface(
    _rel_path: &str,
    language: &str,
    full_text: &str,
) -> ApiSurfaceFacts {
    match language {
        "rust" => analyze_rust(full_text),
        "javascript" | "typescript" | "jsx" | "tsx" => analyze_javascript(full_text),
        "python" => analyze_python(full_text),
        _ => ApiSurfaceFacts::default(),
    }
}

fn analyze_rust(full_text: &str) -> ApiSurfaceFacts {
    let mut facts = ApiSurfaceFacts::default();
    for (index, line) in full_text.lines().enumerate() {
        let trimmed = strip_line_comment(line, "//").trim_start();
        if trimmed.starts_with("pub(crate) ")
            || trimmed.starts_with("pub(super) ")
            || trimmed.starts_with("pub(in ")
        {
            facts.restricted_export_count += 1;
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("pub ") else {
            continue;
        };
        facts.public_export_count += 1;
        facts
            .primary_location
            .get_or_insert_with(|| line_location(index + 1, line.len()));
        let rest = rest.trim_start();
        if rest.starts_with("use ") {
            facts.public_reexport_count += 1;
        } else if starts_with_any(rest, &["struct ", "enum ", "trait ", "type ", "mod "]) {
            facts.public_type_count += 1;
        } else if rest.starts_with("fn ")
            || rest.starts_with("async fn ")
            || rest.starts_with("unsafe fn ")
        {
            facts.public_function_count += 1;
        }
    }
    facts
}

fn analyze_javascript(full_text: &str) -> ApiSurfaceFacts {
    let mut facts = ApiSurfaceFacts::default();
    for (index, line) in full_text.lines().enumerate() {
        let trimmed = strip_line_comment(line, "//").trim_start();
        let Some(rest) = trimmed.strip_prefix("export") else {
            continue;
        };
        let rest = rest.trim_start();
        if rest.is_empty() {
            continue;
        }
        let count = javascript_export_count(rest);
        if count == 0 {
            continue;
        }
        facts.public_export_count += count;
        facts
            .primary_location
            .get_or_insert_with(|| line_location(index + 1, line.len()));
        if is_javascript_reexport(rest) {
            facts.public_reexport_count += count;
        }
        if starts_with_any(rest, &["class ", "interface ", "type ", "enum "]) {
            facts.public_type_count += 1;
        } else if starts_with_any(rest, &["function ", "async function ", "default function "]) {
            facts.public_function_count += 1;
        }
    }
    facts
}

fn analyze_python(full_text: &str) -> ApiSurfaceFacts {
    let mut facts = ApiSurfaceFacts::default();
    for (index, line) in full_text.lines().enumerate() {
        let trimmed = strip_line_comment(line, "#").trim_start();
        let Some(rest) = trimmed.strip_prefix("__all__") else {
            continue;
        };
        let Some((_, value)) = rest.split_once('=') else {
            continue;
        };
        let count = count_python_all_entries(value);
        if count == 0 {
            continue;
        }
        facts.public_export_count = count;
        facts.primary_location = Some(line_location(index + 1, line.len()));
        break;
    }
    facts
}

fn javascript_export_count(rest: &str) -> i64 {
    if rest.starts_with("* ") || rest.starts_with("*from") || rest.starts_with("* from") {
        return 1;
    }
    if let Some(body) = rest.strip_prefix('{') {
        return count_braced_names(body);
    }
    1
}

fn is_javascript_reexport(rest: &str) -> bool {
    rest.starts_with("* ")
        || rest.starts_with("*from")
        || rest.starts_with("* from")
        || rest.contains(" from ")
}

fn count_braced_names(body: &str) -> i64 {
    let segment = body.split('}').next().unwrap_or_default();
    i64::try_from(
        segment
            .split(',')
            .filter(|part| !part.trim().is_empty())
            .count(),
    )
    .unwrap_or(i64::MAX)
}

fn count_python_all_entries(value: &str) -> i64 {
    let trimmed = value.trim();
    let body = trimmed
        .strip_prefix('[')
        .and_then(|text| text.split(']').next())
        .or_else(|| {
            trimmed
                .strip_prefix('(')
                .and_then(|text| text.split(')').next())
        })
        .unwrap_or_default();
    i64::try_from(
        body.split(',')
            .filter(|part| {
                let name = part.trim().trim_matches('"').trim_matches('\'');
                !name.is_empty()
            })
            .count(),
    )
    .unwrap_or(i64::MAX)
}

fn starts_with_any(value: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| value.starts_with(prefix))
}

fn strip_line_comment<'a>(line: &'a str, marker: &str) -> &'a str {
    line.split_once(marker)
        .map(|(before, _)| before)
        .unwrap_or(line)
}
