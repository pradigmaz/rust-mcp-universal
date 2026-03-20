# Stage 1: Index Versions and Compatibility

Дата фиксации: 2026-03-03  
Репозиторий: `rust-mcp-universal`

## Что реализовано

- Добавлены ключи в `meta`:
  - `schema_version`
  - `index_format_version`
  - `embedding_model_id`
  - `embedding_dim`
  - `ann_version`
- Реализован preflight `schema-compatible`:
  - `db schema_version > supported` -> `hard-fail` до любых schema writes.
- Реализован `index-compatible` check (отдельно от schema-compatible):
  - mismatch/invalid metadata -> `reindex required`.
- Реализована политика `binary x db`:
  - `migrate` (schema migrate + metadata reconcile),
  - `reindex` (при index incompatibility и `auto_index=true`),
  - `hard-fail` (при future schema).

## Артефакты

- Матрица совместимости:
  - `baseline/stage1/BINARY_DB_COMPATIBILITY_MATRIX.md`
- Код:
  - `crates/core/src/engine/compatibility.rs`
  - `crates/core/src/engine.rs`
  - `crates/core/src/engine_brief.rs`
  - `crates/core/src/engine/indexing/post.rs`
  - `crates/core/src/engine/indexing/run.rs`
  - `crates/cli/src/commands/query.rs`

## Проверки

- Пройдено:
  - `cargo check --workspace`
  - `cargo test -p rmu-core -p rmu-cli -p rmu-mcp-server`
- Не пройдено полностью:
  - `cargo clippy -p rmu-core -p rmu-cli -p rmu-mcp-server --all-targets -- -D warnings`
  - Причина: legacy `too_many_arguments` в `crates/cli/src/commands/query.rs` (не часть Stage 1 изменений).

## Ключевые тесты Stage 1

- `crates/core/src/engine/compatibility.rs`:
  - `compatibility_matrix_fresh_db_is_compatible`
  - `compatibility_matrix_legacy_index_requires_reindex`
  - `compatibility_matrix_matching_meta_is_compatible`
  - `compatibility_matrix_model_mismatch_requires_reindex`
  - `preflight_rejects_future_schema_version`
- `crates/core/src/engine.rs`:
  - `future_schema_version_hard_fails_without_meta_writes`

