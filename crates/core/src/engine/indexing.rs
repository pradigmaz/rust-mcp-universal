use anyhow::Result;

use super::{Engine, IndexSummary};
use crate::model::IndexingOptions;

#[path = "indexing/post.rs"]
mod post;
#[path = "indexing/run.rs"]
pub(crate) mod run;
#[path = "indexing/util.rs"]
mod util;

impl Engine {
    pub fn index_path(&self) -> Result<IndexSummary> {
        self.index_path_with_options(&IndexingOptions::default())
    }

    pub fn index_path_with_options(&self, options: &IndexingOptions) -> Result<IndexSummary> {
        run::index_path_with_options_impl(self, options)
    }
}
