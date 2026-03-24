use anyhow::Result;
use serde_json::Value;

use rmu_core::{
    Engine, MigrationMode, PrivacyMode, QualityHotspotAggregation, QualityHotspotsOptions,
    QualityHotspotsSortBy, sanitize_value_for_privacy,
};

use crate::ServerState;
use crate::rpc_tools::errors::{invalid_params_error, tool_domain_error};
use crate::rpc_tools::parsing::{
    parse_optional_non_empty_string, parse_optional_string_list, parse_optional_usize_with_min,
    reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::{ensure_query_index_ready, parse_optional_migration_mode, parse_optional_privacy_mode};

pub(super) fn quality_hotspots(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "quality_hotspots",
        &[
            "aggregation",
            "limit",
            "path_prefix",
            "language",
            "rule_ids",
            "sort_by",
            "auto_index",
            "privacy_mode",
            "migration_mode",
        ],
    )?;
    let aggregation = parse_optional_non_empty_string(args, "quality_hotspots", "aggregation")?
        .map(|raw| {
            QualityHotspotAggregation::parse(&raw).ok_or_else(|| {
                invalid_params_error(
                    "quality_hotspots `aggregation` must be one of: file, directory, module",
                )
            })
        })
        .transpose()?
        .unwrap_or(QualityHotspotAggregation::File);
    let limit = parse_optional_usize_with_min(args, "quality_hotspots", "limit", 1, 20)?;
    let path_prefix = parse_optional_non_empty_string(args, "quality_hotspots", "path_prefix")?
        .map(|value| value.replace('\\', "/"));
    let language = parse_optional_non_empty_string(args, "quality_hotspots", "language")?;
    let rule_ids =
        parse_optional_string_list(args, "quality_hotspots", "rule_ids")?.unwrap_or_default();
    let sort_by = parse_optional_non_empty_string(args, "quality_hotspots", "sort_by")?
        .map(|raw| {
            QualityHotspotsSortBy::parse(&raw).ok_or_else(|| {
                invalid_params_error(
                    "quality_hotspots `sort_by` must be one of: hotspot_score, risk_score_delta, new_violations",
                )
            })
        })
        .transpose()?
        .unwrap_or(QualityHotspotsSortBy::HotspotScore);
    let auto_index =
        crate::rpc_tools::parsing::parse_optional_bool(args, "quality_hotspots", "auto_index")?
            .unwrap_or(false);
    let privacy_mode = parse_optional_privacy_mode(args, "quality_hotspots", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let migration_mode =
        parse_optional_migration_mode(args, "quality_hotspots", "migration_mode")?
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
        engine
            .refresh_quality_if_needed()
            .map_err(|err| tool_domain_error(err.to_string()))?;
    } else if !engine.db_path.exists() {
        return Err(tool_domain_error(
            "index is empty; run an indexing flow or enable automatic indexing before requesting quality hotspots",
        ));
    }

    let result = engine
        .quality_hotspots(&QualityHotspotsOptions {
            limit,
            path_prefix,
            language,
            rule_ids,
            aggregation,
            sort_by,
        })
        .map_err(|err| tool_domain_error(err.to_string()))?;
    let mut payload = serde_json::to_value(result)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}
