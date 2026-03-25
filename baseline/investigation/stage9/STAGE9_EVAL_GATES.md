# Stage 9 Eval Gates

This directory is the operational release-gate surface for the investigation layer.

## Required Artifacts

- `investigation_dataset.json`
  Current aggregate gate corpus used by the dedicated investigation CI workflow.
- `gold/*.json`
  Per-capability gold datasets used to keep the aggregate corpus auditable and split by tool.
- `baseline_report.json`
  Trusted machine-readable baseline report used by compare mode and CI regression checks.
- `latest_report.json`
  Latest accepted report snapshot produced from the current `stage9` aggregate dataset and thresholds.
- `thresholds.json`
  Absolute acceptance thresholds for the current accepted `heuristic_v2` scope.

## Gate Commands

Refresh the current accepted report:

```powershell
target\debug\rmu-cli.exe --project-path . --db-path <temp-db> --json investigation-benchmark --dataset baseline/investigation/stage9/investigation_dataset.json --thresholds baseline/investigation/stage9/thresholds.json --auto-index
```

Run compare mode against the trusted baseline:

```powershell
target\debug\rmu-cli.exe --project-path . --db-path <temp-db> --json investigation-benchmark --dataset baseline/investigation/stage9/investigation_dataset.json --baseline-report baseline/investigation/stage9/baseline_report.json --thresholds baseline/investigation/stage9/thresholds.json --auto-index --enforce-gates
```

## Blocking Metrics

Absolute thresholds still block on:

- case pass-rate per capability
- latency p95
- privacy failures
- unsupported-source rate
- `constraint_evidence_precision`
- `constraint_source_recall`
- `variant_recall_at_3`
- `top_variant_precision`
- `variant_rank_consistency`
- `semantic_state_coverage`
- `semantic_fail_open_visibility`
- `low_signal_semantic_false_penalty_rate`
- `divergence_signal_precision`
- `false_positive_divergence_rate`
- `explain_evidence_coverage`

Compare mode additionally blocks on any regression in the machine-readable `diff` report.

## Diff Contract

The top-level `diff` object is designed for CI consumption and contains:

- `baseline_case_count`
- `current_case_count`
- `per_tool_deltas`
- `regressed_metrics`
- `improved_metrics`
- `regression_failures`

Interpretation:

- `per_tool_deltas` is the full structured comparison for metrics that exist in both reports.
- `regressed_metrics` is the subset that worsened according to the metric direction policy.
- `improved_metrics` is the subset that improved.
- `regression_failures` contains hard blocking reasons such as missing metrics, case inventory drift, privacy regression, or numeric regressions.

## Refresh Rules

- Refresh `latest_report.json` only from `baseline/investigation/stage9/investigation_dataset.json`.
- Refresh `baseline_report.json` only when intentionally accepting a new investigation-quality baseline.
- Keep `stage0` artifacts as historical freeze inputs; do not repurpose them as the current release gate.
- If the aggregate dataset changes, regenerate `latest_report.json`, review `baseline_report.json`, and keep the `gold/*.json` union exactly aligned with the aggregate corpus.
