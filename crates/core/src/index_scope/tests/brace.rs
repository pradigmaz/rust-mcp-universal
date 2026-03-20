use super::super::ScopeRule;
use super::super::glob::glob_match;

#[test]
fn glob_supports_brace_alternatives() {
    assert!(glob_match("{src,tests}/**/*.rs", "src/main.rs"));
    assert!(glob_match("{src,tests}/**/*.rs", "tests/unit/mod.rs"));
    assert!(!glob_match("{src,tests}/**/*.rs", "vendor/main.rs"));
}

#[test]
fn malformed_brace_patterns_are_treated_as_literals() {
    let open_only = ScopeRule::parse("src/{foo").expect("rule").expect("rule");
    assert!(open_only.matches("src/{foo/main.rs"));
    assert!(!open_only.matches("src/foo/main.rs"));

    let close_only = ScopeRule::parse("src/foo}").expect("rule").expect("rule");
    assert!(close_only.matches("src/foo}/main.rs"));
    assert!(!close_only.matches("src/foo/main.rs"));
}

#[test]
fn malformed_brace_patterns_with_glob_meta_are_treated_as_literals() {
    let glob_rule = ScopeRule::parse("src/*/{foo").expect("rule").expect("rule");
    assert!(glob_rule.matches("src/dir/{foo"));
    assert!(!glob_rule.matches("src/dir/foo"));

    let extglob_rule = ScopeRule::parse("@(src|tests)/{foo")
        .expect("rule")
        .expect("rule");
    assert!(extglob_rule.matches("src/{foo"));
    assert!(!extglob_rule.matches("src/foo"));
}

#[test]
fn malformed_braces_do_not_trigger_partial_expansion() {
    let rule = ScopeRule::parse("{src,tests}}/*.rs")
        .expect("rule")
        .expect("rule");
    assert!(rule.matches("{src,tests}}/main.rs"));
    assert!(!rule.matches("src}/main.rs"));
}

#[test]
fn braces_without_alternatives_are_treated_as_literals() {
    let rule = ScopeRule::parse("dir/{raw}").expect("rule").expect("rule");
    assert!(rule.matches("dir/{raw}/file.rs"));
    assert!(!rule.matches("dir/raw/file.rs"));

    assert!(glob_match("dir/*/{raw}.rs", "dir/sub/{raw}.rs"));
    assert!(!glob_match("dir/*/{raw}.rs", "dir/sub/raw.rs"));
}

#[test]
fn brace_expansion_is_bounded() {
    let pattern = std::iter::repeat_n("{a,b}", 13).collect::<String>();
    assert!(ScopeRule::parse(&pattern).is_err());
}
