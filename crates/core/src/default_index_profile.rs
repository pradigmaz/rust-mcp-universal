use std::path::Path;

use crate::model::{IndexProfile, IndexingOptions};

const RUST_ROOT_DIR_MARKERS: &[&str] = &["src", "crates", "tests", "examples", "benches", ".cargo"];
const RUST_ROOT_FILE_MARKERS: &[&str] = &["rust-toolchain", "rust-toolchain.toml"];

pub(crate) fn resolve_default_index_profile(
    project_root: &Path,
    requested_profile: Option<IndexProfile>,
) -> Option<IndexProfile> {
    requested_profile.or_else(|| {
        if is_rust_workspace(project_root) {
            Some(IndexProfile::RustMonorepo)
        } else {
            Some(IndexProfile::Mixed)
        }
    })
}

pub(crate) fn resolve_indexing_options(
    project_root: &Path,
    options: &IndexingOptions,
) -> IndexingOptions {
    let mut resolved = options.clone();
    resolved.profile = resolve_default_index_profile(project_root, options.profile);
    resolved
}

fn is_rust_workspace(project_root: &Path) -> bool {
    project_root.join("Cargo.toml").is_file()
        && (RUST_ROOT_DIR_MARKERS
            .iter()
            .any(|marker| project_root.join(marker).is_dir())
            || RUST_ROOT_FILE_MARKERS
                .iter()
                .any(|marker| project_root.join(marker).is_file()))
}

#[cfg(test)]
#[path = "default_index_profile_tests.rs"]
mod tests;
