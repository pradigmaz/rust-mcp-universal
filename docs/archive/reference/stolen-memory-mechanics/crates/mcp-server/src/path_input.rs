use std::path::PathBuf;

use crate::state::normalize_existing_directory;

pub(crate) fn supported_directory_input_hint() -> &'static str {
    #[cfg(windows)]
    {
        "supported path forms on this runtime: Windows path or file:// URI"
    }

    #[cfg(not(windows))]
    {
        "supported path forms on this runtime: Unix path, file:// URI, or Windows path that maps to /mnt/<drive>/..."
    }
}

pub(crate) fn resolve_existing_directory_input(raw: &str) -> Option<PathBuf> {
    let path = parse_path_like(raw)?;
    normalize_existing_directory(&path)
}

pub(crate) fn parse_path_like(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("file://") {
        return parse_file_uri(trimmed);
    }
    parse_platform_specific_path(trimmed).or_else(|| Some(PathBuf::from(trimmed)))
}

fn parse_file_uri(raw: &str) -> Option<PathBuf> {
    let remainder = raw.strip_prefix("file://")?;
    if remainder.is_empty() {
        return None;
    }
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

#[cfg(windows)]
fn parse_platform_specific_path(_raw: &str) -> Option<PathBuf> {
    None
}

#[cfg(not(windows))]
fn parse_platform_specific_path(raw: &str) -> Option<PathBuf> {
    translate_windows_path_to_wsl(raw)
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
        None => normalize_unix_local_file_uri_path(decoded_path),
        Some(authority) if authority.eq_ignore_ascii_case("localhost") => {
            normalize_unix_local_file_uri_path(decoded_path)
        }
        Some(authority) => PathBuf::from(format!("//{authority}{decoded_path}")),
    }
}

#[cfg(not(windows))]
fn normalize_unix_local_file_uri_path(decoded_path: &str) -> PathBuf {
    translate_windows_path_to_wsl(decoded_path).unwrap_or_else(|| PathBuf::from(decoded_path))
}

#[cfg(not(windows))]
fn translate_windows_path_to_wsl(raw: &str) -> Option<PathBuf> {
    let candidate = raw.trim();
    let candidate = if candidate.starts_with('/') && candidate.as_bytes().get(2) == Some(&b':') {
        &candidate[1..]
    } else {
        candidate
    };
    let bytes = candidate.as_bytes();
    if bytes.len() < 3 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
        return None;
    }
    if bytes[2] != b'/' && bytes[2] != b'\\' {
        return None;
    }

    let drive = (bytes[0] as char).to_ascii_lowercase();
    let rest = candidate[3..].replace('\\', "/");
    let rest = rest.trim_start_matches('/');
    let translated = if rest.is_empty() {
        format!("/mnt/{drive}")
    } else {
        format!("/mnt/{drive}/{rest}")
    };
    Some(PathBuf::from(translated))
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
    use std::path::PathBuf;

    use super::parse_path_like;
    #[cfg(not(windows))]
    use super::resolve_existing_directory_input;

    #[test]
    fn parse_path_like_rejects_empty_inputs_and_malformed_file_uri() {
        assert!(parse_path_like("").is_none());
        assert!(parse_path_like("   ").is_none());
        assert!(parse_path_like("file://").is_none());
    }

    #[cfg(not(windows))]
    #[test]
    fn parse_path_like_translates_windows_forms_to_wsl_mounts() {
        assert_eq!(
            parse_path_like(r"C:\tmp\vault").expect("windows path"),
            PathBuf::from("/mnt/c/tmp/vault")
        );
        assert_eq!(
            parse_path_like("C:/tmp/vault").expect("windows path"),
            PathBuf::from("/mnt/c/tmp/vault")
        );
        assert_eq!(
            parse_path_like("file:///C:/tmp/vault").expect("file uri"),
            PathBuf::from("/mnt/c/tmp/vault")
        );
        assert_eq!(
            parse_path_like("/tmp/vault").expect("unix path"),
            PathBuf::from("/tmp/vault")
        );
    }

    #[cfg(windows)]
    #[test]
    fn parse_path_like_preserves_windows_native_forms() {
        assert_eq!(
            parse_path_like(r"C:\tmp\vault").expect("windows path"),
            PathBuf::from(r"C:\tmp\vault")
        );
        assert_eq!(
            parse_path_like("file:///C:/tmp/vault").expect("file uri"),
            PathBuf::from(r"C:\tmp\vault")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn resolve_existing_directory_input_accepts_translated_existing_windows_path() {
        let root = temp_dir_on_mounted_drive("path-input-existing");
        let windows_path = unix_mount_to_windows_path(&root).expect("windows path");

        let resolved =
            resolve_existing_directory_input(&windows_path).expect("translated existing path");

        assert_eq!(
            resolved,
            std::fs::canonicalize(&root).expect("canonical root")
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(not(windows))]
    #[test]
    fn resolve_existing_directory_input_rejects_missing_translated_windows_path() {
        let root = temp_dir_on_mounted_drive("path-input-missing");
        let windows_path = unix_mount_to_windows_path(&root).expect("windows path");
        let missing = format!(r"{}\missing-vault", windows_path);

        assert!(resolve_existing_directory_input(&missing).is_none());

        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(not(windows))]
    fn temp_dir_on_mounted_drive(prefix: &str) -> PathBuf {
        let cwd = std::env::current_dir()
            .expect("cwd")
            .canonicalize()
            .expect("canonical cwd");
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = cwd
            .join("target")
            .join("path-input-tests")
            .join(format!("{prefix}-{suffix}"));
        std::fs::create_dir_all(&root).expect("create mounted temp dir");
        root
    }

    #[cfg(not(windows))]
    fn unix_mount_to_windows_path(path: &std::path::Path) -> Option<String> {
        let canonical = std::fs::canonicalize(path).ok()?;
        let raw = canonical.to_string_lossy().replace('\\', "/");
        let rest = raw.strip_prefix("/mnt/")?;
        let (drive, tail) = rest.split_once('/')?;
        Some(format!(
            "{}:\\{}",
            drive.to_ascii_uppercase(),
            tail.replace('/', "\\")
        ))
    }
}
