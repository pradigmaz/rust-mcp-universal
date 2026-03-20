# Stage 3 - Query-Time Chunk Retrieval

Дата: 2026-03-03

## Что сделано

- Добавлен внутренний query-time chunk-candidate stage (semantic) в `chunking.rs`:
  - выбор лучшего chunk на файл по blended score (semantic + lexical),
  - агрегация `chunk -> path` без изменения контракта `SearchHit`.
- Budget pack теперь поднимает приоритет chunk-источников (`context.rs`):
  - сначала кандидаты с chunk,
  - затем preview fallback.
- В `query_report` добавлена telemetry:
  - `index_telemetry.chunk_coverage`,
  - `index_telemetry.chunk_source`.
- В пайплайн отчёта добавлен stage:
  - `semantic_chunk_candidate_pool(file_chunks)`,
  - budget stage переименован в `budget_pack(prioritize_chunk_sources)`.
- Обновлены JSON schema и MCP-тесты под новые поля.

## Критерии этапа

### 1) chunk_coverage >= 0.95

- Файл: `baseline/stage3/chunk_telemetry_summary.json`
- Результат:
  - `chunk_coverage_avg = 1.0`
  - `chunk_coverage_min = 1.0`
- Статус: выполнено.

### 2) avg_estimated_tokens снижён >= 15% без падения nDCG@k

- Baseline (Stage 0, median):
  - `avg_estimated_tokens = 1846.9166`
  - `ndcg@10 = 0.19167012`
- Текущий Stage 3 (median, 5 прогонов):
  - `avg_estimated_tokens = 1285.5`
  - `ndcg@10 = 0.24952103`
- Изменение:
  - token-cost: `-30.40%`
  - nDCG@10: `+30.18%` (не упал)
- Статус: выполнено.

## Артефакты этапа

- `baseline/stage3/query_benchmark_run_1.json` ... `query_benchmark_run_5.json`
- `baseline/stage3/query_benchmark_summary.json`
- `baseline/stage3/chunk_telemetry_summary.json`
- `baseline/stage3/stage3_gate_eval.json`

## Команды верификации

```bash
cargo test -p rmu-core
cargo test -p rmu-mcp-server -p rmu-cli
target/debug/rmu-cli --project-path . --json query-benchmark --dataset baseline/stage0/query_benchmark_dataset.json --semantic --k 10 --limit 20 --max-chars 12000 --max-tokens 3000
target/debug/rmu-cli --project-path . --json report --query "<query>" --limit 20 --semantic --max-chars 12000 --max-tokens 3000
```
