use super::{AdapterMatch, ConstraintAdapterInput, is_migration_path, strong_match};

pub(super) fn sql_prisma_adapter(input: &ConstraintAdapterInput<'_>) -> Option<AdapterMatch> {
    if input.language == "sql" {
        if input.lowered_line.contains("create unique index")
            || input.lowered_line.contains("unique index")
        {
            return Some(strong_match("index_constraint", "index_declaration", 0.95));
        }
        if input.lowered_line.contains("create index") {
            return Some(strong_match("index_constraint", "index_declaration", 0.88));
        }
        if input.lowered_line.contains("add constraint")
            || input.lowered_line.contains("foreign key")
            || input.lowered_line.contains(" references ")
            || input.lowered_line.contains(" check ")
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
                    "schema_hint"
                },
                0.9,
            ));
        }
    }
    if input.language == "prisma" {
        if input.lowered_line.contains("@@index") {
            return Some(strong_match("index_constraint", "index_declaration", 0.9));
        }
        if input.lowered_line.contains("@@unique")
            || input.lowered_line.contains("@unique")
            || input.lowered_line.contains("@id")
            || input.lowered_line.contains("@relation")
        {
            return Some(strong_match("model_constraint", "model_declaration", 0.9));
        }
    }
    None
}
