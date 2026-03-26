use super::{AdapterMatch, ConstraintAdapterInput, is_migration_path, strong_match};

pub(super) fn python_adapter(input: &ConstraintAdapterInput<'_>) -> Option<AdapterMatch> {
    if input.language != "python" {
        return None;
    }
    if input.lowered_line.contains("op.create_index(") {
        return Some(strong_match(
            "index_constraint",
            "index_declaration",
            if input.lowered_line.contains("unique") {
                0.95
            } else {
                0.9
            },
        ));
    }
    if input.lowered_line.contains("index(")
        && !input.lowered_line.contains("drop_index(")
        && (is_migration_path(input.path) || input.lowered_path.contains("/models/"))
    {
        return Some(strong_match("index_constraint", "index_declaration", 0.88));
    }
    if input.lowered_line.contains("op.create_unique_constraint")
        || input.lowered_line.contains("op.create_foreign_key")
        || input.lowered_line.contains("op.create_check_constraint")
        || input.lowered_line.contains("foreignkeyconstraint")
        || input.lowered_line.contains("checkconstraint")
    {
        return Some(strong_match(
            if is_migration_path(input.path) {
                "migration_constraint"
            } else {
                "model_constraint"
            },
            if is_migration_path(input.path) {
                "migration_declaration"
            } else {
                "model_declaration"
            },
            0.92,
        ));
    }
    if input.lowered_line.contains("uniqueconstraint")
        || input.lowered_line.contains("foreignkey(")
        || input.lowered_line.contains(" references ")
    {
        return Some(strong_match("model_constraint", "model_declaration", 0.9));
    }
    None
}
