#[path = "compatibility/evaluate.rs"]
mod evaluate;
#[path = "compatibility/meta_store.rs"]
mod meta_store;
#[path = "compatibility/reconcile.rs"]
mod reconcile;
#[path = "compatibility/types.rs"]
mod types;

#[cfg(test)]
#[path = "compatibility/tests.rs"]
mod tests;

pub(crate) const CURRENT_SCHEMA_VERSION: u32 = super::schema::CURRENT_SCHEMA_MIGRATION_VERSION;
pub(crate) const CURRENT_INDEX_FORMAT_VERSION: u32 = 1;
pub(crate) const CURRENT_ANN_VERSION: u32 = 1;
const LEGACY_INDEX_FORMAT_VERSION: u32 = 0;
const LEGACY_ANN_VERSION: u32 = 0;

const META_SCHEMA_VERSION: &str = "schema_version";
const META_INDEX_FORMAT_VERSION: &str = "index_format_version";
const META_EMBEDDING_MODEL_ID: &str = "embedding_model_id";
const META_EMBEDDING_DIM: &str = "embedding_dim";
const META_ANN_VERSION: &str = "ann_version";

pub(crate) use evaluate::evaluate_index_compatibility;
pub(crate) use reconcile::{
    ensure_schema_preflight, reconcile_schema_and_index_meta, write_index_identity_meta,
};
pub(crate) use types::{ExpectedIndexMeta, IndexCompatibilityDecision};

pub(crate) fn expected_index_meta() -> ExpectedIndexMeta {
    types::expected_index_meta()
}
