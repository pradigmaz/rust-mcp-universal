use crate::model::IndexingOptions;

use super::super::IndexScope;

#[test]
fn scope_handles_windows_separators() {
    let scope = IndexScope::new(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec!["src".to_string()],
        exclude_paths: vec![],
        reindex: false,
    })
    .expect("scope");

    assert!(scope.allows(r"src\main.rs"));
    assert!(!scope.allows(r"vendor\main.rs"));
}

#[cfg(windows)]
#[test]
fn scope_is_case_insensitive_on_windows() {
    let scope = IndexScope::new(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec!["SRC".to_string()],
        exclude_paths: vec![],
        reindex: false,
    })
    .expect("scope");

    assert!(scope.allows("src/main.rs"));
    assert!(scope.allows("Src/Main.rs"));
}

#[cfg(windows)]
#[test]
fn scope_is_unicode_case_insensitive_on_windows() {
    let unicode_dir = "\u{043F}\u{0430}\u{043F}\u{043A}\u{0430}";
    let unicode_upper = unicode_dir.to_uppercase();
    let unicode_file = "\u{0444}\u{0430}\u{0439}\u{043B}.rs";
    let scope = IndexScope::new(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![unicode_upper],
        exclude_paths: vec![],
        reindex: false,
    })
    .expect("scope");

    assert!(scope.allows(&format!("{unicode_dir}/{unicode_file}")));
}
