use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub(crate) struct NormalizedToken {
    pub(crate) value: String,
    pub(crate) line: usize,
}

pub(crate) fn normalize_tokens(language: &str, source: &str) -> Vec<NormalizedToken> {
    let keywords = keywords_for(language);
    let mut out = Vec::new();
    let chars = source.chars().collect::<Vec<_>>();
    let mut i = 0;
    let mut line = 1;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\n' {
            line += 1;
            i += 1;
            continue;
        }
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch == '#' && language == "python" {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if ch == '#'
            && language == "rust"
            && (chars.get(i + 1) == Some(&'[')
                || (chars.get(i + 1) == Some(&'!') && chars.get(i + 2) == Some(&'[')))
        {
            let start_line = line;
            i += if chars.get(i + 1) == Some(&'!') { 3 } else { 2 };
            let mut depth = 1usize;
            while i < chars.len() && depth > 0 {
                if chars[i] == '\n' {
                    line += 1;
                }
                match chars[i] {
                    '[' => depth += 1,
                    ']' => depth = depth.saturating_sub(1),
                    _ => {}
                }
                i += 1;
            }
            out.push(NormalizedToken {
                value: "$attr".to_string(),
                line: start_line,
            });
            continue;
        }
        if ch == '@'
            && matches!(
                language,
                "python" | "java" | "typescript" | "tsx" | "javascript" | "jsx" | "mjs" | "cjs"
            )
        {
            let start_line = line;
            i += 1;
            let mut depth = 0usize;
            while i < chars.len() {
                if chars[i] == '\n' && depth == 0 && language == "python" {
                    break;
                }
                if chars[i] == '\n' {
                    line += 1;
                }
                match chars[i] {
                    '(' => depth += 1,
                    ')' => depth = depth.saturating_sub(1),
                    _ => {}
                }
                if depth == 0 && matches!(chars[i], ' ' | '\t' | '{') && language == "java" {
                    break;
                }
                i += 1;
            }
            out.push(NormalizedToken {
                value: "$attr".to_string(),
                line: start_line,
            });
            continue;
        }
        if language == "rust" && ch == '\'' && looks_like_rust_lifetime(&chars, i) {
            let start_line = line;
            i += 1;
            while i < chars.len() && is_ident_continue(chars[i]) {
                i += 1;
            }
            out.push(NormalizedToken {
                value: "$lifetime".to_string(),
                line: start_line,
            });
            continue;
        }
        if ch == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if ch == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() {
                if chars[i] == '\n' {
                    line += 1;
                }
                if chars[i] == '*' && chars[i + 1] == '/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            let quote = ch;
            let start_line = line;
            i += 1;
            while i < chars.len() {
                if chars[i] == '\n' {
                    line += 1;
                }
                if chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                if chars[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push(NormalizedToken {
                value: "$lit".to_string(),
                line: start_line,
            });
            continue;
        }
        if ch.is_ascii_digit() {
            let start_line = line;
            i += 1;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || matches!(chars[i], '_' | '.' | 'x'))
            {
                i += 1;
            }
            out.push(NormalizedToken {
                value: "$num".to_string(),
                line: start_line,
            });
            continue;
        }
        if is_ident_start(ch) {
            let start = i;
            let start_line = line;
            i += 1;
            while i < chars.len() && is_ident_continue(chars[i]) {
                i += 1;
            }
            let value = chars[start..i].iter().collect::<String>();
            let lowered = value.to_ascii_lowercase();
            let normalized = if language == "rust" && chars.get(i) == Some(&'!') {
                i += 1;
                "$macro".to_string()
            } else if keywords.contains(lowered.as_str()) {
                lowered
            } else {
                "$id".to_string()
            };
            out.push(NormalizedToken {
                value: normalized,
                line: start_line,
            });
            continue;
        }
        let start_line = line;
        if let Some(token) = two_char_token(&chars, i) {
            out.push(NormalizedToken {
                value: token.to_string(),
                line: start_line,
            });
            i += 2;
        } else {
            out.push(NormalizedToken {
                value: ch.to_string(),
                line: start_line,
            });
            i += 1;
        }
    }
    out
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn looks_like_rust_lifetime(chars: &[char], idx: usize) -> bool {
    let Some(next) = chars.get(idx + 1).copied() else {
        return false;
    };
    if !is_ident_start(next) {
        return false;
    }
    let mut end = idx + 2;
    while end < chars.len() && is_ident_continue(chars[end]) {
        end += 1;
    }
    chars.get(end) != Some(&'\'')
}

fn two_char_token(chars: &[char], idx: usize) -> Option<&'static str> {
    let pair = [chars.get(idx).copied()?, chars.get(idx + 1).copied()?];
    match pair {
        ['<', '/'] => Some("</"),
        ['/', '>'] => Some("/>"),
        [':', ':'] => Some("::"),
        ['-', '>'] => Some("->"),
        ['=', '>'] => Some("=>"),
        ['=', '='] => Some("=="),
        ['!', '='] => Some("!="),
        ['<', '='] => Some("<="),
        ['>', '='] => Some(">="),
        ['&', '&'] => Some("&&"),
        ['|', '|'] => Some("||"),
        ['+', '='] => Some("+="),
        ['-', '='] => Some("-="),
        ['*', '='] => Some("*="),
        ['/', '='] => Some("/="),
        _ => None,
    }
}

fn keywords_for(language: &str) -> BTreeSet<&'static str> {
    let values: &[&str] = match language {
        "rust" => &[
            "fn",
            "pub",
            "impl",
            "struct",
            "enum",
            "trait",
            "let",
            "mut",
            "if",
            "else",
            "match",
            "for",
            "while",
            "loop",
            "return",
            "use",
            "mod",
            "crate",
            "self",
            "super",
            "derive",
            "cfg",
            "allow",
            "serde",
            "default",
            "debug",
            "clone",
            "command",
            "arg",
            "subcommand",
            "value_enum",
        ],
        "python" => &[
            "def",
            "class",
            "if",
            "elif",
            "else",
            "for",
            "while",
            "return",
            "import",
            "from",
            "try",
            "except",
            "with",
            "as",
            "pass",
            "break",
            "continue",
            "lambda",
            "dataclass",
            "field",
            "basemodel",
            "modelconfig",
            "mapped",
            "mapped_column",
            "relationship",
            "column",
        ],
        "java" => &[
            "class",
            "interface",
            "record",
            "public",
            "private",
            "protected",
            "static",
            "void",
            "if",
            "else",
            "for",
            "while",
            "return",
            "new",
            "package",
            "import",
            "extends",
            "implements",
            "entity",
            "table",
            "column",
            "bean",
            "component",
            "configuration",
            "configurationproperties",
            "getter",
            "setter",
            "builder",
            "value",
            "data",
        ],
        _ => &[
            "function",
            "class",
            "export",
            "import",
            "from",
            "const",
            "let",
            "var",
            "if",
            "else",
            "for",
            "while",
            "return",
            "new",
            "extends",
            "implements",
            "async",
            "await",
            "children",
            "classname",
            "variant",
            "size",
            "props",
            "provider",
            "layout",
            "page",
            "section",
            "slot",
        ],
    };
    values.iter().copied().collect()
}

#[cfg(test)]
#[path = "tokenize_tests.rs"]
mod tests;
