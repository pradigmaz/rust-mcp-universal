use anyhow::{Result, bail};
use rmu_core::{
    Engine, PrivacyMode, QualityProjectGateStatus, QualityProjectSnapshotCompareAgainst,
    QualityProjectSnapshotKind, QualityProjectSnapshotOptions, sanitize_path_text,
    sanitize_value_for_privacy,
};

use crate::output::{print_json, print_line};

pub(crate) struct QualitySnapshotArgs {
    pub(crate) snapshot_kind: String,
    pub(crate) wave_id: Option<String>,
    pub(crate) output_root: Option<std::path::PathBuf>,
    pub(crate) compare_against: String,
    pub(crate) auto_index: Option<bool>,
    pub(crate) persist_artifacts: Option<bool>,
    pub(crate) promote_self_baseline: bool,
    pub(crate) fail_on_regression: bool,
}

pub(crate) fn run(
    engine: &Engine,
    json: bool,
    privacy_mode: PrivacyMode,
    args: QualitySnapshotArgs,
) -> Result<()> {
    let snapshot_kind = QualityProjectSnapshotKind::parse(&args.snapshot_kind)
        .ok_or_else(|| anyhow::anyhow!("unsupported snapshot_kind `{}`", args.snapshot_kind))?;
    let compare_against = QualityProjectSnapshotCompareAgainst::parse(&args.compare_against)
        .ok_or_else(|| anyhow::anyhow!("unsupported compare_against `{}`", args.compare_against))?;
    let auto_index = args.auto_index.unwrap_or(true);
    let persist_artifacts = args.persist_artifacts.unwrap_or(true);

    let capture = engine.quality_project_snapshot(&QualityProjectSnapshotOptions {
        snapshot_kind,
        wave_id: args.wave_id,
        output_root: args
            .output_root
            .as_ref()
            .map(|path| path.display().to_string()),
        compare_against,
        auto_index,
        promote_self_baseline: args.promote_self_baseline,
        persist_artifacts,
    })?;

    if args.fail_on_regression
        && capture
            .delta
            .as_ref()
            .is_some_and(|delta| delta.gate_status == QualityProjectGateStatus::Regression)
    {
        let reasons = capture
            .delta
            .as_ref()
            .map(|delta| delta.regression_reasons.join(", "))
            .unwrap_or_else(|| "unknown regression".to_string());
        bail!("quality snapshot regression gate failed: {reasons}");
    }

    if json {
        let mut value = serde_json::to_value(&capture)?;
        sanitize_value_for_privacy(privacy_mode, &mut value);
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        let baseline = capture
            .artifacts
            .baseline_summary
            .as_deref()
            .map(|path| sanitize_path_text(privacy_mode, path))
            .unwrap_or_else(|| "<none>".to_string());
        let snapshot_root = capture
            .artifacts
            .snapshot_root
            .as_deref()
            .map(|path| sanitize_path_text(privacy_mode, path))
            .unwrap_or_else(|| "<none>".to_string());
        let gate = capture
            .delta
            .as_ref()
            .map(|delta| match delta.gate_status {
                QualityProjectGateStatus::Ok => "ok",
                QualityProjectGateStatus::Regression => "regression",
            })
            .unwrap_or("n/a");
        print_line(format!(
            "snapshot_kind={}, total_violations={}, post_refresh_status={}, gate_status={}, snapshot_root={}, baseline_summary={}",
            args.snapshot_kind,
            capture.snapshot.total_violations,
            capture.snapshot.quality_status_after_refresh.as_str(),
            gate,
            snapshot_root,
            baseline
        ));
    }

    Ok(())
}
