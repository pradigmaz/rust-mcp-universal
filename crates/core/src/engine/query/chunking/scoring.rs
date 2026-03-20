pub(super) fn chunk_lexical_signal(query_lc: &str, query_tokens: &[String], excerpt: &str) -> f32 {
    let excerpt_lc = excerpt.to_lowercase();
    let token_ratio = if query_tokens.is_empty() {
        0.0
    } else {
        let matched = query_tokens
            .iter()
            .filter(|token| excerpt_lc.contains(token.as_str()))
            .count();
        matched as f32 / query_tokens.len() as f32
    };
    let phrase_bonus = if !query_lc.is_empty() && excerpt_lc.contains(query_lc) {
        0.25
    } else {
        0.0
    };
    let starts_with_bonus = if !query_lc.is_empty() && excerpt_lc.starts_with(query_lc) {
        0.10
    } else {
        0.0
    };
    (token_ratio * 0.72 + phrase_bonus + starts_with_bonus).clamp(0.0, 1.0)
}

pub(super) fn compact_excerpt_for_budget(
    excerpt: &str,
    query_lc: &str,
    query_tokens: &[String],
    max_chars: usize,
) -> String {
    let compact = excerpt.replace(['\n', '\r', '\t'], " ");
    let trimmed = compact.trim();
    let chars = trimmed.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return trimmed.to_string();
    }

    let lowered = trimmed.to_lowercase();
    let anchor_byte = if query_lc.is_empty() {
        None
    } else {
        lowered.find(query_lc)
    }
    .or_else(|| {
        query_tokens
            .iter()
            .filter(|token| !token.is_empty())
            .find_map(|token| lowered.find(token.as_str()))
    });
    let anchor_char = anchor_byte
        .map(|byte_idx| lowered[..byte_idx].chars().count())
        .unwrap_or(0);

    let mut start = anchor_char.saturating_sub(max_chars / 3);
    if start + max_chars > chars.len() {
        start = chars.len().saturating_sub(max_chars);
    }
    let end = (start + max_chars).min(chars.len());

    let mut out = String::with_capacity(max_chars + 6);
    if start > 0 {
        out.push_str("...");
    }
    for ch in &chars[start..end] {
        out.push(*ch);
    }
    if end < chars.len() {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::compact_excerpt_for_budget;

    #[test]
    fn compact_excerpt_keeps_query_anchor_visible() {
        let excerpt = "prefix ".repeat(80) + "needle_anchor " + &"suffix ".repeat(80);
        let compact = compact_excerpt_for_budget(
            &excerpt,
            "needle_anchor",
            &["needle_anchor".to_string()],
            120,
        );
        assert!(compact.contains("needle_anchor"));
        assert!(compact.chars().count() <= 126);
    }

    #[test]
    fn compact_excerpt_preserves_short_text() {
        let excerpt = "short text";
        let compact = compact_excerpt_for_budget(excerpt, "short", &["short".to_string()], 120);
        assert_eq!(compact, "short text");
    }
}
