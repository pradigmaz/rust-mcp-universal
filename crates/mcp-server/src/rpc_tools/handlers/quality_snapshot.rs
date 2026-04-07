use anyhow::Result;
use serde_json::Value;

use rmu_core::{
    Engine, MigrationMode, PrivacyMode, QualityProjectGateStatus,
    QualityProjectSnapshotCompareAgainst, QualityProjectSnapshotKind,
    QualityProjectSnapshotOptions, sanitize_value_for_privacy,
};

use crate::ServerState;
use crate::rpc_tools::errors::{invalid_params_error, tool_domain_error};
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_non_empty_string, reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::{ensure_query_index_ready, parse_optional_migration_mode, parse_optional_privacy_mode};

pub(super) fn quality_snapshot(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "quality_snapshot",
        &[
            "snapshot_kind",
            "wave_id",
            "output_root",
            "compare_against",
            "auto_index",
            "persist_artifacts",
            "promote_self_baseline",
            "fail_on_regression",
            "privacy_mode",
            "migration_mode",
        ],
    )?;

    let snapshot_kind = parse_optional_non_empty_string(args, "quality_snapshot", "snapshot_kind")?
        .map(|raw| {
            QualityProjectSnapshotKind::parse(&raw).ok_or_else(|| {
                invalid_params_error(
                    "quality_snapshot `snapshot_kind` must be one of: ad_hoc, before, after, baseline",
                )
            })
        })
        .transpose()?
        .unwrap_or(QualityProjectSnapshotKind::AdHoc);
    let wave_id = parse_optional_non_empty_string(args, "quality_snapshot", "wave_id")?;
    let output_root = parse_optional_non_empty_string(args, "quality_snapshot", "output_root")?;
    let compare_against =
        parse_optional_non_empty_string(args, "quality_snapshot", "compare_against")?
            .map(|raw| {
                QualityProjectSnapshotCompareAgainst::parse(&raw).ok_or_else(|| {
                    invalid_params_error(
                        "quality_snapshot `compare_against` must be one of: none, self_baseline, wave_before",
                    )
                })
            })
            .transpose()?
            .unwrap_or(QualityProjectSnapshotCompareAgainst::None);
    let auto_index = parse_optional_bool(args, "quality_snapshot", "auto_index")?.unwrap_or(true);
    let persist_artifacts =
        parse_optional_bool(args, "quality_snapshot", "persist_artifacts")?.unwrap_or(true);
    let promote_self_baseline =
        parse_optional_bool(args, "quality_snapshot", "promote_self_baseline")?.unwrap_or(false);
    let fail_on_regression =
        parse_optional_bool(args, "quality_snapshot", "fail_on_regression")?.unwrap_or(false);
    let privacy_mode = parse_optional_privacy_mode(args, "quality_snapshot", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let migration_mode = parse_optional_migration_mode(args, "quality_snapshot", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);

    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )
    .map_err(|err| tool_domain_error(err.to_string()))?;

    if auto_index {
        ensure_query_index_ready(&engine, true)
            .map_err(|err| tool_domain_error(err.to_string()))?;
    }

    let capture = engine
        .quality_project_snapshot(&QualityProjectSnapshotOptions {
            snapshot_kind,
            wave_id,
            output_root,
            compare_against,
            auto_index,
            promote_self_baseline,
            persist_artifacts,
        })
        .map_err(|err| tool_domain_error(err.to_string()))?;

    if fail_on_regression
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
        return Err(tool_domain_error(format!(
            "quality snapshot regression gate failed: {reasons}"
        )));
    }

    let mut payload = serde_json::to_value(capture)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}
