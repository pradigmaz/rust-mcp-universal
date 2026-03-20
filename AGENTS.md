# Repository Guidelines

## Project Structure & Module Organization
This workspace contains three Rust crates:

- `crates/core`: indexing, retrieval, ranking, schema, and shared domain models.
- `crates/cli`: `rmu-cli` command surface (args, output, validation, query/index commands).
- `crates/mcp-server`: MCP/JSON-RPC server exposing the same core capabilities.

Tests live both inline (`mod tests`) and as integration suites under `crates/*/tests` (for example `cli_contract.rs`, `engine_contracts.rs`). JSON schemas for tool outputs are in `schemas/`. Benchmark baselines and rollout artifacts are in `baseline/`.

## Build, Test, and Development Commands
- `cargo build --release -p rmu-cli -p rmu-mcp-server`  
  Build production binaries.
- `cargo test -p rmu-core -p rmu-cli -p rmu-mcp-server`  
  Run unit and integration tests across the workspace.
- `cargo clippy -p rmu-core -p rmu-cli -p rmu-mcp-server --all-targets -- -D warnings`  
  Enforce lint-clean code (matches project gate expectations).
- `cargo run --locked -p rmu-cli -- --project-path . --json status`  
  Quick local sanity check of CLI + core wiring.
- `cargo run --locked -p rmu-cli -- --project-path . --json query-benchmark --dataset baseline/stage0/query_benchmark_dataset.json --semantic --k 10 --limit 20 --max-chars 12000 --max-tokens 3000`  
  Run retrieval benchmark in the same shape used by CI/regression docs.

## Coding Style & Naming Conventions
Use Rust 2024 (`rust-version = 1.85`) and standard `rustfmt` formatting (4-space indentation). Follow idiomatic naming: `snake_case` for functions/modules/files, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants. Keep public API types centralized via `crates/core/src/model`. Treat `unsafe` as exceptional; workspace lint policy sets `unsafe_code = "warn"`.

## Testing Guidelines
Add tests with every behavior change. Prefer focused unit tests near implementation, plus integration coverage for contracts:

- CLI behavior: `crates/cli/tests/cli_contract/...`
- Core retrieval/indexing contracts: `crates/core/tests/engine_contracts/...`
- MCP tool/protocol contracts: `crates/mcp-server/src/*tests*`

Test names should describe observable behavior (for example `ignores_non_json_flags`).

## Commit & Pull Request Guidelines
Current branch history is empty, so no established commit pattern exists yet. Use Conventional Commit style moving forward (`feat:`, `fix:`, `chore:`), imperative mood, and scoped subjects when useful.

For PRs, include:
- clear problem/solution summary;
- linked issue (if applicable);
- test/lint commands run and results;
- benchmark impact when retrieval/ranking logic changes (attach `query-benchmark` output).
