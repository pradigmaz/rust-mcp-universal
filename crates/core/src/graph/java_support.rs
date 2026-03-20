use super::common::read_identifier;

pub(super) fn normalize_java_type_ref(candidate: &str) -> String {
    if let Some((head, tail)) = candidate.rsplit_once('.')
        && is_all_caps_identifier(tail)
    {
        return head.to_string();
    }
    candidate.to_string()
}

pub(super) fn read_trailing_identifier(text: &str) -> Option<(usize, String)> {
    let mut end = text.len();
    while end > 0 {
        let ch = text[..end].chars().next_back()?;
        if ch.is_whitespace() {
            end -= ch.len_utf8();
            continue;
        }
        break;
    }
    if end == 0 {
        return None;
    }

    let mut start = end;
    while start > 0 {
        let ch = text[..start].chars().next_back()?;
        if ch.is_ascii_alphanumeric() || ch == '_' {
            start -= ch.len_utf8();
            continue;
        }
        break;
    }

    let name = read_identifier(&text[start..end])?;
    Some((start, name))
}

pub(super) fn strip_java_comments(raw_line: &str, in_block_comment: &mut bool) -> String {
    let mut out = String::with_capacity(raw_line.len());
    let mut idx = 0;
    while idx < raw_line.len() {
        let rest = &raw_line[idx..];
        if *in_block_comment {
            if rest.starts_with("*/") {
                *in_block_comment = false;
                out.push_str("  ");
                idx += 2;
            } else {
                let ch = rest.chars().next().unwrap_or(' ');
                out.push(if ch == '\t' { '\t' } else { ' ' });
                idx += ch.len_utf8();
            }
            continue;
        }

        if rest.starts_with("/*") {
            *in_block_comment = true;
            out.push_str("  ");
            idx += 2;
            continue;
        }
        if rest.starts_with("//") {
            out.push_str(&" ".repeat(rest.len()));
            break;
        }

        let ch = rest.chars().next().unwrap_or(' ');
        out.push(ch);
        idx += ch.len_utf8();
    }
    out
}

pub(super) fn strip_java_item_modifiers(mut text: &str) -> &str {
    loop {
        let trimmed = text.trim_start();
        if let Some(rest) = strip_leading_java_annotation(trimmed) {
            text = rest;
            continue;
        }
        let mut stripped = None;
        for modifier in [
            "public ",
            "private ",
            "protected ",
            "static ",
            "final ",
            "abstract ",
            "synchronized ",
            "native ",
            "strictfp ",
            "transient ",
            "volatile ",
            "default ",
            "sealed ",
            "non-sealed ",
        ] {
            if let Some(rest) = trimmed.strip_prefix(modifier) {
                stripped = Some(rest);
                break;
            }
        }
        if let Some(rest) = stripped {
            text = rest;
            continue;
        }
        return trimmed;
    }
}

pub(super) fn starts_with_uppercase(text: &str) -> bool {
    text.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
}

fn is_all_caps_identifier(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

fn strip_balanced_group(text: &str, open: char, close: char) -> Option<&str> {
    let mut chars = text.char_indices();
    let (_, first) = chars.next()?;
    if first != open {
        return Some(text);
    }

    let mut depth = 1_u32;
    for (idx, ch) in chars {
        match ch {
            ch if ch == open => depth += 1,
            ch if ch == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[idx + ch.len_utf8()..]);
                }
            }
            _ => {}
        }
    }
    None
}

fn strip_leading_java_annotation(text: &str) -> Option<&str> {
    if !text.starts_with('@') || text.starts_with("@interface ") {
        return None;
    }

    let ident_end = text
        .char_indices()
        .skip(1)
        .take_while(|(_, ch)| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.'))
        .last()
        .map_or(1, |(idx, ch)| idx + ch.len_utf8());
    let rest = text[ident_end..].trim_start();
    if let Some(after_parens) = strip_balanced_group(rest, '(', ')') {
        Some(after_parens.trim_start())
    } else {
        Some(rest)
    }
}
