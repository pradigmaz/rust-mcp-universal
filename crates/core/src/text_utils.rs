pub(crate) fn escape_like_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '%' | '_' | '\\' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub(crate) fn symbol_tail(symbol: &str) -> &str {
    symbol
        .rsplit_once("::")
        .map(|(_, tail)| tail)
        .or_else(|| symbol.rsplit_once('.').map(|(_, tail)| tail))
        .unwrap_or(symbol)
}

pub(crate) fn trim_compact_text(text: &str, max_chars: usize) -> String {
    let compact = text.replace(['\n', '\r', '\t'], " ");
    let trimmed = compact.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let mut out = String::with_capacity(max_chars + 3);
        for ch in trimmed.chars().take(max_chars) {
            out.push(ch);
        }
        out.push_str("...");
        out
    }
}

pub(crate) fn is_low_priority_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    normalized.starts_with("docs/archive/")
        || normalized.contains("/docs/archive/")
        || normalized.starts_with("archive/")
        || normalized.contains("/archive/")
        || normalized.starts_with("baseline/")
        || normalized.contains("/baseline/")
        || normalized.starts_with("fixtures/")
        || normalized.contains("/fixtures/")
        || normalized.starts_with("fixture/")
        || normalized.contains("/fixture/")
        || normalized.starts_with("testdata/")
        || normalized.contains("/testdata/")
        || normalized.starts_with("snapshots/")
        || normalized.contains("/snapshots/")
}

pub(crate) fn query_allows_low_priority_paths(query: &str) -> bool {
    let lowered = query.to_ascii_lowercase();
    [
        "archive", "archived", "baseline", "fixture", "fixtures", "testdata", "snapshot",
    ]
    .iter()
    .any(|token| lowered.contains(token))
}

pub(crate) fn i64_to_option_usize(value: i64) -> Option<usize> {
    usize::try_from(value).ok()
}

pub(crate) fn i64_to_usize_non_negative(value: i64) -> usize {
    if value <= 0 {
        0
    } else {
        usize::try_from(value).unwrap_or(usize::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        escape_like_value, i64_to_option_usize, i64_to_usize_non_negative, is_low_priority_path,
        query_allows_low_priority_paths, symbol_tail, trim_compact_text,
    };

    #[test]
    fn symbol_tail_supports_rust_and_member_paths() {
        assert_eq!(symbol_tail("crate::engine::Graph"), "Graph");
        assert_eq!(symbol_tail("value.method"), "method");
        assert_eq!(symbol_tail("PlainName"), "PlainName");
    }

    #[test]
    fn escape_like_value_escapes_sqlite_like_meta_characters() {
        assert_eq!(escape_like_value(r"100%_done\ok"), r"100\%\_done\\ok");
    }

    #[test]
    fn integer_helpers_keep_non_negative_contracts() {
        assert_eq!(i64_to_option_usize(-1), None);
        assert_eq!(i64_to_option_usize(7), Some(7));
        assert_eq!(i64_to_usize_non_negative(-1), 0);
        assert_eq!(i64_to_usize_non_negative(7), 7);
    }

    #[test]
    fn trim_compact_text_normalizes_whitespace_before_truncation() {
        assert_eq!(trim_compact_text("  a\tb\nc  ", 16), "a b c");
        assert_eq!(trim_compact_text("alpha beta gamma", 5), "alpha...");
    }

    #[test]
    fn low_priority_path_helpers_detect_archival_material() {
        assert!(is_low_priority_path("docs/archive/reference/lib.rs"));
        assert!(is_low_priority_path(
            "baseline/investigation/fixtures/app.rs"
        ));
        assert!(!is_low_priority_path("crates/core/src/lib.rs"));
        assert!(query_allows_low_priority_paths("inspect archive fixture"));
        assert!(!query_allows_low_priority_paths("inspect usage stats"));
    }
}
