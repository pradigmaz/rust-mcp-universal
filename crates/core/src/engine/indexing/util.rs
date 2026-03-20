pub(super) fn path_under_walk_error(path: &str, error_prefixes: &[String]) -> bool {
    error_prefixes.iter().any(|prefix| {
        path == prefix
            || path.starts_with(&format!("{prefix}/"))
            || prefix.starts_with(&format!("{path}/"))
    })
}

#[cfg(test)]
mod tests {
    use super::path_under_walk_error;

    #[test]
    fn path_under_walk_error_matches_exact_path() {
        assert!(path_under_walk_error(
            "src/main.rs",
            &[String::from("src/main.rs")]
        ));
    }

    #[test]
    fn path_under_walk_error_matches_parent_prefix() {
        assert!(path_under_walk_error(
            "src/nested/file.rs",
            &[String::from("src")]
        ));
    }

    #[test]
    fn path_under_walk_error_matches_when_error_is_child() {
        assert!(path_under_walk_error("src", &[String::from("src/private")]));
    }
}
