use super::{AdapterMatch, ConstraintAdapterInput, strong_match};

pub(super) fn typescript_adapter(input: &ConstraintAdapterInput<'_>) -> Option<AdapterMatch> {
    if !matches!(input.language, "typescript" | "javascript") {
        return None;
    }
    if input.lowered_line.contains("createindex(") {
        return Some(strong_match("index_constraint", "index_declaration", 0.9));
    }
    if input.lowered_line.contains("unique: true") || input.lowered_line.contains("@unique") {
        return Some(strong_match("model_constraint", "model_declaration", 0.86));
    }
    if input.lowered_line.contains("references:") || input.lowered_line.contains("foreignkey") {
        return Some(strong_match("model_constraint", "model_declaration", 0.84));
    }
    None
}
