# Stage 1: Binary x DB Compatibility Matrix

Дата: 2026-03-03

## Правило принятия решения

1. `schema_version` в БД `> CURRENT_SCHEMA_VERSION` -> `hard-fail` (без write side effects).
2. `schema_version` `<= CURRENT_SCHEMA_VERSION` -> допускается migration/update metadata.
3. Для непустого индекса (`files > 0`) проверяется `index-compatible`:
   - `index_format_version`
   - `embedding_model_id`
   - `embedding_dim`
   - `ann_version`
4. При несовместимости индекса:
   - `auto_index=true` -> `reindex`
   - `auto_index=false` -> стабильная ошибка `reindex required`

## Матрица

| DB state | Schema-compatible | Index-compatible | auto_index | Action |
|---|---|---|---|---|
| `schema_version > supported` | No | N/A | any | `hard-fail` |
| `files = 0` | Yes | N/A | `false` | `index-not-ready` error |
| `files = 0` | Yes | N/A | `true` | `index` |
| `files > 0`, metadata match | Yes | Yes | any | `continue` |
| `files > 0`, metadata mismatch | Yes | No | `false` | `reindex required` error |
| `files > 0`, metadata mismatch | Yes | No | `true` | `semantic-index --reindex` path |

## Покрытие тестами

- `crates/core/src/engine/compatibility.rs`:
  - `compatibility_matrix_fresh_db_is_compatible`
  - `compatibility_matrix_legacy_index_requires_reindex`
  - `compatibility_matrix_matching_meta_is_compatible`
  - `compatibility_matrix_model_mismatch_requires_reindex`
  - `preflight_rejects_future_schema_version`
- `crates/core/src/engine.rs`:
  - `future_schema_version_hard_fails_without_meta_writes`

