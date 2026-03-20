use anyhow::Result;

mod brace;
mod matcher;
mod segment;
mod segment_eval;

pub(super) fn has_glob_meta(pattern: &str) -> bool {
    pattern
        .chars()
        .any(|ch| matches!(ch, '*' | '?' | '[' | ']'))
        || contains_extglob_prefix(pattern)
        || contains_brace_alternation(pattern)
}

fn contains_extglob_prefix(pattern: &str) -> bool {
    let chars = pattern.chars().collect::<Vec<_>>();
    for idx in 0..chars.len().saturating_sub(1) {
        if chars[idx + 1] == '(' && matches!(chars[idx], '@' | '!' | '+' | '*' | '?') {
            return true;
        }
    }
    false
}

fn contains_brace_alternation(pattern: &str) -> bool {
    let mut offset = 0_usize;
    let mut rest = pattern;
    while let Some((open, close)) = brace::find_first_brace_group(rest) {
        let inner = &rest[open + 1..close];
        if brace::split_top_level(inner, ',').len() >= 2 {
            return true;
        }
        let next_offset = close + 1;
        offset += next_offset;
        if offset >= pattern.len() {
            break;
        }
        rest = &pattern[offset..];
    }
    false
}

pub(super) fn validate_glob_variant(pattern: &str) -> Result<()> {
    for segment in matcher::split_segments(pattern) {
        if segment == "**" {
            continue;
        }
        let _ = segment::parse_segment_tokens(segment)?;
    }
    Ok(())
}

pub(super) fn expand_braces(pattern: &str) -> Result<Vec<String>> {
    brace::expand_braces(pattern)
}

#[cfg(test)]
pub(super) fn glob_match(pattern: &str, path: &str) -> bool {
    let variants = match expand_braces(pattern) {
        Ok(value) => value,
        Err(_) => return false,
    };
    variants
        .iter()
        .any(|variant| matcher::glob_match_single_variant(variant, path))
}

pub(super) fn glob_match_single_variant(pattern: &str, path: &str) -> bool {
    matcher::glob_match_single_variant(pattern, path)
}
