use std::path::Path;

pub fn infer_language(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "cpp",
        "cs" => "csharp",
        "php" => "php",
        "rb" => "ruby",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "scala" | "sc" => "scala",
        "lua" => "lua",
        "sh" | "bash" | "zsh" => "shell",
        "ps1" => "powershell",
        "sql" => "sql",
        "html" => "html",
        "css" | "scss" | "sass" | "less" => "css",
        "vue" => "vue",
        "svelte" => "svelte",
        "md" => "markdown",
        "json" => "json",
        "toml" => "toml",
        _ => "text",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::infer_language;

    #[test]
    fn infer_language_classifies_code_like_extensions_without_falling_back_to_text() {
        assert_eq!(infer_language(Path::new("src/app.mjs")), "javascript");
        assert_eq!(infer_language(Path::new("templates/index.html")), "html");
        assert_eq!(infer_language(Path::new("styles/site.css")), "css");
        assert_eq!(infer_language(Path::new("scripts/bootstrap.sh")), "shell");
        assert_eq!(infer_language(Path::new("scripts/setup.ps1")), "powershell");
    }
}
