use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use time::OffsetDateTime;

use crate::model::{
    QualityProjectDeltaReport, QualityProjectSnapshotKind, QualityProjectSnapshotReport,
};

pub(super) const SNAPSHOT_REPORT_NAME: &str = "snapshot.report.json";
const BASELINE_SUMMARY_NAME: &str = "baseline-summary.json";

pub(super) fn persist_snapshot_artifacts(
    root: &Path,
    snapshot: &QualityProjectSnapshotReport,
) -> Result<()> {
    fs::create_dir_all(root)
        .with_context(|| format!("failed to create snapshot directory `{}`", root.display()))?;
    write_json(root.join(SNAPSHOT_REPORT_NAME), snapshot)?;
    write_json(
        root.join("violations.by_violation_count.json"),
        &snapshot.rule_violations_by_violation_count,
    )?;
    write_json(
        root.join("violations.by_size_bytes.json"),
        &snapshot.rule_violations_by_size_bytes,
    )?;
    write_json(
        root.join("violations.by_non_empty_lines.json"),
        &snapshot.rule_violations_by_non_empty_lines,
    )?;
    write_json(
        root.join("violations.by_metric_graph_edge_out_count.json"),
        &snapshot.rule_violations_by_metric_graph_edge_out_count,
    )?;
    write_json(
        root.join("violations.by_metric_max_cognitive_complexity.json"),
        &snapshot.rule_violations_by_metric_max_cognitive_complexity,
    )?;
    write_json(
        root.join("violations.by_metric_duplicate_density_bps.json"),
        &snapshot.rule_violations_by_metric_duplicate_density_bps,
    )?;
    write_json(root.join("hotspots.file.json"), &snapshot.file_hotspots)?;
    write_json(
        root.join("hotspots.directory.json"),
        &snapshot.directory_hotspots,
    )?;
    write_json(root.join("hotspots.module.json"), &snapshot.module_hotspots)?;
    fs::write(root.join("notes.md"), snapshot_notes(snapshot)).with_context(|| {
        format!(
            "failed to write notes `{}`",
            root.join("notes.md").display()
        )
    })?;
    Ok(())
}

pub(super) fn persist_delta_artifact(
    project_root: &Path,
    output_root: Option<&str>,
    wave_id: Option<&str>,
    delta: &QualityProjectDeltaReport,
) -> Result<Option<PathBuf>> {
    let Some(wave_id) = wave_id else {
        return Ok(None);
    };
    let path = resolve_quality_artifact_root(project_root, output_root)
        .join("quality-waves")
        .join(safe_segment(wave_id))
        .join("delta")
        .join(format!("{}.json", run_stamp()?));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create delta directory `{}`", parent.display()))?;
    }
    write_json(&path, delta)?;
    Ok(Some(path))
}

pub(super) fn persist_self_baseline_artifacts(
    project_root: &Path,
    snapshot: &QualityProjectSnapshotReport,
) -> Result<PathBuf> {
    let baseline_root = project_root.join("baseline/quality/self");
    fs::create_dir_all(&baseline_root).with_context(|| {
        format!(
            "failed to create self baseline directory `{}`",
            baseline_root.display()
        )
    })?;
    let baseline_summary = baseline_root.join(BASELINE_SUMMARY_NAME);
    write_json(&baseline_summary, snapshot)?;
    write_json(
        baseline_root.join("file-hotspots.json"),
        &snapshot.file_hotspots,
    )?;
    write_json(
        baseline_root.join("directory-hotspots.json"),
        &snapshot.directory_hotspots,
    )?;
    write_json(
        baseline_root.join("module-hotspots.json"),
        &snapshot.module_hotspots,
    )?;
    Ok(baseline_summary)
}

pub(super) fn load_self_baseline_snapshot(
    project_root: &Path,
) -> Result<QualityProjectSnapshotReport> {
    load_snapshot_from_path(
        project_root
            .join("baseline/quality/self")
            .join(BASELINE_SUMMARY_NAME),
    )
}

pub(super) fn load_latest_wave_before_snapshot(
    project_root: &Path,
    wave_id: &str,
    output_root: Option<&str>,
) -> Result<QualityProjectSnapshotReport> {
    let before_root = resolve_quality_artifact_root(project_root, output_root)
        .join("quality-waves")
        .join(safe_segment(wave_id))
        .join("before");
    let latest = latest_snapshot_path(&before_root)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no before snapshot found for wave `{wave_id}` under `{}`",
            before_root.display()
        )
    })?;
    load_snapshot_from_path(latest)
}

pub(super) fn snapshot_output_root(
    project_root: &Path,
    output_root: Option<&str>,
    snapshot_kind: QualityProjectSnapshotKind,
    wave_id: Option<&str>,
) -> Result<Option<PathBuf>> {
    let stamp = run_stamp()?;
    let artifact_root = resolve_quality_artifact_root(project_root, output_root);
    let root = match (snapshot_kind, wave_id) {
        (QualityProjectSnapshotKind::Before, Some(wave)) => artifact_root
            .join("quality-waves")
            .join(safe_segment(wave))
            .join("before")
            .join(stamp),
        (QualityProjectSnapshotKind::After, Some(wave)) => artifact_root
            .join("quality-waves")
            .join(safe_segment(wave))
            .join("after")
            .join(stamp),
        (kind, _) => artifact_root
            .join("quality-snapshots")
            .join(snapshot_kind_label(kind))
            .join(stamp),
    };
    Ok(Some(root))
}

pub(super) fn snapshot_exclude_paths(
    project_root: &Path,
    output_root: Option<&str>,
) -> Vec<String> {
    let mut excludes = Vec::new();
    if let Some(path) = output_root {
        let resolved = resolve_quality_artifact_root(project_root, Some(path));
        if let Ok(relative) = resolved.strip_prefix(project_root) {
            let normalized = relative.to_string_lossy().replace('\\', "/");
            if !normalized.is_empty() && normalized != "." {
                excludes.push(normalized);
            }
        }
    }
    excludes
}

fn load_snapshot_from_path(path: PathBuf) -> Result<QualityProjectSnapshotReport> {
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read snapshot `{}`", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse snapshot `{}`", path.display()))
}

fn latest_snapshot_path(root: &Path) -> Result<Option<PathBuf>> {
    if !root.exists() {
        return Ok(None);
    }
    let mut entries = fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().ok().is_some_and(|kind| kind.is_dir()))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries
        .pop()
        .map(|entry| entry.path().join(SNAPSHOT_REPORT_NAME))
        .filter(|path| path.exists()))
}

fn resolve_quality_artifact_root(project_root: &Path, output_root: Option<&str>) -> PathBuf {
    match output_root.map(str::trim).filter(|value| !value.is_empty()) {
        Some(path) => {
            let candidate = Path::new(path);
            if candidate.is_absolute() {
                candidate.to_path_buf()
            } else {
                project_root.join(candidate)
            }
        }
        None => project_root.join(".codex"),
    }
}

fn snapshot_kind_label(kind: QualityProjectSnapshotKind) -> &'static str {
    match kind {
        QualityProjectSnapshotKind::AdHoc => "ad_hoc",
        QualityProjectSnapshotKind::Before => "before",
        QualityProjectSnapshotKind::After => "after",
        QualityProjectSnapshotKind::Baseline => "baseline",
    }
}

fn snapshot_notes(snapshot: &QualityProjectSnapshotReport) -> String {
    format!(
        concat!(
            "snapshot_kind={}\n",
            "generated_at_utc={}\n",
            "quality_status_before_refresh={}\n",
            "quality_status_after_refresh={}\n",
            "refresh_performed={}\n",
            "evaluated_files={}\n",
            "violating_files={}\n",
            "total_violations={}\n",
            "suppressed_violations={}\n",
            "total_non_empty_lines={}\n",
            "total_size_bytes={}\n"
        ),
        snapshot_kind_label(snapshot.snapshot_kind),
        snapshot.generated_at_utc,
        snapshot.quality_status_before_refresh.as_str(),
        snapshot.quality_status_after_refresh.as_str(),
        snapshot.refresh_performed,
        snapshot.evaluated_files,
        snapshot.violating_files,
        snapshot.total_violations,
        snapshot.suppressed_violations,
        snapshot.total_non_empty_lines,
        snapshot.total_size_bytes
    )
}

fn write_json(path: impl AsRef<Path>, value: &impl serde::Serialize) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory `{}`", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{serialized}\n"))
        .with_context(|| format!("failed to write json file `{}`", path.display()))?;
    Ok(())
}

fn safe_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

fn run_stamp() -> Result<String> {
    Ok(
        OffsetDateTime::now_utc().format(&time::macros::format_description!(
            "[year][month][day]T[hour][minute][second]Z"
        ))?,
    )
}
