# STAGE8 - Rollout and Rollback

Дата: 2026-03-04

## Что внедрено

1. Волновой rollout для векторного слоя:
- `shadow` (`0%`)
- `canary_5` (`5%`)
- `canary_25` (`25%`)
- `full_100` (`100%`)

2. Feature flags:
- `vector_layer_enabled` (CLI global, MCP tool arg)
- `semantic_fail_mode` (уже был)
- `privacy_mode` (уже был)
- `migration_mode` (`auto|off`)

3. Rollback framework:
- Триггеры: `quality`, `latency`, `token_cost`, `privacy`, `error_spike`
- Рекомендации:
  - `none`
  - `fast` (flag rollback)
  - `full` (backup restore + binary rollback)

4. Query benchmark compare (`baseline_vs_candidate`) теперь возвращает:
- `feature_flags`
- `rollout` (`stable_cycles_required`, `stable_cycles_observed`, `ready_for_next_wave`, `waves`)
- `rollback` (recommended level + reasons + action lists)

## CLI примеры

```powershell
rmu --project-path . --vector-layer-enabled true --rollout-phase canary_5 search --query "symbol" --semantic
```

```powershell
rmu --project-path . --migration-mode off status
```

```powershell
rmu --project-path . --rollout-phase canary_25 --vector-layer-enabled true --json query-benchmark `
  --dataset baseline/stage0/query_benchmark_dataset.json `
  --baseline baseline/stage0/query_benchmark_baseline_summary.json `
  --thresholds baseline/stage0/release_rollback_thresholds.json `
  --runs 5
```

## MCP примеры (tool arguments)

- Query tools:
  - `vector_layer_enabled: boolean`
  - `rollout_phase: "shadow" | "canary_5" | "canary_25" | "full_100"`
  - `migration_mode: "auto" | "off"`

- Service tools (`index_status`, `workspace_brief`, `index`, `semantic_index`, `delete_index`, `db_maintenance`):
  - `migration_mode: "auto" | "off"`

## Ограничения

- Критерий этапа (`2 stable cycles`) считается не кодом, а фактическими прогонными данными benchmark.
- `migration_mode=off` запрещает автоинициализацию/миграции и требует уже инициализированную БД.

## Practical Drill (2026-03-05)

- Script: `baseline/stage8/run_offline_rollback_drill.ps1`
- Result artifact: `baseline/stage8/ROLLBACK_OFFLINE_DRILL_20260305_204655.json`
- Offline checks passed:
  - local `index`
  - `status` with `migration_mode=off`
  - lexical `search`
  - semantic `search` with local backend
- Fast rollback check passed:
  - benchmark compare returned `rollback.level = fast`
  - expected `rollback.fast_actions` present
- Full rollback dry-run passed:
  - backup files created
  - `delete-index --yes` executed
  - `migration_mode=off status` failed after deletion (expected)
  - DB restored from backup
  - `migration_mode=off status` succeeded after restore
