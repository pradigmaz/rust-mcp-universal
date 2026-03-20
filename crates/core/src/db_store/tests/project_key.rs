use std::fs;

#[cfg(not(windows))]
use anyhow::Result;

use super::super::project_key;
use super::temp_dir;

#[cfg(not(windows))]
#[test]
fn project_key_preserves_case_on_case_sensitive_platforms() -> Result<()> {
    let root = temp_dir("rmu-project-key-case-sensitive");
    let upper = root.join("CaseProject");
    let lower = root.join("caseproject");
    fs::create_dir_all(&upper)?;
    fs::create_dir_all(&lower)?;

    let upper_key = project_key(&upper)?;
    let lower_key = project_key(&lower)?;
    assert_ne!(upper_key, lower_key);

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[cfg(windows)]
#[test]
fn project_key_normalizes_case_on_windows() -> anyhow::Result<()> {
    let root = temp_dir("rmu-project-key-case-insensitive-windows");
    let canonical_case = root.join("CaseProject");
    fs::create_dir_all(&canonical_case).expect("create test directory");

    let upper = std::path::PathBuf::from(canonical_case.to_string_lossy().to_uppercase());
    let lower = std::path::PathBuf::from(canonical_case.to_string_lossy().to_lowercase());
    if !(upper.exists() && lower.exists()) {
        // Skip on case-sensitive volumes where casing denotes a distinct path.
        let _ = fs::remove_dir_all(root);
        return Ok(());
    }
    assert_eq!(project_key(&upper)?, project_key(&lower)?);

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[cfg(windows)]
#[test]
fn project_key_rejects_unresolved_surrogate_paths() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let first = std::path::PathBuf::from(OsString::from_wide(&[0xD800]));
    let second = std::path::PathBuf::from(OsString::from_wide(&[0xD801]));

    assert!(project_key(&first).is_err());
    assert!(project_key(&second).is_err());
}

#[cfg(unix)]
#[test]
fn project_key_distinguishes_utf8_percent_and_raw_byte_paths() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let root = temp_dir("rmu-project-key-percent-collision");
    fs::create_dir_all(&root).expect("temp root");

    let utf8 = root.join("%FF");
    let raw = root.join(OsStr::from_bytes(b"\xFF"));
    fs::create_dir_all(&utf8).expect("utf8 dir");
    fs::create_dir_all(&raw).expect("raw-byte dir");

    assert_ne!(
        project_key(&utf8).expect("utf8 path key"),
        project_key(&raw).expect("raw byte path key")
    );

    let _ = fs::remove_dir_all(root);
}
