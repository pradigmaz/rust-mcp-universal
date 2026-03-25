use anyhow::Result;
use rmu_core::{Engine, QualityHotspotAggregation, QualityHotspotsOptions, QualityHotspotsResult};

pub(super) fn run_quality_hotspots(
    engine: &Engine,
    aggregation: QualityHotspotAggregation,
) -> Result<QualityHotspotsResult> {
    engine.quality_hotspots(&QualityHotspotsOptions {
        limit: 20,
        aggregation,
        ..QualityHotspotsOptions::default()
    })
}

pub(super) fn top_hotspot_bucket_ids(result: &QualityHotspotsResult, limit: usize) -> Vec<String> {
    result
        .buckets
        .iter()
        .take(limit)
        .map(|bucket| bucket.bucket_id.clone())
        .collect()
}
