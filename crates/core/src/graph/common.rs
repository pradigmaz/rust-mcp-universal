pub(super) fn iter_call_candidates(line: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    for (idx, ch) in line.char_indices() {
        if ch != '(' {
            continue;
        }
        let prefix = &line[..idx];
        let Some((start_idx, candidate)) = read_call_candidate(prefix) else {
            continue;
        };
        out.push((candidate, start_idx));
    }
    out
}

pub(super) fn iter_path_candidates(line: &str) -> Vec<(String, usize, usize)> {
    let mut out = Vec::new();
    let mut chars = line.char_indices().peekable();
    while let Some((start_idx, ch)) = chars.next() {
        if !is_identifier_start(ch) {
            continue;
        }
        if start_idx > 0
            && line[..start_idx]
                .chars()
                .next_back()
                .is_some_and(is_path_symbol_char)
        {
            continue;
        }

        let mut end_idx = start_idx + ch.len_utf8();
        while let Some(&(idx, next)) = chars.peek() {
            if !is_path_symbol_char(next) {
                break;
            }
            end_idx = idx + next.len_utf8();
            chars.next();
        }

        let candidate = line[start_idx..end_idx].trim_end_matches("::").trim();
        if candidate.is_empty() {
            continue;
        }
        out.push((candidate.to_string(), start_idx, end_idx));
    }
    out
}

pub(super) fn strip_rust_item_modifiers(mut text: &str) -> &str {
    loop {
        let trimmed = text.trim_start();
        if let Some(rest) = trimmed.strip_prefix("pub(") {
            let Some(close_idx) = rest.find(')') else {
                return trimmed;
            };
            text = &rest[close_idx + 1..];
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("pub ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("async ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("const ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("unsafe ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("default ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("extern ") {
            text = strip_extern_abi(rest);
            continue;
        }
        return trimmed;
    }
}

pub(super) fn strip_javascript_item_modifiers(mut text: &str) -> &str {
    loop {
        let trimmed = text.trim_start();
        if let Some(rest) = trimmed.strip_prefix("export ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("default ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("declare ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("abstract ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("async ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("public ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("private ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("protected ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("static ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("readonly ") {
            text = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("override ") {
            text = rest;
            continue;
        }
        return trimmed;
    }
}

pub(super) fn strip_extern_abi(text: &str) -> &str {
    let trimmed = text.trim_start();
    if !trimmed.starts_with('"') {
        return trimmed;
    }
    let bytes = trimmed.as_bytes();
    let mut idx = 1;
    while idx < bytes.len() {
        if bytes[idx] == b'"' {
            return trimmed[idx + 1..].trim_start();
        }
        idx += 1;
    }
    trimmed
}

pub(super) fn parse_impl_target(rest: &str) -> Option<String> {
    let mut text = rest.trim_start();
    if let Some(stripped) = strip_leading_angle_group(text) {
        text = stripped.trim_start();
    }
    let target_segment = if let Some((_, rhs)) = text.split_once(" for ") {
        rhs
    } else {
        text
    };
    read_path_tail_identifier(target_segment)
}

pub(super) fn strip_leading_angle_group(text: &str) -> Option<&str> {
    let mut depth = 0_u32;
    let mut seen_open = false;
    for (idx, ch) in text.char_indices() {
        match ch {
            '<' => {
                depth += 1;
                seen_open = true;
            }
            '>' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 && seen_open {
                    return Some(&text[idx + 1..]);
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn read_path_tail_identifier(text: &str) -> Option<String> {
    let head = text
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '{' | '(' | '<' | ';'))
        .next()
        .unwrap_or("")
        .trim();
    let candidate = head.trim_end_matches("::");
    let tail = candidate.rsplit("::").next().unwrap_or(candidate);
    read_identifier(tail)
}

pub(super) fn strip_line_comment(line: &str) -> &str {
    line.split("//").next().unwrap_or(line)
}

pub(super) fn find_identifier_column(line: &str, identifier: &str) -> Option<usize> {
    line.find(identifier)
        .map(|byte_idx| column_from_byte_index(line, byte_idx))
}

pub(super) fn column_from_byte_index(line: &str, byte_idx: usize) -> usize {
    line[..byte_idx].chars().count() + 1
}

pub(super) fn read_identifier(text: &str) -> Option<String> {
    let ident = text
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>();

    if ident.is_empty() { None } else { Some(ident) }
}

pub(super) fn extract_javascript_quoted_argument(text: &str, marker: &str) -> Option<String> {
    let (_, rest) = text.split_once(marker)?;
    read_javascript_string_literal(rest)
}

pub(super) fn read_javascript_string_literal(text: &str) -> Option<String> {
    let trimmed = text.trim_start();
    let mut chars = trimmed.char_indices();
    let (_, quote) = chars.next()?;
    if !matches!(quote, '"' | '\'') {
        return None;
    }

    let mut escaped = false;
    for (idx, ch) in chars {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            ch if ch == quote => return Some(trimmed[1..idx].to_string()),
            _ => {}
        }
    }

    None
}

fn read_call_candidate(prefix: &str) -> Option<(usize, String)> {
    let mut end = prefix.len();
    while end > 0 {
        let ch = prefix[..end].chars().next_back()?;
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
        let ch = prefix[..start].chars().next_back()?;
        if is_call_symbol_char(ch) {
            start -= ch.len_utf8();
            continue;
        }
        break;
    }

    let candidate = prefix[start..end].trim();
    if candidate.is_empty() {
        return None;
    }
    Some((start, candidate.to_string()))
}

fn is_call_symbol_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':' | '.' | '!')
}

fn is_path_symbol_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':' | '.')
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}
