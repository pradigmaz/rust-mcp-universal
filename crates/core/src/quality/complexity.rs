use crate::quality::HotspotFacts;

#[path = "complexity/common.rs"]
mod common;
#[path = "complexity/java.rs"]
mod java;
#[path = "complexity/javascript.rs"]
mod javascript;
#[path = "complexity/python.rs"]
mod python;
#[path = "complexity/rust.rs"]
mod rust;

pub(crate) fn analyze_complexity(rel_path: &str, language: &str, source: &str) -> HotspotFacts {
    match language {
        "javascript" | "jsx" | "mjs" | "cjs" | "typescript" | "tsx" => {
            javascript::analyze(rel_path, language, source)
        }
        "python" => python::analyze(source),
        "rust" => rust::analyze(source),
        "java" => java::analyze(source),
        _ => HotspotFacts::default(),
    }
}
