# Investigation Stage 0 Freeze

Дата: 2026-03-24

## Что зафиксировано

- public surface: `preflight`, `symbol_body`, `route_trace`, `constraint_evidence`, `concept_cluster`, `divergence_report`, `investigation-benchmark`
- bounded source scope: Rust, Python, TypeScript/JavaScript, SQL/prisma-like schema sources
- additive contract rule: старые CLI/MCP обязательные поля не ломаются
- benchmark input: `baseline/investigation/stage0/investigation_dataset.json`
- benchmark thresholds: `baseline/investigation/stage9/thresholds.json`

## Fixture baseline

Основной fixture для investigation wave:

- `baseline/investigation/fixtures/mixed_app/src/services/origin_service.rs`
- `baseline/investigation/fixtures/mixed_app/legacy/origin_service.py`
- `baseline/investigation/fixtures/mixed_app/migrations/001_create_origins.sql`
- `baseline/investigation/fixtures/mixed_app/web/origin_client.ts`

## Acceptance focus

- deterministic `preflight`
- bounded adapters without silent unsupported fallbacks
- divergence severity driven by evidence, not by LLM heuristics
- machine-readable benchmark report with thresholds verdict
