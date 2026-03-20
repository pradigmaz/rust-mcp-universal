use super::super::MAX_MATCH_TEXT_CHARS;
use super::super::glob::glob_match;

#[test]
fn glob_supports_recursive_double_star() {
    assert!(glob_match("**/*.rs", "src/main.rs"));
    assert!(glob_match("**/*.rs", "a/b/c/mod.rs"));
    assert!(!glob_match("**/*.rs", "src/main.py"));
}

#[test]
fn glob_supports_single_star_question_and_class() {
    assert!(glob_match("src/*.rs", "src/main.rs"));
    assert!(!glob_match("src/*.rs", "src/nested/main.rs"));
    assert!(glob_match("file?.txt", "file1.txt"));
    assert!(!glob_match("file?.txt", "file12.txt"));
    assert!(glob_match("file[0-9].txt", "file7.txt"));
    assert!(!glob_match("file[!0-9].txt", "file7.txt"));
}

#[test]
fn glob_runtime_guard_rejects_oversized_segments() {
    let oversized = "a".repeat(MAX_MATCH_TEXT_CHARS + 1);
    assert!(!glob_match("*", &oversized));
}

#[test]
fn glob_runtime_guard_rejects_oversized_state_space() {
    let pattern = "*".repeat(400);
    let text = "a".repeat(900);
    assert!(!glob_match(&pattern, &text));
}

#[test]
fn glob_handles_many_recursive_segments() {
    let pattern = std::iter::repeat_n("**", 48)
        .chain(std::iter::once("*.rs"))
        .collect::<Vec<_>>()
        .join("/");
    let path = std::iter::repeat_n("dir", 48)
        .chain(std::iter::once("main.rs"))
        .collect::<Vec<_>>()
        .join("/");

    assert!(glob_match(&pattern, &path));
}
