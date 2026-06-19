use anyhow::Result;

use crate::engine::Engine;
use crate::model::{
    QualityHotspotAggregation, QualityHotspotsOptions, QualityHotspotsResult,
    QualityHotspotsSummary, QualityStatus, RuleViolationsOptions, RuleViolationsSortBy,
};
use crate::quality::{default_quality_policy, load_quality_policy};

const HOTSPOT_SCAN_LIMIT: usize = 100_000;

use super::hotspots_buckets::{aggregate_buckets, compare_buckets};

pub(super) fn load_quality_hotspots(
    engine: &Engine,
    options: &QualityHotspotsOptions,
) -> Result<QualityHotspotsResult> {
    let scan = engine.rule_violations(&RuleViolationsOptions {
        limit: HOTSPOT_SCAN_LIMIT,
        path_prefix: options.path_prefix.clone(),
        language: options.language.clone(),
        rule_ids: options.rule_ids.clone(),
        metric_ids: Vec::new(),
        sort_metric_id: None,
        sort_by: RuleViolationsSortBy::ViolationCount,
    })?;
    let (policy, status) = match load_quality_policy(&engine.project_root) {
        Ok(policy) => (policy, scan.summary.status),
        Err(_) => (default_quality_policy(), QualityStatus::Degraded),
    };
    let mut buckets = aggregate_buckets(&scan.hits, options.aggregation, policy.layering.as_ref())?;
    buckets.sort_by(|left, right| compare_buckets(left, right, options.sort_by));

    let summary = QualityHotspotsSummary {
        status,
        aggregation: options.aggregation,
        evaluated_buckets: buckets.len(),
        hot_buckets: buckets
            .iter()
            .filter(|bucket| bucket.active_violation_count > 0)
            .count(),
        total_active_violations: buckets
            .iter()
            .map(|bucket| bucket.active_violation_count)
            .sum(),
        total_suppressed_violations: buckets
            .iter()
            .map(|bucket| bucket.suppressed_violation_count)
            .sum(),
        new_violations: 0,
        resolved_violations: 0,
        hotspot_score_delta_total: 0.0,
        risk_score_delta_total: 0.0,
    };

    buckets.truncate(options.limit);

    Ok(QualityHotspotsResult { summary, buckets })
}

#[allow(dead_code)]
fn _empty_result(
    status: QualityStatus,
    aggregation: QualityHotspotAggregation,
) -> QualityHotspotsResult {
    QualityHotspotsResult {
        summary: QualityHotspotsSummary {
            status,
            aggregation,
            evaluated_buckets: 0,
            hot_buckets: 0,
            total_active_violations: 0,
            total_suppressed_violations: 0,
            new_violations: 0,
            resolved_violations: 0,
            hotspot_score_delta_total: 0.0,
            risk_score_delta_total: 0.0,
        },
        buckets: Vec::new(),
    }
}
