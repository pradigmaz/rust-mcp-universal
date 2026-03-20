pub(super) fn normalize(raw: &str) -> String {
    let normalized = raw
        .replace('\\', "/")
        .trim()
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string();
    #[cfg(windows)]
    {
        normalized.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        normalized
    }
}

pub(super) fn normalize_match_candidates(raw: &str) -> Vec<String> {
    let normalized = normalize(raw);
    let mut out = vec![normalized.clone()];
    if let Some(decoded) = decode_scope_path(&normalized) {
        let decoded_norm = normalize(&decoded);
        if decoded_norm != normalized {
            out.push(decoded_norm);
        }
    }
    out
}

#[cfg(windows)]
pub(super) fn decode_scope_path(input: &str) -> Option<String> {
    let chars = input.chars().collect::<Vec<_>>();
    let mut idx = 0_usize;
    let mut utf16 = Vec::with_capacity(chars.len());
    let mut changed = false;

    while idx < chars.len() {
        if chars[idx] == '%'
            && idx + 5 < chars.len()
            && chars[idx + 1].eq_ignore_ascii_case(&'u')
            && chars[idx + 2].is_ascii_hexdigit()
            && chars[idx + 3].is_ascii_hexdigit()
            && chars[idx + 4].is_ascii_hexdigit()
            && chars[idx + 5].is_ascii_hexdigit()
        {
            let unit = u16::from_str_radix(&chars[idx + 2..idx + 6].iter().collect::<String>(), 16)
                .ok()?;
            utf16.push(unit);
            idx += 6;
            changed = true;
            continue;
        }
        let mut buf = [0u16; 2];
        let encoded = chars[idx].encode_utf16(&mut buf);
        utf16.extend_from_slice(encoded);
        idx += 1;
    }

    if !changed {
        return None;
    }
    Some(
        char::decode_utf16(utf16)
            .map(|result| result.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect::<String>(),
    )
}

#[cfg(not(windows))]
pub(super) fn decode_scope_path(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut idx = 0_usize;
    let mut out = Vec::with_capacity(bytes.len());
    let mut changed = false;

    while idx < bytes.len() {
        if bytes[idx] == b'%'
            && idx + 2 < bytes.len()
            && (bytes[idx + 1] as char).is_ascii_hexdigit()
            && (bytes[idx + 2] as char).is_ascii_hexdigit()
        {
            let hi = (bytes[idx + 1] as char).to_digit(16)?;
            let lo = (bytes[idx + 2] as char).to_digit(16)?;
            out.push(((hi << 4) | lo) as u8);
            idx += 3;
            changed = true;
            continue;
        }
        out.push(bytes[idx]);
        idx += 1;
    }

    if !changed {
        return None;
    }
    String::from_utf8(out).ok()
}
