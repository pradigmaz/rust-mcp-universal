use crate::model::FindingConfidence;

use super::DeadCodeFacts;

pub(crate) fn analyze_dead_code(rel_path: &str, language: &str, full_text: &str) -> DeadCodeFacts {
    if !supports_dead_code_scan(rel_path, language) || has_runtime_registry_markers(full_text) {
        return DeadCodeFacts::default();
    }

    let mut exported_symbol_count = 0_i64;
    let mut first_location = None;
    for (idx, line) in full_text.lines().enumerate() {
        let trimmed = line.trim_start();
        if is_export_line(language, trimmed) {
            exported_symbol_count += 1;
            first_location
                .get_or_insert_with(|| super::location::line_location(idx + 1, line.len()));
        }
    }

    if exported_symbol_count == 0 {
        return DeadCodeFacts::default();
    }

    DeadCodeFacts {
        exported_symbol_count,
        candidate: true,
        location: first_location,
        confidence: Some(if exported_symbol_count <= 2 {
            FindingConfidence::Medium
        } else {
            FindingConfidence::Low
        }),
        noise_reason: None,
    }
}

fn supports_dead_code_scan(rel_path: &str, language: &str) -> bool {
    if matches!(language, "markdown" | "text" | "json" | "yaml" | "toml") {
        return false;
    }
    let path = rel_path.to_ascii_lowercase();
    !path.contains("/tests/")
        && !path.contains("/examples/")
        && !path.contains("/benches/")
        && !path.contains("/target/")
        && !path.contains("/generated/")
        && !path.ends_with("/main.rs")
        && !path.ends_with("/lib.rs")
        && !path.ends_with("/mod.rs")
        && !path.ends_with("/build.rs")
}

fn is_export_line(language: &str, trimmed: &str) -> bool {
    match language {
        "rust" => {
            trimmed.starts_with("pub fn ")
                || trimmed.starts_with("pub(crate) fn ")
                || trimmed.starts_with("pub struct ")
                || trimmed.starts_with("pub enum ")
                || trimmed.starts_with("pub trait ")
                || trimmed.starts_with("pub const ")
                || trimmed.starts_with("pub static ")
        }
        "python" => trimmed.starts_with("def ") && !trimmed.starts_with("def _"),
        "javascript" | "jsx" | "mjs" | "cjs" | "typescript" | "tsx" => {
            trimmed.starts_with("export function ")
                || trimmed.starts_with("export const ")
                || trimmed.starts_with("export class ")
                || trimmed.starts_with("export default ")
        }
        _ => false,
    }
}

fn has_runtime_registry_markers(full_text: &str) -> bool {
    [
        "inventory::submit!",
        "linkme::distributed_slice",
        "#[no_mangle]",
        "proc_macro",
        "register_plugin",
        "register_handler",
        "plugin_registry",
    ]
    .iter()
    .any(|marker| full_text.contains(marker))
}
