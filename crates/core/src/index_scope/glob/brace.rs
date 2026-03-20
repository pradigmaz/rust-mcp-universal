use anyhow::{Result, bail};

use super::super::MAX_GLOB_VARIANTS;

pub(super) fn expand_braces(pattern: &str) -> Result<Vec<String>> {
    if has_unbalanced_braces(pattern) {
        return Ok(vec![pattern.to_string()]);
    }

    let Some((open, close)) = find_first_brace_group(pattern) else {
        return Ok(vec![pattern.to_string()]);
    };

    let prefix = &pattern[..open];
    let inner = &pattern[open + 1..close];
    let suffix = &pattern[close + 1..];
    let alternatives = split_top_level(inner, ',');
    if alternatives.len() < 2 {
        let expanded_suffix = expand_braces(suffix)?;
        return Ok(expanded_suffix
            .into_iter()
            .map(|tail| format!("{prefix}{{{inner}}}{tail}"))
            .collect());
    }

    let mut out = Vec::new();
    let expanded_suffix = expand_braces(suffix)?;
    for alternative in alternatives {
        let expanded_alternative = expand_braces(&alternative)?;
        for alt in expanded_alternative {
            for tail in &expanded_suffix {
                out.push(format!("{prefix}{alt}{tail}"));
                if out.len() > MAX_GLOB_VARIANTS {
                    bail!("scope pattern expands to too many variants (>{MAX_GLOB_VARIANTS})");
                }
            }
        }
    }
    Ok(out)
}

pub(super) fn find_first_brace_group(pattern: &str) -> Option<(usize, usize)> {
    let mut brace_depth = 0_usize;
    let mut in_char_class = false;
    let mut open_at: Option<usize> = None;

    for (idx, ch) in pattern.char_indices() {
        if in_char_class {
            if ch == ']' {
                in_char_class = false;
            }
            continue;
        }

        match ch {
            '[' => in_char_class = true,
            '{' => {
                if brace_depth == 0 {
                    open_at = Some(idx);
                }
                brace_depth += 1;
            }
            '}' => {
                if brace_depth == 0 {
                    continue;
                }
                brace_depth -= 1;
                if brace_depth == 0 {
                    return open_at.map(|open| (open, idx));
                }
            }
            _ => {}
        }
    }

    None
}

fn has_unbalanced_braces(pattern: &str) -> bool {
    let mut brace_depth = 0_usize;
    let mut in_char_class = false;

    for ch in pattern.chars() {
        if in_char_class {
            if ch == ']' {
                in_char_class = false;
            }
            continue;
        }
        match ch {
            '[' => in_char_class = true,
            '{' => brace_depth += 1,
            '}' => {
                if brace_depth == 0 {
                    return true;
                }
                brace_depth -= 1;
            }
            _ => {}
        }
    }

    brace_depth != 0
}

pub(super) fn split_top_level(input: &str, delimiter: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0_usize;
    let mut paren_depth = 0_usize;
    let mut brace_depth = 0_usize;
    let mut in_char_class = false;

    for (idx, ch) in input.char_indices() {
        if in_char_class {
            if ch == ']' {
                in_char_class = false;
            }
            continue;
        }

        match ch {
            '[' => in_char_class = true,
            '(' => paren_depth += 1,
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
            }
            '{' => brace_depth += 1,
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
            }
            _ if ch == delimiter && paren_depth == 0 && brace_depth == 0 => {
                out.push(input[start..idx].to_string());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    out.push(input[start..].to_string());
    out
}
