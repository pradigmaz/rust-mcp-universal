use std::path::Path;

pub const SAMPLE_LIMIT: usize = 4096;
pub const INDEX_FILE_LIMIT: u64 = 1024 * 1024;

pub fn normalize_path(path: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        return percent_encode_bytes(path.as_os_str().as_bytes());
    }

    #[cfg(windows)]
    {
        use std::fmt::Write as _;
        use std::os::windows::ffi::OsStrExt;

        let mut out = String::new();
        for unit in path.as_os_str().encode_wide() {
            if unit == u16::from(b'\\') || unit == u16::from(b'/') {
                out.push('/');
                continue;
            }
            if let Ok(ascii) = u8::try_from(unit) {
                if is_safe_ascii_path_byte(ascii) {
                    out.push(char::from(ascii));
                    continue;
                }
            }
            let _ = write!(&mut out, "%u{unit:04X}");
        }
        out
    }

    #[cfg(all(not(unix), not(windows)))]
    {
        path.to_string_lossy().replace('\\', "/")
    }
}

pub fn is_probably_ignored(path: &Path) -> bool {
    let text = normalize_path(path);
    if text == ".gitignore" || text.ends_with("/.gitignore") {
        return true;
    }
    let patterns = [
        ".git/",
        ".hg/",
        ".svn/",
        "target/",
        "dist/",
        "build/",
        "coverage/",
        "node_modules/",
        ".pnpm-store/",
        ".npm/",
        ".yarn/",
        ".parcel-cache/",
        ".next/",
        ".nuxt/",
        ".svelte-kit/",
        ".venv/",
        "venv/",
        "env/",
        ".tox/",
        ".nox/",
        "__pycache__/",
        ".mypy_cache/",
        ".pytest_cache/",
        ".ruff_cache/",
        ".pytype/",
        ".eggs/",
        ".gradle/",
        ".terraform/",
        ".serverless/",
        ".aws-sam/",
        ".direnv/",
        ".idea/",
        ".vscode/",
        ".rmu/",
    ];
    patterns.iter().any(|p| text.contains(p))
}

#[cfg(unix)]
fn percent_encode_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let mut out = String::with_capacity(bytes.len() * 3);
    for byte in bytes {
        if matches!(byte, b'/' | b'.' | b'-' | b'_')
            || byte.is_ascii_digit()
            || byte.is_ascii_uppercase()
            || byte.is_ascii_lowercase()
        {
            out.push(char::from(*byte));
        } else {
            out.push('%');
            out.push(char::from(HEX[(byte >> 4) as usize]));
            out.push(char::from(HEX[(byte & 0x0F) as usize]));
        }
    }
    out
}

#[cfg(windows)]
fn is_safe_ascii_path_byte(byte: u8) -> bool {
    matches!(byte, b'.' | b'-' | b'_')
        || byte.is_ascii_digit()
        || byte.is_ascii_uppercase()
        || byte.is_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{is_probably_ignored, normalize_path};
    use std::path::Path;

    #[test]
    fn normalize_path_preserves_directory_separators() {
        let normalized = normalize_path(Path::new("src/main.rs"));
        assert_eq!(normalized, "src/main.rs");
    }

    #[cfg(unix)]
    #[test]
    fn normalize_path_encodes_backslash_character_on_unix() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::path::PathBuf;

        let path = PathBuf::from(OsStr::from_bytes(b"src\\main.rs"));
        let normalized = normalize_path(path.as_path());
        assert_eq!(normalized, "src%5Cmain.rs");
    }

    #[cfg(unix)]
    #[test]
    fn normalize_path_percent_encodes_non_utf8_sequences_without_collisions() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::path::PathBuf;

        let first = PathBuf::from(OsStr::from_bytes(b"src/\xFF.rs"));
        let second = PathBuf::from(OsStr::from_bytes(b"src/\xFE.rs"));

        let first_norm = normalize_path(first.as_path());
        let second_norm = normalize_path(second.as_path());

        assert_ne!(first_norm, second_norm);
        assert!(first_norm.contains("%FF"));
        assert!(second_norm.contains("%FE"));
    }

    #[cfg(unix)]
    #[test]
    fn normalize_path_avoids_percent_collision_between_utf8_and_raw_bytes() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::path::PathBuf;

        let utf8_percent = PathBuf::from("%FF");
        let raw_byte = PathBuf::from(OsStr::from_bytes(b"\xFF"));

        let percent_norm = normalize_path(utf8_percent.as_path());
        let raw_norm = normalize_path(raw_byte.as_path());

        assert_eq!(percent_norm, "%25FF");
        assert_eq!(raw_norm, "%FF");
        assert_ne!(percent_norm, raw_norm);
    }

    #[cfg(windows)]
    #[test]
    fn normalize_path_distinguishes_different_surrogates() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use std::path::PathBuf;

        let first = PathBuf::from(OsString::from_wide(&[0xD800]));
        let second = PathBuf::from(OsString::from_wide(&[0xD801]));

        let first_norm = normalize_path(first.as_path());
        let second_norm = normalize_path(second.as_path());

        assert_ne!(first_norm, second_norm);
    }

    #[test]
    fn is_probably_ignored_matches_common_language_artifact_directories() {
        for path in [
            ".gitignore",
            "nested/.gitignore",
            "node_modules/react/index.js",
            ".pnpm-store/v3/files.json",
            ".yarn/cache/pkg.tgz",
            ".venv/lib/site.py",
            "venv/lib/site.py",
            "env/lib/site.py",
            ".pytest_cache/state",
            ".mypy_cache/module.json",
            ".gradle/build.bin",
            ".terraform/terraform.tfstate",
            ".serverless/output.json",
            ".aws-sam/build/template.yaml",
            ".direnv/python",
        ] {
            assert!(
                is_probably_ignored(Path::new(path)),
                "expected ignored: {path}"
            );
        }
        assert!(!is_probably_ignored(Path::new(
            "src/node_module_adapter.ts"
        )));
        assert!(!is_probably_ignored(Path::new("env.example")));
        assert!(!is_probably_ignored(Path::new("packages/build.rs")));
    }
}
