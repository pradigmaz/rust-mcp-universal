use crate::text_utils;

pub(super) fn first_match_char_offset(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    let byte_idx = haystack.find(needle)?;
    Some(haystack[..byte_idx].chars().count())
}

pub(super) fn escape_like_value(value: &str) -> String {
    text_utils::escape_like_value(value)
}

pub(super) fn trim_preview(text: &str, max_chars: usize) -> String {
    text_utils::trim_compact_text(text, max_chars)
}
