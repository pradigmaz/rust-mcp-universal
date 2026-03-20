use std::path::Path;

use crate::model::{IndexProfile, IndexingOptions};
use crate::utils::normalize_path;

#[cfg(not(windows))]
use super::super::normalize::decode_scope_path;
use super::super::{IndexScope, ScopeRule};

#[test]
fn path_rule_matches_directory_and_single_file() {
    let dir_rule = ScopeRule::parse("src").expect("rule").expect("rule");
    assert!(dir_rule.matches("src/main.rs"));
    assert!(!dir_rule.matches("vendor/main.rs"));

    let file_rule = ScopeRule::parse("src/main.rs")
        .expect("rule")
        .expect("rule");
    assert!(file_rule.matches("src/main.rs"));
    assert!(!file_rule.matches("src/lib.rs"));
}

#[test]
fn path_rule_supports_directories_with_dots() {
    let rule = ScopeRule::parse(".github").expect("rule").expect("rule");
    assert!(rule.matches(".github/workflows/ci.yml"));
    assert!(!rule.matches(".gitignore"));
}

#[test]
fn scope_applies_include_and_exclude_rules() {
    let scope = IndexScope::new(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec!["**/*.rs".to_string()],
        exclude_paths: vec!["vendor/**".to_string()],
        reindex: false,
    })
    .expect("scope");

    assert!(scope.allows("src/main.rs"));
    assert!(!scope.allows("vendor/lib.rs"));
    assert!(!scope.allows("src/main.py"));
}

#[test]
fn scope_matches_normalize_path_encoded_input() {
    let unicode_dir = "\u{043F}\u{0430}\u{043F}\u{043A}\u{0430}";
    let unicode_file = "\u{0444}\u{0430}\u{0439}\u{043B}.rs";
    let unicode_scope = IndexScope::new(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![unicode_dir.to_string()],
        exclude_paths: vec![],
        reindex: false,
    })
    .expect("scope");
    let unicode_encoded = normalize_path(Path::new(&format!("{unicode_dir}/{unicode_file}")));
    assert!(unicode_scope.allows(&unicode_encoded));

    let spaced_scope = IndexScope::new(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec!["my dir".to_string()],
        exclude_paths: vec![],
        reindex: false,
    })
    .expect("scope");
    let spaced_encoded = normalize_path(Path::new("my dir/file.rs"));
    assert!(spaced_scope.allows(&spaced_encoded));
}

#[test]
fn manual_include_paths_narrow_profile_scope() -> anyhow::Result<()> {
    let scope = IndexScope::new(&IndexingOptions {
        profile: Some(IndexProfile::RustMonorepo),
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec!["crates".to_string()],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert!(scope.allows("crates/core/src/lib.rs"));
    assert!(!scope.allows("src/main.rs"));
    assert!(!scope.allows("docs/guide.md"));
    Ok(())
}

#[test]
fn profile_excludes_are_combined_with_manual_excludes() -> anyhow::Result<()> {
    let scope = IndexScope::new(&IndexingOptions {
        profile: Some(IndexProfile::Mixed),
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec!["vendor/**".to_string()],
        reindex: false,
    })?;

    assert!(scope.allows("src/main.rs"));
    assert!(!scope.allows(".next/cache/index.js"));
    assert!(!scope.allows("vendor/lib.rs"));
    Ok(())
}

#[cfg(not(windows))]
#[test]
fn percent_decoding_rejects_invalid_utf8_sequences() {
    assert!(decode_scope_path("%FF").is_none());
}
