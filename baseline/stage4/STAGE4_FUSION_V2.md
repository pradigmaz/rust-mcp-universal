# Этап 4: Dual-candidate fusion v2

Дата: 2026-03-03

## Что реализовано

- Три пула кандидатов: `lexical path`, `semantic file`, `semantic chunk`.
- Source-aware fusion через weighted RRF с дедупликацией по path.
- Адаптивный профиль запроса (`Precise/Balanced/Exploratory/Bugfix`) для весов fusion и probe-factor.
- Гейт запуска semantic, чтобы не раздувать latency на запросах с уверенным lexical.

## Калибровка

- Benchmark-набор: `baseline/stage0/query_benchmark_dataset.json`
- Команда: `target/debug/rmu-cli --project-path . --json query-benchmark --dataset baseline/stage0/query_benchmark_dataset.json --semantic --k 10 --limit 20 --max-chars 12000 --max-tokens 3000`
- Повторений: 5 (медиана)

## Итоги (медиана 5 прогонов)

- `recall@10`: **0.7083** (baseline: 0.25)
- `MRR@10`: **0.2392** (baseline: 0.1968)
- `nDCG@10`: **0.3562** (baseline: 0.1917)
- `avg_estimated_tokens`: **966.33** (baseline: 1846.92)
- `latency p50`: **31.5184 ms** (порог: <= 27.7745 ms)
- `latency p95`: **35.2486 ms** (порог: <= 36.1850 ms)

## Критерии этапа

- Рост `recall@10`, `MRR@10`, `nDCG@10` относительно baseline: **выполнено**.
- `latency p95` в целевом пороге: **выполнено (по медиане 5 прогонов)**.

## Артефакты

- `baseline/stage4/query_benchmark_run_1.json`
- `baseline/stage4/query_benchmark_run_2.json`
- `baseline/stage4/query_benchmark_run_3.json`
- `baseline/stage4/query_benchmark_run_4.json`
- `baseline/stage4/query_benchmark_run_5.json`
- `baseline/stage4/query_benchmark_summary.json`
