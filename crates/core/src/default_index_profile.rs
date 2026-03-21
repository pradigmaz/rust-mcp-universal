use std::path::Path;

use crate::model::{IndexProfile, IndexingOptions};

const RUST_ROOT_DIR_MARKERS: &[&str] = &["src", "crates", "tests", "examples", "benches", ".cargo"];
const RUST_ROOT_FILE_MARKERS: &[&str] = &["rust-toolchain", "rust-toolchain.toml"];

pub(crate) fn resolve_default_index_profile(
    project_root: &Path,
    requested_profile: Option<IndexProfile>,
) -> Option<IndexProfile> {
    requested_profile
        .or_else(|| is_rust_workspace(project_root).then_some(IndexProfile::RustMonorepo))
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
mod tests {
    use super::{resolve_default_index_profile, resolve_indexing_options};
    use crate::Engine;
    use crate::model::{IndexProfile, IndexingOptions};
    use rusqlite::Connection;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    fn write_project_file(root: &Path, relative: &str, contents: &str) -> anyhow::Result<()> {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
        Ok(())
    }

    fn indexed_paths(engine: &Engine) -> anyhow::Result<Vec<String>> {
        let conn = Connection::open(&engine.db_path)?;
        let mut stmt = conn.prepare("SELECT path FROM files ORDER BY path ASC")?;
        Ok(stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    #[test]
    fn resolver_detects_rust_workspace_from_root_markers() -> anyhow::Result<()> {
        let root = temp_dir("rmu-rust-default-profile");
        fs::create_dir_all(root.join("crates"))?;
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/core\"]\n",
        )?;

        assert_eq!(
            resolve_default_index_profile(&root, None),
            Some(IndexProfile::RustMonorepo)
        );

        let _ = fs::remove_dir_all(root);
        Ok(())
    }

    #[test]
    fn resolver_preserves_explicit_profile() -> anyhow::Result<()> {
        let root = temp_dir("rmu-rust-default-profile-explicit");
        fs::create_dir_all(root.join("src"))?;
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )?;

        assert_eq!(
            resolve_default_index_profile(&root, Some(IndexProfile::Mixed)),
            Some(IndexProfile::Mixed)
        );

        let _ = fs::remove_dir_all(root);
        Ok(())
    }

    #[test]
    fn resolver_leaves_non_rust_repo_unscoped() -> anyhow::Result<()> {
        let root = temp_dir("rmu-non-rust-default-profile");
        fs::create_dir_all(root.join("src"))?;
        fs::write(root.join("package.json"), "{ \"name\": \"demo\" }\n")?;

        assert_eq!(resolve_default_index_profile(&root, None), None);

        let _ = fs::remove_dir_all(root);
        Ok(())
    }

    #[test]
    fn resolve_indexing_options_only_changes_profile_field() -> anyhow::Result<()> {
        let root = temp_dir("rmu-resolve-indexing-options");
        fs::create_dir_all(root.join("src"))?;
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )?;
        let options = IndexingOptions {
            include_paths: vec!["src".to_string()],
            exclude_paths: vec!["target/**".to_string()],
            reindex: true,
            ..IndexingOptions::default()
        };

        let resolved = resolve_indexing_options(&root, &options);
        assert_eq!(resolved.profile, Some(IndexProfile::RustMonorepo));
        assert_eq!(resolved.include_paths, options.include_paths);
        assert_eq!(resolved.exclude_paths, options.exclude_paths);
        assert_eq!(resolved.reindex, options.reindex);

        let _ = fs::remove_dir_all(root);
        Ok(())
    }

    #[test]
    fn engine_index_path_defaults_to_rust_monorepo() -> anyhow::Result<()> {
        let root = temp_dir("rmu-engine-default-rust-monorepo");
        write_project_file(
            &root,
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )?;
        write_project_file(&root, "src/lib.rs", "pub fn rust_only_default() {}\n")?;
        write_project_file(
            &root,
            "apps/desktop/Cargo.toml",
            "[package]\nname = \"desktop\"\nversion = \"0.1.0\"\n",
        )?;
        write_project_file(
            &root,
            "apps/desktop/src/main.rs",
            "fn main() { println!(\"desktop\"); }\n",
        )?;
        write_project_file(
            &root,
            "tools/content_bake/Cargo.toml",
            "[package]\nname = \"content_bake\"\nversion = \"0.1.0\"\n",
        )?;
        write_project_file(
            &root,
            "tools/content_bake/src/main.rs",
            "fn main() { println!(\"bake\"); }\n",
        )?;
        write_project_file(&root, "docs/guide.md", "should_not_be_indexed\n")?;
        write_project_file(&root, "packs/core/item.ron", "(name: \"ignored\")\n")?;

        let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
        engine.index_path()?;

        assert_eq!(
            indexed_paths(&engine)?,
            vec![
                "Cargo.toml".to_string(),
                "apps/desktop/Cargo.toml".to_string(),
                "apps/desktop/src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "tools/content_bake/Cargo.toml".to_string(),
                "tools/content_bake/src/main.rs".to_string(),
            ]
        );

        let preview = engine.scope_preview_with_options(&IndexingOptions::default())?;
        assert_eq!(preview.profile, Some(IndexProfile::RustMonorepo));
        assert_eq!(
            preview.candidate_paths,
            vec![
                "Cargo.toml",
                "apps/desktop/Cargo.toml",
                "apps/desktop/src/main.rs",
                "src/lib.rs",
                "tools/content_bake/Cargo.toml",
                "tools/content_bake/src/main.rs",
            ]
        );
        assert!(
            preview
                .excluded_by_scope_paths
                .contains(&"docs/guide.md".to_string())
        );
        assert!(
            preview
                .excluded_by_scope_paths
                .contains(&"packs/core/item.ron".to_string())
        );

        let _ = fs::remove_dir_all(root);
        Ok(())
    }

    #[test]
    fn explicit_mixed_profile_overrides_rust_default() -> anyhow::Result<()> {
        let root = temp_dir("rmu-engine-explicit-mixed");
        write_project_file(
            &root,
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )?;
        write_project_file(&root, "src/lib.rs", "pub fn mixed_override_symbol() {}\n")?;
        write_project_file(&root, "scripts/tool.py", "print('mixed override')\n")?;

        let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
        engine.index_path_with_options(&IndexingOptions {
            profile: Some(IndexProfile::Mixed),
            reindex: true,
            ..IndexingOptions::default()
        })?;

        assert_eq!(
            indexed_paths(&engine)?,
            vec![
                "Cargo.toml".to_string(),
                "scripts/tool.py".to_string(),
                "src/lib.rs".to_string(),
            ]
        );

        let _ = fs::remove_dir_all(root);
        Ok(())
    }

    #[test]
    fn ensure_index_ready_reindexes_legacy_unscoped_rust_index() -> anyhow::Result<()> {
        let root = temp_dir("rmu-engine-legacy-rust-reindex");
        write_project_file(&root, "src/lib.rs", "pub fn legacy_default_scope() {}\n")?;
        write_project_file(&root, "docs/guide.md", "legacy docs should be pruned\n")?;

        let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
        engine.index_path()?;
        assert_eq!(
            indexed_paths(&engine)?,
            vec!["docs/guide.md".to_string(), "src/lib.rs".to_string()]
        );

        write_project_file(
            &root,
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )?;

        assert!(engine.ensure_index_ready_with_policy(true)?);
        assert_eq!(
            indexed_paths(&engine)?,
            vec!["Cargo.toml".to_string(), "src/lib.rs".to_string()]
        );

        let _ = fs::remove_dir_all(root);
        Ok(())
    }
}
