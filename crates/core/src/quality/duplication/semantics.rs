use crate::quality::duplication::artifact::DuplicationSignalRole;

use super::surface::{is_modelish_path, is_wrapper_surface_path, paths_all};

#[derive(Debug, Clone)]
pub(crate) struct SignalClassification {
    pub(crate) role: DuplicationSignalRole,
    pub(crate) reason: Option<String>,
}

pub(crate) fn classify_signal_role(
    language: &str,
    member_paths: &[String],
    signature_tokens: &[String],
) -> SignalClassification {
    if signature_tokens.is_empty() {
        return primary();
    }
    if is_jsx_wrapper_shell(language, member_paths, signature_tokens) {
        return boilerplate("jsx_wrapper_shell");
    }
    if is_rust_macro_shell(language, signature_tokens) {
        return boilerplate("rust_macro_shell");
    }
    if is_java_annotation_wrapper(language, member_paths, signature_tokens) {
        return boilerplate("java_annotation_wrapper");
    }
    if is_python_model_boilerplate(language, member_paths, signature_tokens) {
        return boilerplate("python_model_boilerplate");
    }
    if is_generic_modelish_surface(member_paths, signature_tokens) {
        return boilerplate("model_or_schema_boilerplate");
    }
    primary()
}

fn primary() -> SignalClassification {
    SignalClassification {
        role: DuplicationSignalRole::Primary,
        reason: None,
    }
}

fn boilerplate(reason: &str) -> SignalClassification {
    SignalClassification {
        role: DuplicationSignalRole::Boilerplate,
        reason: Some(reason.to_string()),
    }
}

fn is_python_model_boilerplate(
    language: &str,
    member_paths: &[String],
    signature_tokens: &[String],
) -> bool {
    let score = python_model_signature_score(signature_tokens);
    matches!(language, "python")
        && score >= 6
        && (paths_all(member_paths, is_modelish_path) || score >= 8)
        && control_flow_count(signature_tokens) == 0
        && declaration_heavy(signature_tokens)
}

fn is_java_annotation_wrapper(
    language: &str,
    member_paths: &[String],
    signature_tokens: &[String],
) -> bool {
    let score = java_wrapper_signature_score(signature_tokens);
    matches!(language, "java")
        && score >= 8
        && (paths_all(member_paths, is_java_wrapper_path) || score >= 10)
        && control_flow_count(signature_tokens) == 0
        && declaration_heavy(signature_tokens)
}

fn is_jsx_wrapper_shell(
    language: &str,
    member_paths: &[String],
    signature_tokens: &[String],
) -> bool {
    matches!(
        language,
        "tsx" | "jsx" | "typescript" | "javascript" | "mjs" | "cjs"
    ) && (paths_all(member_paths, is_wrapper_surface_path)
        || jsx_wrapper_signature_score(signature_tokens) >= 5)
        && jsx_tag_count(signature_tokens) >= 4
        && count_tokens(
            signature_tokens,
            &[
                "children",
                "classname",
                "variant",
                "props",
                "provider",
                "layout",
            ],
        ) >= 1
        && control_flow_count(signature_tokens) <= 1
        && declaration_heavy(signature_tokens)
}

fn is_rust_macro_shell(language: &str, signature_tokens: &[String]) -> bool {
    matches!(language, "rust")
        && count_tokens(signature_tokens, &["$attr", "$macro", "$lifetime"]) >= 2
        && control_flow_count(signature_tokens) == 0
        && declaration_heavy(signature_tokens)
}

fn is_generic_modelish_surface(member_paths: &[String], signature_tokens: &[String]) -> bool {
    paths_all(member_paths, is_modelish_path)
        && declaration_heavy(signature_tokens)
        && control_flow_count(signature_tokens) <= 1
}

fn declaration_heavy(tokens: &[String]) -> bool {
    let structural = count_tokens(
        tokens,
        &[
            "$id",
            "$lit",
            "$num",
            "$attr",
            "$macro",
            "$lifetime",
            "{",
            "}",
            "(",
            ")",
            "[",
            "]",
            "<",
            ">",
            "</",
            "/>",
            ",",
            ":",
            "=",
            "=>",
            "->",
            "::",
            ";",
            "class",
            "interface",
            "record",
            "struct",
            "enum",
            "def",
            "fn",
            "pub",
            "const",
            "let",
            "private",
            "public",
            "protected",
            "static",
            "final",
            "export",
            "type",
        ],
    );
    structural * 100 / tokens.len().max(1) >= 70
}

fn jsx_tag_count(tokens: &[String]) -> usize {
    count_tokens(tokens, &["<", "</", "/>"])
}

fn control_flow_count(tokens: &[String]) -> usize {
    count_tokens(
        tokens,
        &[
            "if", "else", "match", "for", "while", "loop", "switch", "case", "try", "except",
            "catch",
        ],
    )
}

fn count_tokens(tokens: &[String], values: &[&str]) -> usize {
    tokens
        .iter()
        .filter(|token| values.contains(&token.as_str()))
        .count()
}

fn python_model_signature_score(signature_tokens: &[String]) -> usize {
    count_tokens(
        signature_tokens,
        &[
            "$attr",
            "class",
            "dataclass",
            "field",
            "basemodel",
            "modelconfig",
            ":",
        ],
    )
}

fn java_wrapper_signature_score(signature_tokens: &[String]) -> usize {
    count_tokens(
        signature_tokens,
        &[
            "$attr",
            "class",
            "record",
            "private",
            "public",
            ";",
            "configuration",
            "configurationproperties",
        ],
    )
}

fn jsx_wrapper_signature_score(signature_tokens: &[String]) -> usize {
    jsx_tag_count(signature_tokens)
        + count_tokens(
            signature_tokens,
            &[
                "children",
                "classname",
                "variant",
                "props",
                "provider",
                "layout",
            ],
        )
}

fn is_java_wrapper_path(lowered: &str) -> bool {
    lowered.contains("config")
        || lowered.contains("dto")
        || lowered.contains("entity")
        || lowered.contains("view")
}

#[cfg(test)]
#[path = "semantics/tests.rs"]
mod tests;
