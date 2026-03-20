use super::super::glob::glob_match;
use super::super::{MAX_EXTGLOB_ALTERNATIVES, MAX_EXTGLOB_NESTING, ScopeRule};

#[test]
fn extglob_alternative_count_is_bounded() {
    let alternatives = (0..=MAX_EXTGLOB_ALTERNATIVES)
        .map(|idx| format!("v{idx}"))
        .collect::<Vec<_>>()
        .join("|");
    let pattern = format!("@({alternatives})");
    assert!(ScopeRule::parse(&pattern).is_err());
}

#[test]
fn extglob_nesting_is_bounded() {
    let mut pattern = "leaf".to_string();
    for _ in 0..=MAX_EXTGLOB_NESTING {
        pattern = format!("@({pattern})");
    }
    assert!(ScopeRule::parse(&pattern).is_err());
}

#[test]
fn glob_supports_extglob_groups() {
    assert!(glob_match("@(src|tests)/*.rs", "src/main.rs"));
    assert!(glob_match("@(src|tests)/*.rs", "tests/main.rs"));
    assert!(!glob_match("@(src|tests)/*.rs", "vendor/main.rs"));

    assert!(glob_match("@(src|{tests,benches})/*.rs", "tests/main.rs"));
    assert!(glob_match("@(src|{tests,benches})/*.rs", "benches/main.rs"));

    assert!(glob_match("?(foo|bar).txt", ".txt"));
    assert!(glob_match("?(foo|bar).txt", "foo.txt"));
    assert!(!glob_match("?(foo|bar).txt", "baz.txt"));

    assert!(glob_match("+(ab|cd).txt", "ab.txt"));
    assert!(glob_match("+(ab|cd).txt", "abcdab.txt"));
    assert!(!glob_match("+(ab|cd).txt", ".txt"));

    assert!(glob_match("*(ab|cd).txt", ".txt"));
    assert!(glob_match("*(ab|cd).txt", "abcdab.txt"));

    assert!(glob_match("!(vendor)", "src"));
    assert!(!glob_match("!(vendor)", "vendor"));
}
