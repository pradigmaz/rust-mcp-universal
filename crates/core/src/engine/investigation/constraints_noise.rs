use super::{AdapterMatch, ConstraintAdapterInput, is_schema_like_path, weak_match};

pub(super) fn generic_weak_fallback_adapter(
    input: &ConstraintAdapterInput<'_>,
) -> Option<AdapterMatch> {
    if !supports_generic_weak_fallback(input) {
        return None;
    }
    if looks_like_runtime_guard(input) {
        return Some(weak_match("runtime_guard", "runtime_guard_code", 0.5));
    }
    if is_schema_like_path(&input.lowered_path) && looks_like_schema_hint(&input.lowered_line) {
        return Some(weak_match("ddl_like_hint", "schema_hint", 0.45));
    }
    None
}

pub(super) fn should_ignore_constraint_line(language: &str, normalized_line: &str) -> bool {
    let trimmed = normalized_line.trim_start();
    let lowered = trimmed.to_ascii_lowercase();
    match language {
        "python" => {
            trimmed.starts_with('#')
                || trimmed.starts_with("\"\"\"")
                || trimmed.starts_with("'''")
                || lowered.starts_with("revision id:")
                || lowered.starts_with("revision:")
                || lowered.starts_with("revises:")
                || lowered.starts_with("down_revision:")
                || lowered.starts_with("create date:")
        }
        "rust" | "typescript" | "javascript" => {
            trimmed.starts_with("//")
                || trimmed.starts_with("/*")
                || trimmed.starts_with('*')
                || trimmed.starts_with("*/")
        }
        "sql" => trimmed.starts_with("--"),
        _ => false,
    }
}

fn supports_generic_weak_fallback(input: &ConstraintAdapterInput<'_>) -> bool {
    matches!(
        input.language,
        "rust" | "python" | "typescript" | "javascript" | "sql" | "prisma"
    )
}

fn looks_like_runtime_guard(input: &ConstraintAdapterInput<'_>) -> bool {
    let line = input.lowered_line.trim();
    if line.starts_with("raise httpexception(") {
        return false;
    }
    if !backendish_path(&input.lowered_path) {
        return false;
    }
    if line.starts_with("assert ") {
        return true;
    }
    if line.starts_with("def validate_")
        || line.starts_with("async def validate_")
        || line.starts_with("fn validate_")
    {
        return true;
    }
    let looks_like_guard = line.starts_with("raise ")
        || line.starts_with("if not ")
        || line.starts_with("guard ")
        || line.contains(" ensure_")
        || line.starts_with("ensure_")
        || line.contains(".ensure(");
    if !looks_like_guard {
        return false;
    }
    if input.lowered_path.contains("validator") {
        return true;
    }
    has_constraint_guard_signal(line)
}

fn has_constraint_guard_signal(line: &str) -> bool {
    [
        "validate",
        "invalid",
        "constraint",
        "unique",
        "duplicate",
        "conflict",
        "required",
        "missing",
        "exists",
        "deadline",
        "slot",
        "limit",
        "max_",
        "min_",
        "allowed",
        "forbid",
    ]
    .iter()
    .any(|token| line.contains(token))
}

fn looks_like_schema_hint(line: &str) -> bool {
    line.contains("create index")
        || line.contains("drop index")
        || line.contains("create_index(")
        || line.contains("drop_index(")
        || line.contains("constraint(")
        || line.contains(" constraint ")
        || line.contains("foreign key")
        || line.contains(" references ")
}

fn backendish_path(lowered_path: &str) -> bool {
    !lowered_path.contains("/frontend/")
        && !lowered_path.starts_with("frontend/")
        && !lowered_path.ends_with(".tsx")
        && !lowered_path.ends_with(".jsx")
}
