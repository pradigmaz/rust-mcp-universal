use crate::quality::{
    CURRENT_QUALITY_RULESET_VERSION, IndexedQualityMetrics, QUALITY_RULESET_ID,
    build_indexed_quality_facts, build_oversize_quality_facts, default_quality_policy,
    evaluate_quality,
};

#[test]
fn quality_constants_are_stable() {
    assert_eq!(QUALITY_RULESET_ID, "quality-core-v13");
    assert_eq!(CURRENT_QUALITY_RULESET_VERSION, 13);
}

#[test]
fn indexed_quality_collects_text_and_indexed_rules() {
    let facts = build_indexed_quality_facts(
        "src/lib.rs",
        "rust",
        64,
        Some(1),
        "use std::fmt;\nfn alpha() {}\n",
    );
    let evaluation = evaluate_quality(
        &facts,
        &IndexedQualityMetrics {
            symbol_count: Some(81),
            ..IndexedQualityMetrics::default()
        },
        &default_quality_policy(),
    );

    assert_eq!(evaluation.snapshot.total_lines, Some(2));
    assert_eq!(evaluation.snapshot.non_empty_lines, Some(2));
    assert_eq!(evaluation.snapshot.import_count, Some(1));
    assert!(
        evaluation
            .snapshot
            .violations
            .iter()
            .any(|entry| entry.rule_id == "max_symbol_count_per_file")
    );
}

#[test]
fn oversize_quality_stays_quality_only() {
    let facts = build_oversize_quality_facts("src/big.rs", "rust", 300_000, Some(1));
    let evaluation = evaluate_quality(
        &facts,
        &IndexedQualityMetrics::default(),
        &default_quality_policy(),
    );
    assert!(evaluation.snapshot.total_lines.is_none());
    assert!(evaluation.snapshot.non_empty_lines.is_none());
    assert!(evaluation.snapshot.import_count.is_none());
    assert!(
        evaluation
            .snapshot
            .violations
            .iter()
            .any(|entry| entry.rule_id == "max_size_bytes")
    );
}

#[test]
fn basic_rule_registry_keeps_stable_rule_ids() {
    let facts = build_indexed_quality_facts(
        "src/lib.rs",
        "rust",
        64,
        Some(1),
        "use std::fmt;\nfn alpha() {}\n",
    );
    let evaluation = evaluate_quality(
        &facts,
        &IndexedQualityMetrics::default(),
        &default_quality_policy(),
    );
    let metric_ids = evaluation
        .snapshot
        .metrics
        .iter()
        .map(|entry| entry.metric_id.as_str())
        .collect::<Vec<_>>();

    assert!(metric_ids.contains(&"size_bytes"));
    assert!(metric_ids.contains(&"non_empty_lines"));
    assert!(metric_ids.contains(&"import_count"));
    assert!(metric_ids.contains(&"max_line_length"));
    assert!(metric_ids.contains(&"duplicate_block_count"));
    assert!(metric_ids.contains(&"duplicate_density_bps"));
}
