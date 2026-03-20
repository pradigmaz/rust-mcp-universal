# Этап 6: Eval Harness и Release Gate

Дата: 2026-03-03

## Что реализовано

- `query-benchmark` поддерживает режим `baseline vs candidate` в CLI и MCP.
- Добавлены fail-fast гейты по `quality`, `latency`, `token_cost`.
- Добавлен machine-readable diff-отчёт (JSON) с baseline/candidate/diff/gates.
- Добавлен CI workflow для regression gate:
  - `.github/workflows/benchmark-regression.yml`

## Как работает release-gate в CI

1. Workflow запускается на `pull_request`.
2. Dataset, baseline и thresholds читаются из base commit PR (`git show ${BASE_SHA}:...`).
3. Запускается `rmu-cli query-benchmark` в compare-mode с `--runs 5 --enforce-gates`.
4. При нарушении порогов команда завершается с non-zero кодом.
5. Артефакты JSON загружаются в GitHub Actions (`upload-artifact` с `if: always()`).

## Локальный запуск

```bash
target/debug/rmu-cli --project-path . --json query-benchmark \
  --dataset baseline/stage0/query_benchmark_dataset.json \
  --baseline baseline/stage0/query_benchmark_baseline_summary.json \
  --thresholds baseline/stage0/release_rollback_thresholds.json \
  --semantic --k 10 --limit 20 --max-chars 12000 --max-tokens 3000 \
  --runs 5 --enforce-gates
```

## Формат артефактов

- JSON payload compare-mode содержит:
  - `mode=baseline_vs_candidate`
  - `baseline.metrics`
  - `candidate.runs`, `candidate.median`
  - `diff` по ключевым метрикам
  - `thresholds.run_evaluations`, `thresholds.candidate_median_evaluation`, `thresholds.passed`

## Критерий merge-blocking

- Технически обеспечено workflow-джобой: при регрессии за порогами job падает.
- Для полного enforcement на репозитории check `release-gate` должен быть добавлен в Required status checks branch protection.
