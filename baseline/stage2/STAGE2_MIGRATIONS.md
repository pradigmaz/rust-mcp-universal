# Stage 2: DB Migrations and Durability

Дата фиксации: 2026-03-03  
Репозиторий: `rust-mcp-universal`

## Что сделано

1. Введён versioned migration runner:
   - таблица `schema_migrations(id, name, applied_at_utc)`;
   - монотонные ID (строгая проверка порядка плана);
   - идемпотентный повторный запуск (пропуск уже применённых id).
2. Добавлен pre-migration backup:
   - `index.db` + best-effort `-wal/-shm`;
   - путь: `<db_parent>/migration_backups/index.pre_migration.v<from>_to_v<to>.<ts>.*`.
3. Запрещён silent-downgrade:
   - если `MAX(schema_migrations.id)` в БД больше поддерживаемого бинарём, миграции завершаются hard-fail.
4. Подтверждено восстановление после прерванной миграции:
   - миграции выполняются транзакционно по одной;
   - failed migration не записывается в `schema_migrations`;
   - повторный запуск корректно продолжает с нужного шага.

## Изменённые файлы

- `crates/core/src/engine/schema.rs`
- `crates/core/src/engine.rs`
- `crates/core/src/engine/compatibility.rs`
- `crates/core/src/engine/indexing/run.rs`
- `crates/core/src/engine/indexing/post.rs`
- `crates/core/src/vector_rank.rs`
- `crates/cli/src/commands/query.rs`

## Ключевые тесты Stage 2

В `crates/core/src/engine/schema.rs`:

- `migration_runner_applies_n_to_n_plus_one`
- `migration_runner_is_idempotent_on_repeat_run`
- `migration_runner_creates_premigration_backup_for_existing_db`
- `migration_runner_forbids_silent_downgrade`
- `migration_runner_recovers_after_interrupted_migration`

## Прогоны

- `cargo check --workspace` — OK
- `cargo test -p rmu-core -p rmu-cli -p rmu-mcp-server` — OK
- `cargo clippy -p rmu-core --all-targets -- -D warnings` — OK

