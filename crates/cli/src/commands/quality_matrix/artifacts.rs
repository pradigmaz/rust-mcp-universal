use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use super::RepoRunOutcome;

pub(super) fn create_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create directory `{}`", path.display()))?;
    Ok(())
}

pub(super) fn write_repo_artifacts(
    repo_output_root: &Path,
    outcome: &RepoRunOutcome,
) -> Result<()> {
    write_json_file(
        &repo_output_root.join("brief.before_refresh.json"),
        &outcome.brief_before_refresh,
    )?;
    write_json_file(
        &repo_output_root.join("brief.after_refresh.json"),
        &outcome.brief_after_refresh,
    )?;
    write_json_file(
        &repo_output_root.join("violations.by_violation_count.json"),
        &outcome.by_violation_count,
    )?;
    write_json_file(
        &repo_output_root.join("violations.by_size_bytes.json"),
        &outcome.by_size_bytes,
    )?;
    write_json_file(
        &repo_output_root.join("violations.by_non_empty_lines.json"),
        &outcome.by_non_empty_lines,
    )?;
    write_json_file(
        &repo_output_root.join("violations.by_metric_graph_edge_out_count.json"),
        &outcome.by_metric_graph_edge_out_count,
    )?;
    write_json_file(
        &repo_output_root.join("violations.by_metric_max_cognitive_complexity.json"),
        &outcome.by_metric_max_cognitive_complexity,
    )?;
    write_json_file(
        &repo_output_root.join("hotspots.file.json"),
        &outcome.file_hotspots,
    )?;
    write_json_file(
        &repo_output_root.join("hotspots.directory.json"),
        &outcome.directory_hotspots,
    )?;
    write_json_file(
        &repo_output_root.join("hotspots.module.json"),
        &outcome.module_hotspots,
    )?;
    fs::write(repo_output_root.join("notes.md"), &outcome.notes_markdown).with_context(|| {
        format!(
            "failed to write notes markdown `{}`",
            repo_output_root.join("notes.md").display()
        )
    })?;
    Ok(())
}

pub(super) fn write_json_file(path: &Path, value: &impl Serialize) -> Result<()> {
    let serialized = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{serialized}\n"))
        .with_context(|| format!("failed to write json file `{}`", path.display()))?;
    Ok(())
}

pub(super) fn run_stamp() -> Result<String> {
    Ok(
        time::OffsetDateTime::now_utc().format(&time::macros::format_description!(
            "[year][month][day]T[hour][minute][second]Z"
        ))?,
    )
}
