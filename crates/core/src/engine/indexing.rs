use anyhow::Result;

use super::{Engine, IndexSummary};
use crate::default_index_profile;
use crate::model::IndexProfile;
use crate::model::IndexingOptions;

#[path = "indexing/post.rs"]
mod post;
#[path = "indexing/run.rs"]
pub(crate) mod run;
#[path = "indexing/util.rs"]
mod util;

impl Engine {
    pub fn resolve_default_index_profile(
        &self,
        requested_profile: Option<IndexProfile>,
    ) -> Option<IndexProfile> {
        default_index_profile::resolve_default_index_profile(&self.project_root, requested_profile)
    }

    pub fn resolve_indexing_options(&self, options: &IndexingOptions) -> IndexingOptions {
        default_index_profile::resolve_indexing_options(&self.project_root, options)
    }

    pub fn index_path(&self) -> Result<IndexSummary> {
        self.index_path_with_options(&IndexingOptions::default())
    }

    pub fn index_path_with_options(&self, options: &IndexingOptions) -> Result<IndexSummary> {
        let resolved = self.resolve_indexing_options(options);
        run::index_path_with_options_impl(self, &resolved)
    }
}
