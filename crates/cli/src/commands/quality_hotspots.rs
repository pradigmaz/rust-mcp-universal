use anyhow::{Result, bail};
use rmu_core::{
    Engine, PrivacyMode, QualityHotspotAggregation, QualityHotspotsOptions, QualityHotspotsSortBy,
    sanitize_path_text, sanitize_value_for_privacy,
};

use crate::output::{print_json, print_line};

pub(crate) struct QualityHotspotsArgs {
    pub(crate) aggregation: String,
    pub(crate) limit: usize,
    pub(crate) path_prefix: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) rule_ids: Vec<String>,
    pub(crate) sort_by: String,
    pub(crate) auto_index: bool,
}

pub(crate) fn run(
    engine: &Engine,
    json: bool,
    privacy_mode: PrivacyMode,
    args: QualityHotspotsArgs,
) -> Result<()> {
    if args.auto_index {
        let _ = engine.ensure_index_ready_with_policy(true)?;
        engine.refresh_quality_if_needed()?;
    } else if !engine.db_path.exists() {
        bail!(
            "index is empty; run an indexing flow or enable automatic indexing before requesting quality hotspots"
        );
    }

    let aggregation = QualityHotspotAggregation::parse(&args.aggregation)
        .ok_or_else(|| anyhow::anyhow!("unsupported aggregation `{}`", args.aggregation))?;
    let sort_by = QualityHotspotsSortBy::parse(&args.sort_by)
        .ok_or_else(|| anyhow::anyhow!("unsupported sort_by `{}`", args.sort_by))?;
    let result = engine.quality_hotspots(&QualityHotspotsOptions {
        limit: args.limit,
        path_prefix: args.path_prefix.map(|value| value.replace('\\', "/")),
        language: args.language,
        rule_ids: args.rule_ids,
        aggregation,
        sort_by,
    })?;

    if json {
        let mut value = serde_json::to_value(&result)?;
        sanitize_value_for_privacy(privacy_mode, &mut value);
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        let top_bucket = result
            .buckets
            .first()
            .map(|bucket| sanitize_path_text(privacy_mode, &bucket.bucket_id))
            .unwrap_or_else(|| "<none>".to_string());
        print_line(format!(
            "aggregation={}, buckets={}, hot_buckets={}, total_active_violations={}, top_bucket={}",
            result.summary.aggregation.as_str(),
            result.summary.evaluated_buckets,
            result.summary.hot_buckets,
            result.summary.total_active_violations,
            top_bucket
        ));
    }

    Ok(())
}
