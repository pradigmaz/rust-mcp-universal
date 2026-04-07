use std::path::PathBuf;

use crate::state::normalize_existing_directory;

pub(crate) fn resolve_existing_directory_input(raw: &str) -> Option<PathBuf> {
    let path = parse_path_like(raw)?;
    normalize_existing_directory(&path)
}

pub(crate) fn parse_path_like(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    parse_file_uri(trimmed).or_else(|| Some(PathBuf::from(trimmed)))
}

fn parse_file_uri(raw: &str) -> Option<PathBuf> {
    let remainder = raw.strip_prefix("file://")?;
    let (authority, raw_path) = split_file_uri(remainder);
    let decoded_path = percent_decode(raw_path);
    let decoded_authority = authority.map(percent_decode);

    #[cfg(windows)]
    {
        Some(parse_windows_file_uri(
            decoded_authority.as_deref(),
            &decoded_path,
        ))
    }

    #[cfg(not(windows))]
    {
        Some(parse_unix_file_uri(
            decoded_authority.as_deref(),
            &decoded_path,
        ))
    }
}

fn split_file_uri(remainder: &str) -> (Option<&str>, &str) {
    if remainder.starts_with('/') {
        return (None, remainder);
    }

    match remainder.split_once('/') {
        Some((authority, _path)) => (Some(authority), &remainder[authority.len()..]),
        None => (Some(remainder), ""),
    }
}

#[cfg(windows)]
fn parse_windows_file_uri(authority: Option<&str>, decoded_path: &str) -> PathBuf {
    match authority.filter(|value| !value.is_empty()) {
        None => normalize_windows_local_file_uri_path(decoded_path),
        Some(authority) if authority.eq_ignore_ascii_case("localhost") => {
            normalize_windows_local_file_uri_path(decoded_path)
        }
        Some(authority) => {
            let share_path = decoded_path.trim_start_matches('/').replace('/', "\\");
            if share_path.is_empty() {
                PathBuf::from(format!(r"\\{authority}"))
            } else {
                PathBuf::from(format!(r"\\{authority}\{share_path}"))
            }
        }
    }
}

#[cfg(windows)]
fn normalize_windows_local_file_uri_path(decoded_path: &str) -> PathBuf {
    let without_drive_prefix =
        if decoded_path.starts_with('/') && decoded_path.as_bytes().get(2) == Some(&b':') {
            &decoded_path[1..]
        } else {
            decoded_path
        };
    PathBuf::from(without_drive_prefix.replace('/', "\\"))
}

#[cfg(not(windows))]
fn parse_unix_file_uri(authority: Option<&str>, decoded_path: &str) -> PathBuf {
    match authority.filter(|value| !value.is_empty()) {
        None => PathBuf::from(decoded_path),
        Some(authority) if authority.eq_ignore_ascii_case("localhost") => {
            PathBuf::from(decoded_path)
        }
        Some(authority) => PathBuf::from(format!("//{authority}{decoded_path}")),
    }
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut index = 0usize;
    let mut decoded = Vec::with_capacity(raw.len());
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let pair = &raw[index + 1..index + 3];
            if let Ok(value) = u8::from_str_radix(pair, 16) {
                decoded.push(value);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use super::parse_path_like;
    use std::path::PathBuf;

    #[cfg(windows)]
    #[test]
    fn parses_localhost_file_uri_on_windows() {
        let parsed = parse_path_like("file://localhost/C:/Users/Test/project")
            .expect("localhost file URI should parse");
        assert_eq!(parsed, PathBuf::from(r"C:\Users\Test\project"));
    }

    #[cfg(windows)]
    #[test]
    fn parses_wsl_localhost_file_uri_on_windows() {
        let parsed = parse_path_like("file://wsl.localhost/Ubuntu/home/test/project")
            .expect("WSL file URI should parse");
        assert_eq!(
            parsed,
            PathBuf::from(r"\\wsl.localhost\Ubuntu\home\test\project")
        );
    }

    #[cfg(windows)]
    #[test]
    fn parses_wsl_dollar_file_uri_on_windows() {
        let parsed = parse_path_like("file://wsl$/Ubuntu/home/test/project")
            .expect("WSL file URI should parse");
        assert_eq!(parsed, PathBuf::from(r"\\wsl$\Ubuntu\home\test\project"));
    }

    #[cfg(not(windows))]
    #[test]
    fn parses_localhost_file_uri_on_unix() {
        let parsed =
            parse_path_like("file://localhost/tmp/project").expect("localhost URI should parse");
        assert_eq!(parsed, PathBuf::from("/tmp/project"));
    }

    #[cfg(not(windows))]
    #[test]
    fn preserves_non_local_authority_on_unix() {
        let parsed =
            parse_path_like("file://server/share/project").expect("authority URI should parse");
        assert_eq!(parsed, PathBuf::from("//server/share/project"));
    }
}
