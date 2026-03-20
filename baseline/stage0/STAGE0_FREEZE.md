# Stage 0 Freeze and Baseline

Дата фиксации: 2026-03-03  
Репозиторий: `rust-mcp-universal`  
Цель: закрыть требования Этапа 0 из `SEMANTIC_IMPROVEMENT_CHECKPLAN.md`.

## 1) Freeze контрактов

- CLI flags/commands зафиксированы: `baseline/stage0/contract_freeze_manifest.json`
- MCP tool names зафиксированы: `baseline/stage0/contract_freeze_manifest.json`
- JSON schema snapshot + SHA256 зафиксированы: `baseline/stage0/contract_freeze_manifest.json`

Контрольные источники:

- `crates/cli/src/args.rs`
- `crates/mcp-server/src/rpc_tools/registry.rs`
- `schemas/*.json`

## 2) Baseline через query-benchmark (5 прогонов)

Команда baseline:

```bash
rmu-cli --project-path . --json query-benchmark \
  --dataset baseline/stage0/query_benchmark_dataset.json \
  --semantic --k 10 --limit 20 --max-chars 12000 --max-tokens 3000
```

Raw прогоны:

- `baseline/stage0/query_benchmark_run_1.json`
- `baseline/stage0/query_benchmark_run_2.json`
- `baseline/stage0/query_benchmark_run_3.json`
- `baseline/stage0/query_benchmark_run_4.json`
- `baseline/stage0/query_benchmark_run_5.json`

Сводка с медианой:

- `baseline/stage0/query_benchmark_baseline_summary.json`

Базовые медианы:

- `recall@10`: `0.25`
- `MRR@10`: `0.196759`
- `nDCG@10`: `0.191670`
- `avg_estimated_tokens`: `1846.9166`
- `latency_p50_ms`: `23.1454`
- `latency_p95_ms`: `31.465199`

## 3) Пороги rollback релиза

Файл порогов:

- `baseline/stage0/release_rollback_thresholds.json`

Текущая политика:

- качество: не более `-5%` от baseline (`recall@k`, `MRR`, `nDCG`);
- токены: не более `+10%` к baseline `avg_estimated_tokens`;
- latency: `p50 <= +20%`, `p95 <= min(+15%, +30ms)`.

## 4) Dataset и стратификация запросов

Dataset:

- `baseline/stage0/query_benchmark_dataset.json`

Страты:

| Stratum | Query count | IDs |
|---|---:|---|
| `code` | 4 | `code-01..code-04` |
| `design` | 4 | `design-01..design-04` |
| `bugfix` | 4 | `bugfix-01..bugfix-04` |

Итог: `12` запросов, равномерная стратификация по `code/design/bugfix`.

