use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Result, bail};

use crate::engine::Engine;
use crate::index_scope::IndexScope;
use crate::model::{
    IndexingOptions, QualityHotspotAggregation, QualityHotspotBucket, QualityHotspotRuleCount,
    QualityHotspotStructuralSignals, QualityHotspotsOptions, QualityHotspotsResult,
    QualityHotspotsSortBy, QualityHotspotsSummary, QualityRiskScoreBreakdown, QualityStatus,
    RuleViolationFileHit, RuleViolationsOptions, RuleViolationsSortBy,
};
use crate::quality::{
    StructuralPolicy, StructuralUnmatchedBehavior, default_quality_policy, load_quality_policy,
};

const HOTSPOT_SCAN_LIMIT: usize = 100_000;
const UNMATCHED_MODULE_BUCKET: &str = "unmatched";

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
    let mut buckets =
        aggregate_buckets(&scan.hits, options.aggregation, policy.structural.as_ref())?;
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

fn aggregate_buckets(
    hits: &[RuleViolationFileHit],
    aggregation: QualityHotspotAggregation,
    structural_policy: Option<&StructuralPolicy>,
) -> Result<Vec<QualityHotspotBucket>> {
    let matcher = ModuleBucketMatcher::new(aggregation, structural_policy)?;
    let mut grouped = BTreeMap::<String, Vec<&RuleViolationFileHit>>::new();

    for hit in hits {
        let Some(bucket_id) = bucket_id_for_hit(hit, aggregation, &matcher)? else {
            continue;
        };
        grouped.entry(bucket_id).or_default().push(hit);
    }

    Ok(grouped
        .into_iter()
        .map(|(bucket_id, bucket_hits)| build_bucket(bucket_id, aggregation, &bucket_hits))
        .collect())
}

fn bucket_id_for_hit(
    hit: &RuleViolationFileHit,
    aggregation: QualityHotspotAggregation,
    matcher: &ModuleBucketMatcher,
) -> Result<Option<String>> {
    match aggregation {
        QualityHotspotAggregation::File => Ok(Some(hit.path.clone())),
        QualityHotspotAggregation::Directory => Ok(Some(directory_bucket_id(&hit.path))),
        QualityHotspotAggregation::Module => matcher.bucket_id(&hit.path),
    }
}

fn build_bucket(
    bucket_id: String,
    aggregation: QualityHotspotAggregation,
    hits: &[&RuleViolationFileHit],
) -> QualityHotspotBucket {
    let active_violation_count = hits.iter().map(|hit| hit.violations.len()).sum();
    let suppressed_violation_count = hits.iter().map(|hit| hit.suppressed_violations.len()).sum();
    let rule_counts = build_rule_counts(hits);
    let structural_signals = build_structural_signals(hits);
    let top_files = build_top_files(hits, aggregation);
    let hotspot_score = match aggregation {
        QualityHotspotAggregation::File => hits
            .first()
            .and_then(|hit| hit.risk_score)
            .map(|risk| risk.score)
            .unwrap_or(0.0),
        QualityHotspotAggregation::Directory | QualityHotspotAggregation::Module => {
            let top_five_sum = hits
                .iter()
                .filter_map(|hit| hit.risk_score.map(|risk| risk.score))
                .collect::<Vec<_>>();
            let mut risk_scores = top_five_sum;
            risk_scores.sort_by(|left, right| right.partial_cmp(left).unwrap_or(Ordering::Equal));
            let top_five_sum: f64 = risk_scores.into_iter().take(5).sum();
            let active_hot_file_count =
                hits.iter().filter(|hit| !hit.violations.is_empty()).count();
            let structural_bonus = (structural_signals.module_cycle_member
                + structural_signals.hub_module
                + structural_signals.cross_layer_dependency)
                as f64;
            top_five_sum + active_hot_file_count as f64 + structural_bonus
        }
    };

    QualityHotspotBucket {
        bucket_id,
        hotspot_score,
        active_violation_count,
        suppressed_violation_count,
        risk_score: (aggregation == QualityHotspotAggregation::File)
            .then(|| hits.first().and_then(|hit| hit.risk_score))
            .flatten(),
        rule_counts,
        structural_signals,
        top_files,
        delta: Default::default(),
    }
}

fn build_rule_counts(hits: &[&RuleViolationFileHit]) -> Vec<QualityHotspotRuleCount> {
    let mut counts = HashMap::<String, usize>::new();
    for hit in hits {
        for violation in &hit.violations {
            *counts.entry(violation.rule_id.clone()).or_default() += 1;
        }
    }
    let mut counts = counts
        .into_iter()
        .map(|(rule_id, violations)| QualityHotspotRuleCount {
            rule_id,
            violations,
        })
        .collect::<Vec<_>>();
    counts.sort_by(|left, right| {
        right
            .violations
            .cmp(&left.violations)
            .then_with(|| left.rule_id.cmp(&right.rule_id))
    });
    counts
}

fn build_structural_signals(hits: &[&RuleViolationFileHit]) -> QualityHotspotStructuralSignals {
    let mut signals = QualityHotspotStructuralSignals::default();
    for hit in hits {
        for violation in &hit.violations {
            match violation.rule_id.as_str() {
                "module_cycle_member" => signals.module_cycle_member += 1,
                "hub_module" => signals.hub_module += 1,
                "cross_layer_dependency" => signals.cross_layer_dependency += 1,
                _ => {}
            }
        }
    }
    signals
}

fn build_top_files(
    hits: &[&RuleViolationFileHit],
    aggregation: QualityHotspotAggregation,
) -> Vec<String> {
    if aggregation == QualityHotspotAggregation::File {
        return hits
            .first()
            .map(|hit| vec![hit.path.clone()])
            .unwrap_or_default();
    }
    let mut files = hits
        .iter()
        .map(|hit| {
            (
                hit.risk_score
                    .unwrap_or(QualityRiskScoreBreakdown {
                        score: 0.0,
                        components: Default::default(),
                        weights: Default::default(),
                    })
                    .score,
                hit.path.clone(),
            )
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.1.cmp(&right.1))
    });
    files.into_iter().map(|(_, path)| path).take(5).collect()
}

fn compare_buckets(
    left: &QualityHotspotBucket,
    right: &QualityHotspotBucket,
    sort_by: QualityHotspotsSortBy,
) -> Ordering {
    let primary = match sort_by {
        QualityHotspotsSortBy::HotspotScore => right
            .hotspot_score
            .partial_cmp(&left.hotspot_score)
            .unwrap_or(Ordering::Equal),
        QualityHotspotsSortBy::RiskScoreDelta => right
            .delta
            .risk_score_delta
            .partial_cmp(&left.delta.risk_score_delta)
            .unwrap_or(Ordering::Equal),
        QualityHotspotsSortBy::NewViolations => {
            right.delta.new_violations.cmp(&left.delta.new_violations)
        }
    };
    primary
        .then_with(|| {
            right
                .active_violation_count
                .cmp(&left.active_violation_count)
        })
        .then_with(|| left.bucket_id.cmp(&right.bucket_id))
}

fn directory_bucket_id(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(|parent| parent.to_str())
        .map(|value| {
            let normalized = value.replace('\\', "/");
            if normalized.is_empty() {
                ".".to_string()
            } else {
                normalized
            }
        })
        .unwrap_or_else(|| ".".to_string())
}

struct ModuleBucketMatcher {
    aggregation: QualityHotspotAggregation,
    zones: Vec<(String, IndexScope)>,
    unmatched_behavior: StructuralUnmatchedBehavior,
}

impl ModuleBucketMatcher {
    fn new(
        aggregation: QualityHotspotAggregation,
        structural_policy: Option<&StructuralPolicy>,
    ) -> Result<Self> {
        if aggregation != QualityHotspotAggregation::Module {
            return Ok(Self {
                aggregation,
                zones: Vec::new(),
                unmatched_behavior: StructuralUnmatchedBehavior::Allow,
            });
        }
        let Some(policy) = structural_policy else {
            return Ok(Self {
                aggregation,
                zones: Vec::new(),
                unmatched_behavior: StructuralUnmatchedBehavior::Allow,
            });
        };
        let zones = policy
            .zones
            .iter()
            .map(|zone| {
                Ok((
                    zone.id.clone(),
                    IndexScope::new(&IndexingOptions {
                        profile: None,
                        changed_since: None,
                        changed_since_commit: None,
                        include_paths: zone.paths.clone(),
                        exclude_paths: Vec::new(),
                        reindex: false,
                    })?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            aggregation,
            zones,
            unmatched_behavior: policy.unmatched_behavior,
        })
    }

    fn bucket_id(&self, rel_path: &str) -> Result<Option<String>> {
        if self.aggregation != QualityHotspotAggregation::Module || self.zones.is_empty() {
            return Ok(Some(directory_bucket_id(rel_path)));
        }

        let matched = self
            .zones
            .iter()
            .filter(|(_, scope)| scope.allows(rel_path))
            .map(|(zone_id, _)| zone_id.clone())
            .collect::<Vec<_>>();
        match matched.as_slice() {
            [zone_id] => Ok(Some(zone_id.clone())),
            [] => match self.unmatched_behavior {
                StructuralUnmatchedBehavior::Allow => Ok(Some(UNMATCHED_MODULE_BUCKET.to_string())),
                StructuralUnmatchedBehavior::Ignore => Ok(None),
            },
            _ => bail!(
                "structural policy matches path `{rel_path}` to multiple zones: {}",
                matched.join(", ")
            ),
        }
    }
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
