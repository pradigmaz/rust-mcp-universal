use super::*;

#[test]
fn query_benchmark_reports_baseline_metrics() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let dataset_path = project_dir.join("benchmark-dataset.json");
    write_single_query_dataset(&dataset_path)?;

    let report = engine.query_benchmark(
        &dataset_path,
        QueryBenchmarkOptions::new(5, 20, false, 12_000, 3_000),
    )?;
    assert_eq!(report.query_count, 1);
    assert_eq!(report.k, 5);
    assert!((0.0..=1.0).contains(&report.recall_at_k));
    assert!((0.0..=1.0).contains(&report.mrr_at_k));
    assert!((0.0..=1.0).contains(&report.ndcg_at_k));
    assert!(report.recall_at_k > 0.0);
    assert!(report.mrr_at_k > 0.0);
    assert!(report.ndcg_at_k > 0.0);
    assert!(report.avg_estimated_tokens >= 0.0);
    assert!(report.latency_p50_ms >= 0.0);
    assert!(report.latency_p95_ms >= 0.0);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn query_benchmark_compare_mode_returns_median_diff_and_gate_results() -> Result<(), Box<dyn Error>>
{
    let (project_dir, engine) = setup_indexed_project()?;
    let dataset_path = project_dir.join("benchmark-dataset.json");
    write_single_query_dataset(&dataset_path)?;

    let baseline = QueryBenchmarkOptions::new(5, 20, false, 12_000, 3_000).with_auto_index(true);
    let candidate = QueryBenchmarkOptions::new(5, 20, true, 12_000, 3_000).with_auto_index(true);
    let loose_thresholds = QueryBenchmarkGateThresholds {
        quality_max_drop_ratio: 1.0,
        latency_p50_max_increase_ratio: 10.0,
        latency_p95_max_increase_ratio: 10.0,
        latency_p95_max_increase_ms: 10_000.0,
        token_cost_max_increase_ratio: 10.0,
    };
    let report = engine.query_benchmark_baseline_vs_candidate(
        &dataset_path,
        QueryBenchmarkComparisonOptions::new(baseline, candidate)
            .with_runs(3)
            .with_gate_thresholds(loose_thresholds)
            .with_fail_fast(true),
    )?;

    assert_eq!(report.runs_count, 3);
    assert_eq!(report.median_rule, "median_of_3_runs");
    assert_eq!(report.baseline.runs.len(), 3);
    assert_eq!(report.candidate.runs.len(), 3);
    assert_eq!(report.baseline.median.query_count, 1);
    assert_eq!(report.candidate.median.query_count, 1);
    assert!(report.diff.recall_at_k.delta.is_finite());
    assert!(report.diff.avg_estimated_tokens.delta.is_finite());
    assert!(report.gates.overall_pass);
    assert!(report.gates.first_failed_category.is_none());

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn query_benchmark_compare_mode_fail_fast_stops_on_first_failed_gate() -> Result<(), Box<dyn Error>>
{
    let (project_dir, engine) = setup_indexed_project()?;
    let dataset_path = project_dir.join("benchmark-dataset.json");
    write_single_query_dataset(&dataset_path)?;

    let baseline = QueryBenchmarkOptions::new(5, 20, false, 12_000, 3_000).with_auto_index(true);
    let candidate = QueryBenchmarkOptions::new(5, 20, false, 12_000, 3_000).with_auto_index(true);
    let strict_quality = QueryBenchmarkGateThresholds {
        quality_max_drop_ratio: -0.1,
        latency_p50_max_increase_ratio: 10.0,
        latency_p95_max_increase_ratio: 10.0,
        latency_p95_max_increase_ms: 10_000.0,
        token_cost_max_increase_ratio: 10.0,
    };
    let report = engine.query_benchmark_baseline_vs_candidate(
        &dataset_path,
        QueryBenchmarkComparisonOptions::new(baseline, candidate)
            .with_runs(1)
            .with_gate_thresholds(strict_quality)
            .with_fail_fast(true),
    )?;

    assert!(!report.gates.overall_pass);
    assert_eq!(
        report.gates.first_failed_category.as_deref(),
        Some("quality")
    );
    assert_eq!(report.gates.categories.len(), 3);
    assert!(!report.gates.categories[0].skipped);
    assert!(report.gates.categories[1].skipped);
    assert!(report.gates.categories[2].skipped);

    cleanup_project(&project_dir);
    Ok(())
}
