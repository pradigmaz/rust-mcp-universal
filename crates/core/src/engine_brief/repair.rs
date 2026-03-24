use anyhow::Result;

use crate::engine::Engine;
use crate::engine::compatibility;
use crate::model::{QualityStatus, WorkspaceBrief, WorkspaceQualitySummary, WorkspaceRepairHint};

pub(super) fn read_only_repair_hint(engine: &Engine) -> Result<Option<WorkspaceRepairHint>> {
    if !engine.db_path.exists() {
        return Ok(None);
    }

    let conn = engine.open_db_read_only()?;
    let files = super::count_files(&conn)?;
    if files == 0 {
        return Ok(None);
    }

    if super::uses_legacy_default_scope(&conn, engine)? {
        return Ok(Some(WorkspaceRepairHint {
            action: "reindex".to_string(),
            reason: "legacy_default_scope".to_string(),
            message: "index was built before current scoped defaults; run a full reindex to repair the brief surface".to_string(),
        }));
    }

    let compatibility = compatibility::evaluate_index_compatibility(&conn)?;
    Ok(compatibility.reason().map(|reason| WorkspaceRepairHint {
        action: "reindex".to_string(),
        reason: reason.to_string(),
        message: format!(
            "index metadata is incompatible with the current binary; run a full reindex to repair the brief surface ({reason})"
        ),
    }))
}

pub(super) fn build_repair_brief(
    engine: &Engine,
    repair_hint: WorkspaceRepairHint,
) -> Result<WorkspaceBrief> {
    let status = engine.index_status()?;
    let mut recommendations = super::make_recommendations(&status);
    recommendations.push(repair_hint.message.clone());

    Ok(WorkspaceBrief {
        auto_indexed: false,
        index_status: status,
        languages: super::load_top_languages_for_brief(engine, 8).unwrap_or_default(),
        top_symbols: super::load_top_symbols_for_brief(engine, 12).unwrap_or_default(),
        quality_summary: empty_quality_summary(),
        recommendations,
        repair_hint: Some(repair_hint),
    })
}

fn empty_quality_summary() -> WorkspaceQualitySummary {
    WorkspaceQualitySummary {
        ruleset_id: crate::quality::QUALITY_RULESET_ID.to_string(),
        status: QualityStatus::Unavailable,
        evaluated_files: 0,
        violating_files: 0,
        total_violations: 0,
        suppressed_violations: 0,
        top_rules: Vec::new(),
        top_metrics: Vec::new(),
        severity_breakdown: Vec::new(),
        category_breakdown: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::engine::test_index_path_with_options_impl;
    use crate::model::IndexingOptions;

    fn temp_project_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    #[test]
    fn read_only_workspace_brief_returns_repair_hint_for_incompatible_index() -> anyhow::Result<()>
    {
        let project_dir = temp_project_dir("rmu-brief-repair-hint");
        fs::create_dir_all(project_dir.join("src"))?;
        fs::write(project_dir.join("src/lib.rs"), "pub fn brief_repair() {}\n")?;

        let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
        let _ = test_index_path_with_options_impl(&engine, &IndexingOptions::default())?;
        let conn = engine.open_db()?;
        conn.execute("DELETE FROM meta WHERE key = 'index_format_version'", [])?;

        let brief = engine.workspace_brief_with_policy(false)?;
        assert!(!brief.auto_indexed);
        assert!(brief.repair_hint.is_some());
        assert_eq!(
            brief.repair_hint.as_ref().map(|hint| hint.action.as_str()),
            Some("reindex")
        );
        assert!(
            brief
                .recommendations
                .iter()
                .any(|item| item.contains("reindex"))
        );

        let _ = fs::remove_dir_all(project_dir);
        Ok(())
    }
}
