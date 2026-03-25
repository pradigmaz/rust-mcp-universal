# Investigation Surface

`rust-mcp-universal` now exposes a bounded universal investigation layer on top of retrieval/navigation.

## Public Surface

- `preflight`
- `symbol_body`
- `route_trace`
- `constraint_evidence`
- `concept_cluster`
- `divergence_report`
- CLI-only: `investigation-benchmark`

## Stage 9 Eval Gate

`investigation-benchmark` now has two operating modes:

- report mode: evaluate the current implementation against a chosen investigation dataset
- compare mode: evaluate the current implementation and attach machine-readable `diff` output against a trusted `--baseline-report`

Operational stage 9 artifacts now live under `baseline/investigation/stage9/`:

- aggregate gate dataset: `investigation_dataset.json`
- per-capability gold datasets: `gold/*.json`
- trusted compare baseline: `baseline_report.json`
- latest accepted report snapshot: `latest_report.json`
- absolute thresholds: `thresholds.json`

Gate policy:

- absolute thresholds still block on pass-rate / latency / privacy / unsupported-source regressions
- compare mode additionally blocks on any machine-readable regression against the trusted baseline report
- ranking/scoring regressions for `concept_cluster` remain blocking even when other absolute thresholds still pass

## Stable Contracts

- `symbol_body` keeps the existing nested contract:
  - `items[].anchor.path`
  - `items[].anchor.symbol`
  - `items[].anchor.language`
  - `items[].body`
  - `items[].span.start_line`
  - `items[].span.end_line`
  - `items[].source_kind`
  - `items[].confidence`
- `symbol_body` also exposes additive metadata:
  - `items[].resolution_kind = exact_symbol_span | nearest_indexed_lines | chunk_excerpt_anchor`
  - `ambiguity_status = none | multiple_exact | partial_only`
- `route_trace` uses its own `RouteTraceResult` contract:
  - `best_route`
  - `alternate_routes`
  - `unresolved_gaps`
- `constraint_evidence` exposes canonical evidence objects:
  - `items[].constraint_kind`
  - `items[].source_kind`
  - `items[].path`
  - `items[].line_start`
  - `items[].line_end`
  - `items[].excerpt`
  - `items[].confidence`
  - `items[].normalized_key`
- `constraint_evidence` also keeps additive compatibility aliases:
  - `items[].kind`
  - `items[].source_path`
  - `items[].source_span`
  - `items[].normalized_text`
- `divergence_report` exposes explainability-first comparison fields:
  - `surface_kind = divergence_explainability`
  - `overall_severity`
  - `manual_review_required`
  - `summary`
  - `variants`
  - `shared_evidence`
  - `divergence_signals`
  - `unknowns`
  - `missing_evidence`
  - `recommended_followups`

## Embedded Integration Policy

- `query_report` is the primary embedded explainability surface.
  - It now exposes additive `investigation_summary`.
  - `investigation_summary.surface_kind = embedded_investigation_hints`.
  - The payload is compact and summary-only: no raw `symbol_body.body`, no full `variants`, no full `divergence_signals`.
  - Embedded divergence is preview-only:
    - `surface_kind = divergence_preview`
    - `authoritative_tool = divergence_report`
    - `preview_only = true`
  - Retrieval explainability still lives in `retrieval_pipeline`, `selected_context[].explain`, and `confidence.signals`; embedded investigation does not replace the standalone comparison report.
- `context_pack` exposes additive `investigation_hints`.
  - The payload is lightweight guidance for further reading, not a second full report.
  - It includes only `top_variants`, `route_summary`, `constraint_keys`, and `followups`.
- `agent_bootstrap` does not add a separate top-level investigation object.
  - Investigation is available transitively via `query_bundle.report.investigation_summary` when `query_bundle` exists.
- Privacy mode applies to the new embedded fields with the same masking/hash rules as the existing report and investigation payloads.

## Support Matrix

| Capability | Rust | Python | TypeScript/JavaScript | SQL/Prisma-like |
|---|---|---|---|---|
| `symbol_body` | yes | yes | yes | excerpt-only |
| `route_trace` | call_path + typed route | call_path + typed route | call_path + typed route | gap/evidence only |
| `constraint_evidence` | adapter-registry: sqlx/diesel-like + `.sql` | adapter-registry: SQLAlchemy/Alembic-like | adapter-registry: ORM/schema-like | adapter-registry: strong/index evidence |
| `divergence_report` | yes | yes | partial | indirect/evidence-proxy |

## Symbol Body Resolution

`symbol_body` uses a deterministic fallback ladder:

1. `exact_symbol_span`
2. `nearest_indexed_lines`
3. `chunk_excerpt_anchor`

Ambiguity rules:

- multiple exact symbol matches: return exact matches in stable order up to `limit`, `ambiguity_status=multiple_exact`
- no exact match but partial symbol matches exist: use partial matches only, `ambiguity_status=partial_only`
- unsupported source classes: return `capability_status=unsupported|partial` with `unsupported_sources`

## Examples

```bash
rmu-cli --project-path . --json preflight
rmu-cli --project-path . --json symbol-body --seed "src/lib.rs:1" --seed-kind path_line --auto-index
rmu-cli --project-path . --json route-trace --seed "resolve_origin" --seed-kind query --auto-index
rmu-cli --project-path . --json constraint-evidence --seed "resolve_origin" --seed-kind query --auto-index
rmu-cli --project-path . --json divergence-report --seed "resolve_origin" --seed-kind query --auto-index
rmu-cli --project-path . --json investigation-benchmark --dataset baseline/investigation/stage9/investigation_dataset.json --thresholds baseline/investigation/stage9/thresholds.json --auto-index
rmu-cli --project-path . --json investigation-benchmark --dataset baseline/investigation/stage9/investigation_dataset.json --baseline-report baseline/investigation/stage9/baseline_report.json --thresholds baseline/investigation/stage9/thresholds.json --auto-index --enforce-gates
```

## Stage 9 Dataset Layout

- `baseline/investigation/stage9/investigation_dataset.json` is the aggregate gate corpus used by CI and accepted report refreshes.
- `baseline/investigation/stage9/gold/symbol_body_dataset.json` contains only `symbol_body` cases.
- `baseline/investigation/stage9/gold/route_trace_dataset.json` contains only `route_trace` cases.
- `baseline/investigation/stage9/gold/constraint_evidence_dataset.json` contains only `constraint_evidence` cases.
- `baseline/investigation/stage9/gold/concept_cluster_dataset.json` contains only `concept_cluster` cases, including low-signal and semantic fail-open coverage.
- `baseline/investigation/stage9/gold/divergence_report_dataset.json` contains only `divergence_report` cases.
  - divergence gold labels pin not only `axis` and `severity`, but also `evidence_strength` and `classification_reason`.
  - `false_positive_divergence_rate` counts only unexpected problem-like signals; unexpected `informational` and `likely_expected` preview signals do not fail the gate on their own.

## Troubleshooting

### Empty index

- Symptom: query/investigation tools fail with `index is empty`.
- Recovery: run with `--auto-index` or rebuild the index explicitly.
- Note: investigation tools with `--auto-index` may trigger a bounded `mixed` reindex when the current index scope does not cover the requested seed paths.

### Future migration DB

- Symptom: compatibility error mentions `newer than binary supported`.
- Recovery: rebuild/restart with a fresh binary or recreate the index with a compatible binary.

### Stale binary on Windows

- Symptom: `preflight` reports `running_binary_stale=true` with `status=incompatible`, or `stale_process_suspected=true` when another same-binary server PID is still alive.
- Recovery: restart through `scripts/rmu-mcp-server-fresh.cmd`.

### Unsupported source class

- Symptom: `capability_status=unsupported` or non-empty `unsupported_sources`.
- Recovery: inspect the file class and fall back to retrieval/context tools if the source is outside bounded v1 support.
