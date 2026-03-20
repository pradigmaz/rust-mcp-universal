use anyhow::{Result, bail};

use super::super::super::{MAX_EXTGLOB_ALTERNATIVES, MAX_EXTGLOB_NESTING, MAX_SEGMENT_TOKENS};
use super::super::brace::split_top_level;
use super::{CharClass, ClassItem, GroupKind, SegmentToken};

pub(super) fn parse_segment_tokens_with_depth(
    pattern: &str,
    depth: usize,
) -> Result<Vec<SegmentToken>> {
    if depth > MAX_EXTGLOB_NESTING {
        bail!("scope extglob nesting is too deep (>{MAX_EXTGLOB_NESTING})");
    }
    let chars = pattern.chars().collect::<Vec<_>>();
    let mut idx = 0_usize;
    let mut out = Vec::new();
    while idx < chars.len() {
        if let Some(kind) = extglob_kind(chars[idx]) {
            if idx + 1 < chars.len() && chars[idx + 1] == '(' {
                let (group, next) = parse_extglob_group(&chars, idx, kind, depth)?;
                out.push(group);
                idx = next;
                if out.len() > MAX_SEGMENT_TOKENS {
                    bail!("scope segment contains too many tokens (>{MAX_SEGMENT_TOKENS})");
                }
                continue;
            }
        }

        match chars[idx] {
            '*' => {
                out.push(SegmentToken::AnySeq);
                idx += 1;
            }
            '?' => {
                out.push(SegmentToken::AnyChar);
                idx += 1;
            }
            '[' => {
                let (class, next) = parse_char_class(&chars, idx)?;
                out.push(SegmentToken::CharClass(class));
                idx = next;
            }
            ch => {
                out.push(SegmentToken::Literal(ch));
                idx += 1;
            }
        }
        if out.len() > MAX_SEGMENT_TOKENS {
            bail!("scope segment contains too many tokens (>{MAX_SEGMENT_TOKENS})");
        }
    }
    Ok(out)
}

fn extglob_kind(ch: char) -> Option<GroupKind> {
    match ch {
        '@' => Some(GroupKind::One),
        '?' => Some(GroupKind::ZeroOrOne),
        '+' => Some(GroupKind::OneOrMore),
        '*' => Some(GroupKind::ZeroOrMore),
        '!' => Some(GroupKind::Negated),
        _ => None,
    }
}

fn parse_extglob_group(
    chars: &[char],
    start: usize,
    kind: GroupKind,
    depth: usize,
) -> Result<(SegmentToken, usize)> {
    if depth >= MAX_EXTGLOB_NESTING {
        bail!("scope extglob nesting is too deep (>{MAX_EXTGLOB_NESTING})");
    }
    let open = start + 1;
    let close = find_matching_paren(chars, open)?;
    let inner = chars[open + 1..close].iter().collect::<String>();
    let raw_alternatives = split_top_level(&inner, '|');
    if raw_alternatives.len() > MAX_EXTGLOB_ALTERNATIVES {
        bail!("scope extglob contains too many alternatives (>{MAX_EXTGLOB_ALTERNATIVES})");
    }
    let alternatives = raw_alternatives
        .into_iter()
        .map(|variant| parse_segment_tokens_with_depth(&variant, depth + 1))
        .collect::<Result<Vec<_>>>()?;
    Ok((SegmentToken::Group(kind, alternatives), close + 1))
}

fn find_matching_paren(chars: &[char], open: usize) -> Result<usize> {
    if chars.get(open) != Some(&'(') {
        bail!("expected `(` while parsing extglob");
    }
    let mut depth = 1_usize;
    let mut in_char_class = false;
    for (idx, ch) in chars.iter().enumerate().skip(open + 1) {
        if in_char_class {
            if *ch == ']' {
                in_char_class = false;
            }
            continue;
        }
        match ch {
            '[' => in_char_class = true,
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(idx);
                }
            }
            _ => {}
        }
    }
    bail!("unterminated extglob group")
}

fn parse_char_class(chars: &[char], start: usize) -> Result<(CharClass, usize)> {
    let mut idx = start + 1;
    let mut negated = false;
    if idx < chars.len() && matches!(chars[idx], '!' | '^') {
        negated = true;
        idx += 1;
    }

    let mut items = Vec::new();
    while idx < chars.len() {
        if chars[idx] == ']' && !items.is_empty() {
            return Ok((CharClass { negated, items }, idx + 1));
        }

        let ch = chars[idx];
        if idx + 2 < chars.len() && chars[idx + 1] == '-' && chars[idx + 2] != ']' {
            let end = chars[idx + 2];
            items.push(ClassItem::Range(ch, end));
            idx += 3;
            continue;
        }

        items.push(ClassItem::Single(ch));
        idx += 1;
    }

    bail!("unterminated character class in scope pattern")
}
