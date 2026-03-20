use super::{CURRENT_ANN_VERSION, CURRENT_INDEX_FORMAT_VERSION};
use crate::vector_rank::{semantic_model_name, vector_dim};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IndexCompatibilityDecision {
    Compatible,
    ReindexRequired { reason: String },
}

impl IndexCompatibilityDecision {
    #[cfg(test)]
    pub(crate) fn is_reindex_required(&self) -> bool {
        matches!(self, Self::ReindexRequired { .. })
    }

    pub(crate) fn reason(&self) -> Option<&str> {
        match self {
            Self::Compatible => None,
            Self::ReindexRequired { reason } => Some(reason.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExpectedIndexMeta {
    pub(crate) index_format_version: u32,
    pub(crate) embedding_model_id: String,
    pub(crate) embedding_dim: u32,
    pub(crate) ann_version: u32,
}

pub(crate) fn expected_index_meta() -> ExpectedIndexMeta {
    ExpectedIndexMeta {
        index_format_version: CURRENT_INDEX_FORMAT_VERSION,
        embedding_model_id: semantic_model_name(),
        embedding_dim: u32::try_from(vector_dim()).unwrap_or(u32::MAX),
        ann_version: CURRENT_ANN_VERSION,
    }
}
