use super::{AdapterMatch, ConstraintAdapterInput, weak_match};

pub(super) fn rust_adapter(input: &ConstraintAdapterInput<'_>) -> Option<AdapterMatch> {
    if input.language != "rust" {
        return None;
    }
    if input.lowered_line.contains("table!")
        || input.lowered_line.contains("joinable!")
        || input
            .lowered_line
            .contains("allow_tables_to_appear_in_same_query!")
    {
        return Some(weak_match("ddl_like_hint", "schema_hint", 0.6));
    }
    if input.lowered_line.contains("sqlx::query!(")
        || input.lowered_line.contains("sqlx::query_as!(")
        || input.lowered_line.contains("query!(")
        || input.lowered_line.contains("query_as!(")
    {
        return Some(weak_match("runtime_guard", "runtime_guard_code", 0.55));
    }
    None
}
