use anyhow::Result;

use super::super::parsing::parse_optional_metric;
use super::{ThresholdConfig, parse::ParsedThresholdSections};

pub(super) fn normalize_thresholds(
    sections: &ParsedThresholdSections<'_>,
    source: &str,
) -> Result<ThresholdConfig> {
    let min_source = format!("{source}.min");
    let max_source = format!("{source}.max");

    Ok(ThresholdConfig {
        min_recall_at_k: parse_optional_metric(sections.min, "recall_at_k", &min_source)?,
        min_mrr_at_k: parse_optional_metric(sections.min, "mrr_at_k", &min_source)?,
        min_ndcg_at_k: parse_optional_metric(sections.min, "ndcg_at_k", &min_source)?,
        max_avg_estimated_tokens: parse_optional_metric(
            sections.max,
            "avg_estimated_tokens",
            &max_source,
        )?,
        max_latency_p50_ms: parse_optional_metric(sections.max, "latency_p50_ms", &max_source)?,
        max_latency_p95_ms: parse_optional_metric(sections.max, "latency_p95_ms", &max_source)?,
    })
}
