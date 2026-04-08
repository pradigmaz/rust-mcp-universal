use crate::model::FindingConfidence;

use super::{SecuritySmellFacts, SecuritySmellMatch};

pub(crate) fn analyze_security_smells(
    rel_path: &str,
    language: &str,
    full_text: &str,
) -> SecuritySmellFacts {
    if !supports_smell_scan(rel_path, language) {
        return SecuritySmellFacts::default();
    }

    let mut facts = SecuritySmellFacts::default();
    for (idx, line) in full_text.lines().enumerate() {
        let trimmed = line.trim();
        let location = Some(super::location::line_location(idx + 1, line.len()));
        if is_shell_exec_smell(language, trimmed) {
            record(
                &mut facts.shell_exec,
                location.clone(),
                FindingConfidence::Medium,
            );
        }
        if is_path_traversal_smell(language, trimmed) {
            record(
                &mut facts.path_traversal,
                location.clone(),
                FindingConfidence::Low,
            );
        }
        if is_raw_sql_smell(trimmed) {
            record(
                &mut facts.raw_sql,
                location.clone(),
                FindingConfidence::Medium,
            );
        }
        if is_unsafe_deserialize_smell(trimmed) {
            record(
                &mut facts.unsafe_deserialize,
                location,
                FindingConfidence::High,
            );
        }
    }
    facts
}

fn record(
    target: &mut SecuritySmellMatch,
    location: Option<crate::model::QualityLocation>,
    confidence: FindingConfidence,
) {
    target.match_count += 1;
    if target.location.is_none() {
        target.location = location;
        target.confidence = Some(confidence);
    }
}

fn supports_smell_scan(rel_path: &str, language: &str) -> bool {
    let path = rel_path.to_ascii_lowercase();
    !matches!(language, "markdown" | "text" | "json" | "yaml" | "toml")
        && !path.contains("/tests/")
        && !path.contains("/examples/")
        && !path.contains("/benches/")
}

fn is_shell_exec_smell(language: &str, trimmed: &str) -> bool {
    match language {
        "rust" => trimmed.contains("Command::new("),
        "python" => trimmed.contains("subprocess.") && trimmed.contains("shell=True"),
        "javascript" | "jsx" | "mjs" | "cjs" | "typescript" | "tsx" => {
            trimmed.contains("child_process.exec(") || trimmed.contains("execSync(")
        }
        _ => false,
    }
}

fn is_path_traversal_smell(language: &str, trimmed: &str) -> bool {
    let path_input = ["user", "input", "path", "req.", "argv", "param"]
        .iter()
        .any(|needle| trimmed.contains(needle));
    if !path_input {
        return false;
    }
    match language {
        "rust" => {
            trimmed.contains("File::open(")
                || trimmed.contains("fs::read(")
                || trimmed.contains("fs::read_to_string(")
        }
        "python" => trimmed.contains("open(") || trimmed.contains("Path("),
        "javascript" | "jsx" | "mjs" | "cjs" | "typescript" | "tsx" => {
            trimmed.contains("readFile(") || trimmed.contains("readFileSync(")
        }
        _ => false,
    }
}

fn is_raw_sql_smell(trimmed: &str) -> bool {
    let lowered = trimmed.to_ascii_lowercase();
    let has_sql = lowered.contains("select ")
        || lowered.contains("insert into")
        || lowered.contains("update ")
        || lowered.contains("delete from");
    has_sql
        && (trimmed.contains("format!(")
            || trimmed.contains("f\"")
            || trimmed.contains("f'")
            || trimmed.contains("${")
            || trimmed.contains("%s")
            || trimmed.contains(" + "))
}

fn is_unsafe_deserialize_smell(trimmed: &str) -> bool {
    trimmed.contains("pickle.load(")
        || trimmed.contains("pickle.loads(")
        || (trimmed.contains("yaml.load(") && !trimmed.contains("safe_load"))
}
