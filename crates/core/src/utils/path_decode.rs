use std::path::PathBuf;

pub fn decode_normalized_path(path: &str) -> Option<String> {
    #[cfg(unix)]
    {
        percent_decode_bytes(path.as_bytes())
    }

    #[cfg(windows)]
    {
        let chars = path.chars().collect::<Vec<_>>();
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
                let unit =
                    u16::from_str_radix(&chars[idx + 2..idx + 6].iter().collect::<String>(), 16)
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

    #[cfg(all(not(unix), not(windows)))]
    {
        let _ = path;
        None
    }
}

pub fn normalized_path_to_fs_path(path: &str) -> PathBuf {
    PathBuf::from(decode_normalized_path(path).unwrap_or_else(|| path.to_string()))
}

#[cfg(unix)]
fn percent_decode_bytes(bytes: &[u8]) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::{decode_normalized_path, normalized_path_to_fs_path};
    use std::path::Path;

    #[cfg(unix)]
    #[test]
    fn decode_normalized_path_round_trips_percent_encoded_ascii() {
        assert_eq!(
            decode_normalized_path("src/%5Bid%5D/page.rs").as_deref(),
            Some("src/[id]/page.rs")
        );
        assert_eq!(
            normalized_path_to_fs_path("src/%5Bid%5D/page.rs"),
            Path::new("src/[id]/page.rs")
        );
    }

    #[cfg(windows)]
    #[test]
    fn decode_normalized_path_round_trips_utf16_encoded_ascii() {
        assert_eq!(
            decode_normalized_path("src/%u005Bid%u005D/page.rs").as_deref(),
            Some("src/[id]/page.rs")
        );
        assert_eq!(
            normalized_path_to_fs_path("src/%u005Bid%u005D/page.rs"),
            Path::new("src/[id]/page.rs")
        );
    }
}
