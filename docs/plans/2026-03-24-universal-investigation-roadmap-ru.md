# План внедрения универсального investigation-слоя поверх RMU (чек-лист)

Дата: 2026-03-24  
Версия: draft v1  
Режим: additive, offline-first, без ломки CLI/MCP контракта

## Статус на 2026-03-24

Уже реализовано в коде:

- [x] Добавлен v1 `investigation` layer в `rmu-core` поверх текущего retrieval/navigation.
- [x] Добавлены canonical types: `ConceptSeed`, `ImplementationVariant`, `RouteSegment`, `ConstraintEvidence`, `DivergenceReport` и связанные result-объекты.
- [x] Добавлены engine-capabilities: `symbol_body`, `route_trace`, `constraint_evidence`, `concept_cluster`, `divergence_report`.
- [x] Добавлены отдельные CLI команды и MCP tools без ломки старых contracts.
- [x] Исправлен ключевой binary/db compatibility drift: runtime compatibility теперь использует актуальную schema migration version.
- [x] Добавлена structured compatibility diagnostics в CLI и MCP вместо неструктурированного hard-fail.
- [x] Добавлены базовые тесты core/CLI/MCP для нового investigation surface.

### Что сделано в этой волне

- [x] Добавлен общий test-only fixture helper для investigation contract tests CLI/MCP.
- [x] Закрыты CLI contracts для `preflight` и усилен contract coverage для `investigation-benchmark`.
- [x] Закрыт MCP contract coverage для `preflight`, включая raw `tools/call` путь.
- [x] Добавлен schema contract coverage для `schemas/preflight_status.schema.json` и `schemas/investigation_benchmark_report.schema.json`.
- [x] Усилен `divergence_report`: зафиксированы `divergence_signals`, `shared_evidence` и severity escalation rules тестами.

Что ещё остаётся частично:

- [ ] `symbol_body` и `constraint_evidence`.
  Сделано: для `symbol_body` есть bounded v1 coverage для `TypeScript/JavaScript` body extraction; для `constraint_evidence` heuristic-only classifier заменён на ordered language-adapter registry (`Python SQLAlchemy/Alembic-like`, `TypeScript ORM/schema-like`, `Rust sqlx/diesel-like`, `SQL/Prisma-like`), сохранены compatibility aliases и явное `strong`/`weak` evidence.
  Осталось: усилить именно `symbol_body` сверх bounded extraction до более сильной language-specific semantic модели.
- [ ] `route_trace` и `divergence_report`.
  Сделано: есть richer route evidence (`anchor_symbol`, `source_span`, `relation_kind`, `source_kind`), proxy-evidence divergence axes, top-level `summary`, `recommended_followups` и severity model.
  Осталось: усилить route extraction и divergence analysis сверх текущего heuristic/evidence-first уровня, но без claim на full semantic equivalence.

Уже закрыто из прежних блокеров:

- [x] Windows fresh-start / stale-process protection.
  Сделано: есть отдельный `preflight`/health path, stale-process probe, structured `PreflightStatus`, fresh launcher и stage9 runtime KPI-артефакт с `startup_compat_success_rate = 1.0`, `stale_server_detection = deterministic`, `stale_runtime_guard = deterministic`.
- [x] Eval dataset / benchmark surface / rollout guidance.
  Сделано: есть `baseline/investigation/stage0/*` как freeze, `baseline/investigation/stage9/*` как operational eval layer, CLI benchmark/eval mode, machine-readable compare/diff reports, dedicated CI regression workflow, operator-facing `STAGE9_EVAL_GATES.md` и accepted `latest_report.json` с `threshold_verdict.passed = true`.
  Осталось: при следующих heuristics batches обновлять `baseline_report.json` только после явного принятия нового accepted baseline.

Что делать следующим:

1. Досвести documentation surface: обновить `README.md` под investigation surface и затем свести README с уже существующим investigation/MCP/support/examples/troubleshooting doc в консистентное состояние.
2. Усилить уже существующие `TypeScript/JavaScript` и `Rust sqlx/diesel-like` adapters: перейти от bounded heuristics к более сильной language-specific semantic модели.
3. Усилить `route_trace`/`divergence_report`: сохранить текущий evidence-first surface, но поднять точность и language awareness route extraction.
4. Поддерживать `baseline/investigation/stage0/*` и `stage9/*` в accepted-состоянии: обновлять `latest_report.json` при изменении dataset/thresholds, а `baseline_report.json` только после явного принятия нового baseline.

## 1) Цель

Добавить в `rust-mcp-universal` универсальный слой анализа, который работает поверх текущего retrieval/navigation ядра и закрывает четыре класса задач:

1. `symbol/body surface`  
   Быстро отдавать body/snippet по символу или по найденному implementation variant без ухода в shell.
2. `traceable route surface`  
   Давать короткий машинно-собранный маршрут вида `UI/API -> endpoint -> service -> query -> test`.
3. `constraint evidence surface`  
   Показывать инварианты из модели, миграций, индексов и schema-like источников как first-class evidence.
4. `concept divergence analysis`  
   Собирать и сравнивать несколько реализаций одного conceptual query, а не только shortlist файлов.

Итоговая цель: превратить RMU из хорошего retrieval/navigation engine в investigation-grade engine для больших legacy-репозиториев.

## 2) Что именно считается универсальным функционалом

Универсальным считается только то, что формулируется без привязки к конкретному репозиторию или домену.

Подходит:

- поиск нескольких code paths, реализующих один и тот же концепт;
- сравнение реализаций на уровне body/rules/dependencies/tests/constraints;
- извлечение нормализованного маршрута между слоями системы;
- surfacing инвариантов из schema/migration/model источников;
- evidence-first explainability без LLM-only догадок.

Не подходит:

- эвристики под `attendance`, `labs`, `origin resolution` или любой другой конкретный домен;
- жёсткая привязка к именам каталогов/модулей одного проекта;
- любые специальные правила, которые бесполезны вне одного репо.

## 3) Термины и целевые сущности

### Базовые доменно-нейтральные сущности

- [x] `ConceptSeed`
  - запрос или якорь, от которого строится analysis cluster;
  - может быть `query`, `path`, `symbol`, `endpoint`.
- [x] `ImplementationVariant`
  - один путь реализации концепта;
  - содержит entrypoint, body anchor, зависимости, route segments, evidence.
- [x] `RouteSegment`
  - типизированный шаг маршрута;
  - примеры типов: `ui_hook`, `api_client`, `endpoint`, `service`, `crud`, `query`, `test`, `migration`, `constraint_source`.
- [x] `ConstraintEvidence`
  - объект инварианта;
  - содержит тип (`unique`, `partial_index`, `fk`, `check`, `migration_rule`, `model_constraint`), источник, уровень уверенности.
- [x] `DivergenceReport`
  - результат сравнения нескольких `ImplementationVariant`;
  - показывает, где paths совпадают, где расходятся, чего не хватает по evidence.

### Минимальный public surface v1

- [x] `symbol_body`
- [x] `route_trace`
- [x] `constraint_evidence`
- [x] `concept_cluster`
- [x] `divergence_report`

## 4) Нецели v1

- [x] Не делать auto-fix или code rewrite.
- [x] Не добавлять repo-specific domain adapters.
- [x] Не обещать semantic equivalence checking для всех языков.
- [x] Не тащить LLM в критический путь детерминированного evidence extraction.
- [x] Не ломать существующие `search_candidates`, `context_pack`, `query_report`, `symbol_lookup_v2`, `related_files_v2`, `call_path`.
- [x] Не маскировать runtime-проблемы старого бинаря или битой DB под "просто degraded answer" без явного статуса.

## 5) Ограничения и инварианты

- [x] Все новые MCP tools и CLI команды добавляются additive-способом.
- [ ] Все новые result schemas versioned и machine-readable.
- [x] Старые contracts продолжают работать без изменения обязательных полей.
- [x] `privacy_mode` применяется ко всем новым ответам и ошибкам.
- [ ] `migration_mode=off` не должен давать частично-валидные результаты на schema mismatch.
- [ ] Любой analysis answer должен содержать evidence-first basis; без evidence допускается только явно помеченный low-confidence fallback.
- [x] Все новые инструменты должны корректно работать в offline-first режиме.
- [x] Для unsupported language/source tool должен возвращать явный capability status, а не молча выдавать пустоту.

## 6) Почему roadmap начинается не с новых tools

Сначала нужно закрыть operational blocker:

- [x] устранить рассинхрон `live binary` vs `.rmu/index.db` migration support;
- [ ] гарантировать, что MCP-сервер на Windows стартует свежим бинарём;
- [x] сделать диагностику compat mismatch частью нормального operator surface.

Без этого investigation-функции останутся теоретическими: advanced path будет падать раньше, чем дойдёт до новой логики.

## 7) KPI и acceptance gates

### Runtime/stability

Acceptance gates (измеримо сейчас):

- [x] `future_db_hard_fail = deterministic` без write side effects.

Status note:

- локальный `cargo run --locked -p rmu-cli -- --project-path . --json preflight` возвращает `status = ok|warning` в зависимости от наличия живого `rmu-mcp-server.exe`, при этом `errors = []` на supported runtime paths;
- core/CLI tests подтверждают hard-fail на future schema без meta writes;
- `baseline/investigation/stage9/runtime_report.json` даёт `startup_compat_success_rate = 1.0` и `stale_server_detection = deterministic`;
- Windows stale-process path подтверждён и через `preflight_json_detects_running_mcp_server_via_probe_binary_path`, и через `scripts/generate-runtime-kpi-report.ps1`.

Target KPI (измеримо сейчас):

- [x] `startup_compat_success_rate = 1.0` на поддерживаемых migration версиях.
- [x] `stale_server_detection = deterministic` на Windows.

### Symbol/body

Acceptance gates (измеримо сейчас):

- [x] `symbol_body_supported_success >= 0.90` на curated dataset.
- [x] `symbol_body_latency_p95_ms <= 250`.

Status note:

- текущий benchmark даёт `symbol_body pass_rate = 1.0` на 3 labeled кейсах;
- текущий `symbol_body latency_p95 = 0.348ms`, `body_request_p95_budget_ms = 0.343ms`, `body_request_p95_ratio = 1.015`;
- текущий `body_anchor_precision = 1.0`;
- coverage по supported langs теперь подтверждён на labeled `Rust + TypeScript + Python` matrix текущего `mixed_app`.

Target KPI (измеримо сейчас):

- [x] `body_anchor_precision >= 0.95`
- [x] `body_request_p95 <= baseline_navigation_p95 * 1.20`

Правило пометки `[ ]` / `[x]` для KPI:

- [ ] KPI не переводится в `[x]` автоматически только потому, что реализация существует.
- [ ] `[x]` ставится только если есть acceptance evidence по соответствующему measurement path.
- [ ] если implementation есть, но measurement path неполный или baseline невалиден, KPI остаётся `[ ]` и сопровождается status note.

### Route trace

Acceptance gates (измеримо сейчас):

- [x] `route_trace_case_pass_rate >= 0.80` на curated dataset.

Status note:

- текущий benchmark даёт `route_trace pass_rate = 1.0` на 1 кейсе;
- текущий `route_trace latency_p95 = 44.385ms`;
- acceptance закрыт после bounded mixed auto-index + indexed-only adjacent route assembly;
- текущие `route_trace_success@1 = 1.0`, `route_trace_success@3 = 1.0`, `segment_type_precision = 1.0`.

Target KPI (измеримо сейчас):

- [x] `route_trace_success@1 >= 0.80`
- [x] `route_trace_success@3 >= 0.92`
- [x] `segment_type_precision >= 0.90`

### Constraint evidence

Acceptance gates (измеримо сейчас):

- [x] `constraint_evidence_case_pass_rate >= 0.85` на curated dataset.

Status note:

- текущий benchmark даёт `constraint_evidence pass_rate = 1.0` на 2 labeled кейсах;
- текущий `constraint_evidence latency_p95 = 46.608ms`;
- acceptance закрыт на curated case по `index_constraint` и `strong_constraint_present`;
- harness теперь считает `constraint_evidence_precision = 1.0` и `constraint_source_recall = 1.0` на текущем language-adapter dataset.

Target KPI (измеримо сейчас):

- [x] `constraint_evidence_precision >= 0.90`
- [x] `constraint_source_recall >= 0.80` на language-adapter datasets

### Divergence analysis

Acceptance gates (измеримо сейчас):

- [x] `divergence_case_pass_rate >= 0.85` на curated dataset.

Status note:

- текущий benchmark даёт `divergence_report pass_rate = 1.0` на 1 кейсе;
- текущий `divergence_report latency_p95 = 154.195ms`;
- acceptance закрыт по `capability_status`, `min_divergence_axes` и `expected_severity` на curated case;
- harness теперь считает `variant_recall@3 = 1.0`, `divergence_signal_precision = 1.0`, `false_positive_divergence_rate = 0.0`.

Target KPI (измеримо сейчас):

- [x] `variant_recall@3 >= 0.80`
- [x] `divergence_signal_precision >= 0.85`
- [x] `false_positive_divergence_rate <= 0.10`

### Contract and UX

Acceptance gates (измеримо сейчас):

- [x] `existing_contract_breaks = 0`
- [x] `privacy_leaks = 0`

Status note:

- contract shape проверяется отдельными CLI/MCP/schema suites и они проходят после acceptance fixes;
- privacy имеет machine-readable measurement path через `privacy_failures`, текущий `latest_report` даёт `privacy_failures = 0`;
- benchmark report теперь считает `explain_evidence_coverage`; текущий `latest_report` даёт `1.0` по всем tool metrics.

Target KPI (измеримо сейчас):

- [x] `explain/evidence coverage = 1.0` для top-level entities нового API

## 8) Целевая архитектура

```text
Query / Path / Symbol / Endpoint
 -> Concept seed normalization
 -> Candidate expansion
    -> retrieval shortlist
    -> symbol neighbors
    -> related files
    -> route anchors
 -> Body surface
 -> Route assembly
 -> Constraint evidence extraction
 -> Variant grouping
 -> Divergence scoring
 -> Context/report packaging
 -> MCP/CLI output
```

### Внутренние слои

- [x] `Seed layer`
  - нормализует вход.
- [x] `Expansion layer`
  - поднимает candidate set из retrieval/navigation.
- [x] `Evidence layer`
  - body, route, constraints, tests, symbol refs.
- [x] `Variant layer`
  - собирает implementation variants.
- [x] `Comparison layer`
  - считает divergence.
- [x] `Presentation layer`
  - отдаёт stable MCP/CLI payload.

## 9) Рекомендуемая стратегия внедрения

Рекомендуемый порядок:

1. Runtime hardening.
2. Canonical internal model.
3. `symbol_body`.
4. `route_trace`.
5. `constraint_evidence`.
6. `concept_cluster`.
7. `divergence_report`.
8. Dataset + eval gates + rollout.

Причина:

- `divergence_report` без body/route/constraint surfaces превратится в тонкую обвязку над file hits;
- `constraint_evidence` без стабильного internal model даст ad hoc payloads;
- runtime instability убьёт ценность любой следующей фазы.

## 10) Пошаговый roadmap

### Этап 0. Freeze направления и baseline

- [x] Зафиксировать problem statement и non-goals.
- [x] Зафиксировать текущий tool surface как baseline.
- [x] Собрать baseline latency по существующим navigation/report tools.
- [x] Собрать curated investigation dataset:
  - [x] `symbol-body` cases
  - [x] `route-trace` cases
  - [x] `constraint-evidence` cases
  - [x] `divergence` cases
- [x] Зафиксировать schema для будущих benchmark/eval артефактов.

Критерий этапа:

- [x] Есть freeze-документ и baseline-артефакты.

Артефакты:

- [x] `baseline/investigation/stage0/STAGE0_FREEZE.md`
- [x] `baseline/investigation/stage0/contract_freeze_manifest.json`
- [x] `baseline/investigation/stage0/navigation_latency_baseline.json`
- [x] `baseline/investigation/stage0/investigation_dataset.json`

### Этап 1. Runtime hardening и binary/db compatibility

- [x] Проверить, почему live MCP path видит старый максимум migration id.
- [x] Укрепить fresh-start launcher flow на Windows.
- [x] Добавить operator-facing диагностику:
  - [x] running binary version
  - [x] supported schema max
  - [x] db schema max
  - [x] stale process warning
- [x] Добавить отдельный health/preflight путь для MCP и CLI.
- [x] Убедиться, что incompatibility всегда отдаётся одинаково и рано.

#### План закрытия

1. [x] Добавить в `preflight` и compatibility diagnostics поле `running_binary_version`.
2. [x] Ввести self-stale runtime guard: текущий MCP-процесс считается устаревшим, если бинарник на диске был rebuilt после старта процесса.
3. [x] Перевести stale-runtime case из warning-only в ранний structured hard-fail для MCP tool path.
4. [x] Убрать статическую подстановку stale-флагов из compatibility error payloads; использовать только реальные probe/preflight данные.
5. [x] Добить contract/e2e coverage для future-schema и stale-after-rebuild сценариев, затем обновить чекбоксы Этапа 1.

Критерий этапа:

- [x] Нельзя получить тихий downgrade.
- [x] Нельзя случайно работать старым процессом после rebuild.
  Примечание: stale runtime теперь детектируется через `running_binary_stale` и блокируется ранним structured compatibility fail в MCP runtime path.

Затрагиваемые зоны:

- [x] `crates/core/src/engine/schema/migrations.rs`
- [x] `crates/core/src/engine/lifecycle.rs`
- [x] `crates/cli/src/*`
- [x] `crates/mcp-server/src/*`
- [x] `scripts/rmu-mcp-server-fresh.cmd`

### Этап 2. Canonical investigation model

- [x] Добавить новые internal model types:
- [x] `ConceptSeed`
  - [x] `ImplementationVariant`
  - [x] `RouteSegment`
  - [x] `ConstraintEvidence`
  - [x] `DivergenceSignal`
  - [x] `DivergenceReport`
- [x] Зафиксировать их JSON-форму и инварианты.
- [x] Добавить serde/tests/schema fixtures.
- [x] Продумать privacy sanitization для новых объектов.
  Статус: done.
  Причина: privacy-mode теперь санитизирует investigation-специфичные content/path fields, а contract покрыт unit/MCP tests.
  Evidence: `crates/core/src/privacy.rs`, `crates/mcp-server/src/rpc_tools_tests/report/investigation.rs`.

Критерий этапа:

- [x] Есть стабильные типы, на которые могут опираться core, CLI и MCP.
  Статус: done.
  Причина: типы экспортируются из `rmu_core` и используются в engine/CLI/MCP surface.
  Evidence: `crates/core/src/model/core/types/investigation.rs`, `crates/core/src/model/core/types.rs`, `crates/core/src/lib.rs`.

Затрагиваемые зоны:

- [x] `crates/core/src/model/core/types/*`
- [x] `crates/core/src/lib.rs`
- [x] `schemas/*`
  Статус: done.
  Причина: добавлены versioned result schemas и MCP envelope schemas для investigation surface.
  Evidence: `schemas/symbol_body.schema.json`, `schemas/constraint_evidence.schema.json`, `schemas/concept_cluster.schema.json`, `schemas/route_trace.schema.json`, `schemas/divergence_report.schema.json`.

### Этап 3. Symbol body surface v1

- [x] Спроектировать `symbol_body` как отдельный engine capability.
- [x] Определить поддерживаемые sources v1:
  - [x] Rust
  - [x] Python
  - [x] TypeScript/JavaScript
  Статус: done.
  Причина: support matrix и benchmark dataset/report подтверждают `Rust + Python + TypeScript`.
  Evidence: `crates/core/src/engine/investigation/body.rs`, `baseline/investigation/stage0/investigation_dataset.json`, `baseline/investigation/stage9/latest_report.json`, `docs/investigation-surface.md`.
- [x] Определить fallback-иерархию:
  - [x] exact symbol span
  - [x] nearest indexed lines
  - [x] chunk excerpt around anchor
  Статус: done.
  Причина: `symbol_body` использует deterministic ladder и возвращает machine-readable `resolution_kind`.
  Evidence: `crates/core/src/engine/investigation/body.rs`, `crates/core/src/engine/investigation/tests.rs`.
- [x] Добавить в ответ:
  - [x] `path`
  - [x] `symbol`
  - [x] `language`
  - [x] `body`
  - [x] `start_line`
  - [x] `end_line`
  - [x] `source_kind`
  - [x] `confidence`
  Статус: done.
  Причина: поля присутствуют в стабильном nested contract: `items[].anchor.{path,symbol,language}`, `items[].body`, `items[].span.{start_line,end_line}`, `items[].source_kind`, `items[].confidence`.
  Evidence: `crates/core/src/model/core/types/investigation.rs`, `schemas/symbol_body.schema.json`, `docs/investigation-surface.md`.
- [x] Прописать ambiguity handling:
  - [x] multiple exact symbols
  - [x] partial matches
  - [x] unsupported language
  Статус: done.
  Причина: добавлен `ambiguity_status`, partial matches используются только при отсутствии exact matches, unsupported sources отражаются через `capability_status` + `unsupported_sources`.
  Evidence: `crates/core/src/engine/investigation/body.rs`, `crates/core/src/engine/investigation/tests.rs`.
- [x] Добавить MCP handler и CLI command.

Критерий этапа:

- [x] На supported языках можно стабильно получить body/snippet без shell.
  Статус: done.
  Причина: есть core/CLI/MCP tests и acceptance benchmark по curated dataset.
  Evidence: `crates/core/src/engine/investigation/tests.rs`, `crates/cli/tests/cli_contract/investigation.rs`, `crates/mcp-server/src/rpc_tools_tests/report/investigation.rs`, `baseline/investigation/stage9/latest_report.json`.

Затрагиваемые зоны:

- [x] `crates/core/src/engine/investigation/*`
- [x] `crates/core/src/model/*`
- [x] `crates/cli/src/args.rs`
- [x] `crates/cli/src/commands/query/*`
- [x] `crates/mcp-server/src/rpc_tools/handlers/*`
- [x] `crates/mcp-server/src/rpc_tools/registry/*`
- [x] `schemas/*`
  Статус: done.
  Причина: фактическая реализация этапа 3 живёт в `engine/investigation/*`, а contract layer закрыт через model/CLI/MCP/schema/tests.

### Этап 4. Route trace surface v1

- [x] Спроектировать `route_trace` поверх текущих retrieval/navigation примитивов.
- [x] Добавить typed segment classifier:
  - [x] `ui`
  - [x] `api_client`
  - [x] `endpoint`
  - [x] `service`
  - [x] `crud`
  - [x] `query`
  - [x] `test`
  - [x] `migration`
  - [x] `unknown`
- [x] Переиспользовать `call_path`, но поднять abstraction level с file hops до typed route.
- [x] Добавить evidence для каждого segment:
  - [x] anchor symbol
  - [x] source path
  - [x] line/column
  - [x] relation kind
- [x] Добавить truncation/branching policy:
  - [x] best route
  - [x] alternate routes
  - [x] unresolved gap markers

Критерий этапа:

- [x] Можно отдать короткий маршрут между слоями системы, а не только file graph path.

Затрагиваемые зоны:

- [ ] `crates/core/src/engine/navigation/call_path.rs`
- [x] новый route builder в `crates/core/src/engine/navigation/`
- [x] model/CLI/MCP schemas

### Этап 5. Constraint evidence surface v1

- [x] Спроектировать `constraint_evidence` как отдельный extraction layer.
- [x] Определить универсальные типы evidence:
  - [x] `model_constraint`
  - [x] `migration_constraint`
  - [x] `index_constraint`
  - [x] `ddl_like_hint`
  - [x] `runtime_guard`
- [x] Ввести language-adapter strategy вместо repo-specific эвристик.
- [x] Первая волна adapters:
  - [x] Python SQLAlchemy + Alembic-like
  - [x] TypeScript ORM/schema-like files
  - [x] Rust sqlx/diesel-like patterns, если доступны
- [x] Отделить strong evidence от weak evidence.
- [ ] В ответ добавить:
  - [x] `constraint_kind`
  - [x] `source_kind`
  - [x] `path`
  - [x] `line_start`
  - [x] `line_end`
  - [x] `excerpt`
  - [x] `confidence`
  - [x] `normalized_key`

Критерий этапа:

- [x] RMU умеет возвращать инвариант как объект evidence, а не как случайный файл-кандидат.

### Этап 6. Concept cluster assembly

- [x] Спроектировать `concept_cluster`.
- [x] Источники expansion:
  - [x] retrieval shortlist
  - [x] symbol neighbors
  - [x] route trace anchors
  - [x] related files
  - [x] tests
  - [x] constraint evidence
  Статус: done для `heuristic_v2` scope.
  Причина: `concept_cluster` теперь не только де-факто использует эти источники, но и сериализует явную `expansion_policy` в `cluster_summary`, включая `initial_sources`, `enrichment_sources`, `feedback_sources`, `route_trace_reused = true`, `candidate_pool_limit_multiplier = 3`, `dedup_unit` и deterministic `tie_break_order`.
  Evidence: `crates/core/src/engine/investigation/cluster.rs`, `crates/core/src/engine/investigation/cluster_policy.rs`, `crates/core/src/model/core/types/investigation_cluster.rs`.
- [x] Определить grouping logic для `ImplementationVariant`.
  Статус: done.
  Причина: `ImplementationVariant` уже канонически определён и реально используется как bundle из entry/body anchors, route, constraints, related tests, confidence и gaps.
  Evidence: `crates/core/src/model/core/types/investigation_cluster.rs`, `crates/core/src/engine/investigation/cluster.rs`.
- [x] Определить scoring signals:
  - [x] lexical proximity
  - [x] semantic proximity
  - [x] route centrality
  - [x] symbol overlap
  - [x] constraint overlap
  - [x] test adjacency
  Статус: done для `heuristic_v2` scope.
  Причина: tool сериализует явные additive signals, `semantic_state`, `score_model = heuristic_v2` и `score_breakdown`, а `final confidence` считается по explainable formula и валидируется benchmark gates как explainable deterministic ranking model. Learned/rerank model в scope этапа не входит.
  Evidence: `crates/core/src/engine/investigation/cluster.rs`, `crates/core/src/engine/investigation/cluster_scoring.rs`, `crates/core/src/model/core/types/investigation_cluster.rs`.
- [x] Определить cutoff и dedup policy.
  Статус: done для heuristic cluster assembly.
  Причина: expansion pool теперь ограничивается `limit * 3`, scoring/dedup выполняется на полном expanded pool, variant dedup идёт по `entry_anchor.path` с deterministic tie-break (`final confidence` -> `constraint_overlap` -> `route_centrality` -> `lexical_proximity` -> stable path sort), а merge/drop помечаются через `merged_duplicate_variant:<path>`.
  Evidence: `crates/core/src/engine/investigation/cluster.rs`, `crates/core/src/engine/investigation/cluster_policy.rs`.

Критерий этапа:

- [x] На один conceptual query tool собирает не просто список файлов, а несколько осмысленных вариантов реализации и ранжирует их по explainable scoring model, где semantic availability явно сериализована, а не скрыта внутри `confidence`.
  Статус: done для текущего acceptance corpus.
  Причина: acceptance dataset теперь покрывает как минимум две разные conceptual topology (`mixed_app` и `adapter_app`), а Stage 9 gates дополнительно валидируют `concept_cluster_case_pass_rate`, `variant_recall_at_3`, `top_variant_precision`, `rank consistency`, `semantic_state coverage`, `semantic fail-open visibility` и low-signal handling.
  Evidence: `baseline/investigation/stage0/investigation_dataset.json`, `baseline/investigation/stage9/thresholds.json`, `crates/cli/src/commands/query/investigation_benchmark_eval.rs`, `docs/investigation-surface.md`.

### Этап 7. Divergence report v1

- [x] Спроектировать `divergence_report` как comparison layer над `concept_cluster`.
- [x] Определить, что считается divergence:
  - [x] разные guards
  - [x] разные predicates
  - [x] разные downstream symbols
  - [x] разные DB entities/queries
  - [x] разное constraint backing
  - [x] разное test backing
  Статус: done для practical v1.
  Причина: оси divergence теперь фиксируются как evidence-first proxies (`guards_and_validators`, `predicate_signatures`, `downstream_symbols`, `db_entities_and_queries`, `constraint_evidence`, `test_coverage`) без доменной кастомизации и без обещания полной semantic equivalence.
- [x] Определить, что считается expected difference, а не багом.
- [x] Добавить `divergence_signal` severity model:
  - [x] `informational`
  - [x] `likely_expected`
  - [x] `suspicious`
  - [x] `high_risk`
- [x] Добавить answer shape:
  - [x] `summary`
  - [x] `variants`
  - [x] `shared_evidence`
  - [x] `divergence_signals`
  - [x] `unknowns`
  - [x] `recommended_followups`
  Статус: done.
  Причина: top-level payload теперь отдаёт `summary`, `variants`, `shared_evidence`, `divergence_signals`, `unknowns`, `missing_evidence` и `recommended_followups`, а schema/CLI/MCP contracts обновлены additive-способом.

Критерий этапа:

- [x] Tool умеет evidence-first объяснить расхождения по концепту без доменной кастомизации, но не обещает полную semantic equivalence across languages.

### Этап 8. Public surfaces: CLI + MCP

- [x] Добавить CLI команды:
  - [x] `symbol-body`
  - [x] `route-trace`
  - [x] `constraint-evidence`
  - [x] `concept-cluster`
  - [x] `divergence-report`
- [x] Добавить MCP tools с теми же возможностями.
- [x] Определить, что из этого должно также интегрироваться в:
  - [x] `context_pack`
  - [x] `query_report`
  - [x] `agent_bootstrap`
- [x] Для интеграции выбрать additive-поля, а не retrofit ломающий старые schemas.

Статус: done.
Причина: standalone investigation tools сохранены как primary surfaces, а встроенная интеграция выполнена через lightweight additive enrichment без дублирования heavy payloads.
Evidence: `query_report` теперь отдаёт optional `investigation_summary`, `context_pack` отдаёт optional `investigation_hints`, а `agent_bootstrap` использует `query_bundle.report.investigation_summary` как транзитивный путь без нового top-level investigation object.

Критерий этапа:

- [x] Новый функционал доступен как самостоятельные tools без ломки старых flows.

### Этап 9. Eval harness и release gates

- [x] Добавить investigation benchmark mode.
- [x] Ввести отдельные gold datasets:
  - [x] symbol body
  - [x] route trace
  - [x] constraint evidence
  - [x] divergence
- [x] Добавить machine-readable diff отчёты.
- [x] Ввести fail-fast gates по:
  - [x] precision/recall
  - [x] latency
  - [x] privacy
  - [x] unsupported-source behavior
- [x] Ввести scoring/ranking gates по `concept_cluster`:
  - [x] top-variant precision
  - [x] rank consistency
  - [x] semantic-state coverage
  - [x] semantic fail-open visibility
  - [x] no false semantic penalty on low-signal queries
  Статус: done для `heuristic_v2` scope.
  Причина: benchmark/eval/thresholds теперь считают и валидируют ranking/scoring degradation для `concept_cluster`, включая `concept_cluster_case_pass_rate`, `variant_recall_at_3`, explainable semantic availability, отдельный fail-open dataset case и compare-mode regression diff against trusted `baseline_report.json`.
- [x] Добавить CI regression workflow.

Критерий этапа:

- [x] Merge блокируется не только на contract/latency/privacy regression, но и на scoring degradation или silent semantic fallback в `concept_cluster` ranking.

### Этап 10. Docs, rollout, operator guidance

- [ ] Обновить `README.md` под новый investigation surface.
- [x] Обновить investigation/MCP docs.
- [x] Описать language support matrix.
- [x] Описать capability caveats.
- [ ] Описать rollout flags и rollback path в отдельном operator-facing doc.
- [x] Подготовить examples:
  - [x] "сравни две реализации концепта"
  - [x] "дай route trace"
  - [x] "покажи backing constraints"
- [x] Подготовить troubleshooting для stale binary / future migration db.

Критерий этапа:

- [ ] Пользователь может понять, когда новый слой применим, а когда нет.

## 11) Зависимости между этапами

- [x] Этап 1 блокирует все остальные.
- [x] Этап 2 блокирует Этапы 3-8.
- [x] Этап 3 и Этап 4 можно делать частично параллельно после Этапа 2.
- [x] Этап 5 зависит от Этапа 2, но не обязан ждать полного завершения Этапа 4.
- [x] Этап 6 зависит от Этапов 3-5.
- [x] Этап 7 зависит от Этапа 6.
- [x] Этап 8 зависит от Этапов 3-7.
- [x] Этап 9 зависит от наличия хотя бы минимального public surface.
- [x] Этап 10 закрывает deliverable только после прохождения gates.

## 12) Риски

### Технические

- [x] `stale binary risk`
  - симптомы: новый код есть в исходниках, но MCP живёт на старом процессе.
- [x] `schema drift risk`
  - разные бинарники видят разный максимум migration id.
- [x] `false divergence risk`
  - tool принимает intentional variation за проблему.
- [x] `weak evidence inflation`
  - runtime guards и naming hints начинают маскироваться под hard constraints.
- [x] `latency blow-up`
  - multi-stage analysis становится слишком тяжёлым.
- [x] `payload bloat`
  - ответы становятся слишком большими для MCP/агентов.

### Продуктовые

- [x] слишком ранняя попытка сделать "умный universal diff" без опоры на evidence primitives;
- [x] размывание scope в сторону UI/reporting до готовности core objects;
- [x] путаница между retrieval explainability и divergence explainability.

## 13) Негативные тесты

- [x] `INV-RT-01`: DB newer than binary -> hard fail без частичной работы.
- [x] `INV-RT-02`: stale server process -> preflight явно показывает рассинхрон.
- [x] `INV-BD-01`: ambiguous symbol body -> deterministic ambiguity payload.
- [x] `INV-BD-02`: unsupported language -> explicit capability status.
- [x] `INV-RT-03`: route trace с missing middle hop -> gap marker, не silent truncation.
- [x] `INV-CE-01`: migration/model disagree -> оба evidence возвращаются отдельно.
- [x] `INV-CE-02`: weak hint без strong backing -> low confidence, не strong evidence.
- [x] `INV-DV-01`: два intentional variants без конфликта -> no high-risk divergence.
- [x] `INV-DV-02`: conflicting downstream query/constraint -> suspicious/high-risk divergence.
- [x] `INV-PR-01`: privacy_mode mask/hash не допускает raw absolute paths.
- [x] `INV-CT-01`: новые MCP tools валидируют params через `-32602`.
- [x] `INV-CT-02`: CLI `--json` сохраняет валидный envelope на всех ошибках.

## 14) Рекомендуемая файловая карта внедрения

### Core

- [ ] `crates/core/src/model/core/types/navigation.rs`
- [x] `crates/core/src/model/core/types/report.rs`
- [x] новые типы в `crates/core/src/model/core/types/*`
- [x] `crates/core/src/engine/navigation/*`
- [x] новый слой `crates/core/src/engine/investigation/*`
- [x] `crates/core/src/engine/query/*` при необходимости интеграции

### CLI

- [x] `crates/cli/src/args.rs`
- [x] `crates/cli/src/commands/query/*`
- [x] `crates/cli/src/output.rs`

### MCP

- [x] `crates/mcp-server/src/rpc_tools/handlers/*`
- [x] `crates/mcp-server/src/rpc_tools/registry/*`
- [x] `crates/mcp-server/src/rpc_tools_tests/*`

### Tests

- [x] `crates/core/tests/engine_contracts/*`
- [x] `crates/cli/tests/cli_contract/*`
- [x] `crates/mcp-server/src/rpc_tools_tests/*`

### Artifacts

- [x] `baseline/investigation/*`
- [x] `schemas/*`
- [x] `docs/plans/*`

## 15) Артефакты по этапам

- [x] `baseline/investigation/stage0/STAGE0_FREEZE.md`
- [ ] `baseline/investigation/stage1/STAGE1_RUNTIME_HARDENING.md`
- [ ] `baseline/investigation/stage2/STAGE2_CANONICAL_MODEL.md`
- [ ] `baseline/investigation/stage3/STAGE3_SYMBOL_BODY.md`
- [ ] `baseline/investigation/stage4/STAGE4_ROUTE_TRACE.md`
- [ ] `baseline/investigation/stage5/STAGE5_CONSTRAINT_EVIDENCE.md`
- [ ] `baseline/investigation/stage6/STAGE6_CONCEPT_CLUSTER.md`
- [ ] `baseline/investigation/stage7/STAGE7_DIVERGENCE_REPORT.md`
- [ ] `baseline/investigation/stage8/STAGE8_PUBLIC_SURFACES.md`
- [x] `baseline/investigation/stage9/STAGE9_EVAL_GATES.md`
- [ ] `baseline/investigation/stage10/STAGE10_ROLLOUT.md`

## 16) Open questions до старта реализации

- [ ] Делать ли `concept_cluster` как отдельный tool или как internal dependency для `divergence_report`?
- [ ] Делать ли `symbol_body` строго indexed-only или допускать source-read fallback?
- [x] Нужен ли `route_trace` как отдельный tool и как часть `query_report` одновременно?
  Решение: да, но внутри `query_report` только как compact `investigation_summary.route_trace`, а не как полный nested `RouteTraceResult`.
- [ ] Какой минимальный language support обязателен для v1?
- [ ] Где проходит граница между `constraint_evidence` и `runtime_guard`?
- [ ] Нужно ли включать test discovery в v1 или оставить как v1.1?
- [x] Делать ли отдельный `context_mode="investigation"` в `context_pack` после стабилизации новых entities?
  Решение: нет для текущего batch; вместо нового режима используется additive `investigation_hints` поверх существующих `code|design|bugfix` modes.

## 17) Рекомендуемый порядок фактического старта

Если начинать implementation завтра, рекомендованный порядок первых рабочих батчей такой:

1. [x] Runtime hardening и stale-binary diagnosis.
2. [x] Canonical model types + schemas.
3. [x] `symbol_body` minimal end-to-end.
4. [x] `route_trace` minimal end-to-end.
5. [x] `constraint_evidence` для первой language волны.
6. [x] `concept_cluster` над уже существующими primitives.
7. [x] `divergence_report`.
8. [x] eval dataset и gates.

## 18) Критерий завершения всего roadmap

- [x] Новый investigation layer не ломает старые CLI/MCP contracts.
- [x] Есть стабильные tools для body, trace, constraint evidence и divergence.
- [x] На curated datasets достигаются целевые quality thresholds.
- [x] Runtime path устойчив на schema/version/process mismatch.
- [ ] Документация и rollout guidance готовы.
- [x] Пользователь может разбирать legacy-расхождения без shell-first workflow в большинстве supported случаев.

## 19) Следующая волна после текущего v1

Ниже зафиксирован не общий wishlist, а следующий конкретный implementation batch, который должен довести уже внедрённый investigation layer до эксплуатационного состояния.

### 19.1 Runtime hardening до конца

- [x] Добавить отдельный `preflight` surface для CLI и MCP.
- [x] Ввести общий result-object `PreflightStatus` со structured diagnostic-полями:
  - [x] `status`
  - [x] `binary_path`
  - [x] `supported_schema_version`
  - [x] `db_schema_version`
  - [x] `same_binary_other_pids`
  - [x] `stale_process_suspected`
  - [x] `launcher_recommended`
  - [x] `safe_recovery_hint`
- [x] Сделать stale-process detection на Windows детерминированным:
  - [x] искать процессы `rmu-mcp-server.exe` с тем же `ExecutablePath`, кроме текущего PID;
  - [x] возвращать structured warning/error, а не только текстовую подсказку;
  - [x] в recovery hint всегда указывать `scripts/rmu-mcp-server-fresh.cmd`.
- [x] Укрепить `scripts/rmu-mcp-server-fresh.ps1`:
  - [x] kill same-checkout processes;
  - [x] дождаться завершения;
  - [x] если процесс не убит, прерывать запуск с явной ошибкой.

### 19.2 Baseline, dataset и eval gates

- [x] Создать `baseline/investigation/stage0/*` как baseline freeze.
- [x] Создать `baseline/investigation/stage9/*` как eval/gates artifacts.
- [x] Добавить curated dataset для четырёх классов кейсов:
  - [x] `symbol_body`
  - [x] `route_trace`
  - [x] `constraint_evidence`
  - [x] `divergence_report`
- [x] Добавить CLI-only benchmark/eval mode для investigation surface.
- [x] Зафиксировать machine-readable thresholds, report format и compare/diff format.

Обязательные артефакты этой волны:

- [x] `baseline/investigation/stage0/STAGE0_FREEZE.md`
- [x] `baseline/investigation/stage0/contract_freeze_manifest.json`
- [x] `baseline/investigation/stage0/navigation_latency_baseline.json`
- [x] `baseline/investigation/stage0/investigation_dataset.json`
- [x] `baseline/investigation/stage9/STAGE9_EVAL_GATES.md`
- [x] `baseline/investigation/stage9/investigation_dataset.json`
- [x] `baseline/investigation/stage9/gold/*.json`
- [x] `baseline/investigation/stage9/baseline_report.json`
- [x] `baseline/investigation/stage9/thresholds.json`
- [x] `baseline/investigation/stage9/latest_report.json`

### 19.3 Adapters и evidence model

- [x] Расширить `symbol_body` до `TypeScript/JavaScript` sources v1.1.
- [x] Расширить `constraint_evidence` до:
  - [x] `TypeScript ORM/schema-like`
  - [x] `Rust sqlx/diesel-like`
  - [x] `.sql` migration files
- [x] Добавить явное разделение evidence:
  - [x] `strength = strong`
  - [x] `strength = weak`
- [x] Нормализовать `constraint_evidence.kind` до ограниченного набора:
  - [x] `model_constraint`
  - [x] `migration_constraint`
  - [x] `index_constraint`
  - [x] `ddl_like_hint`
  - [x] `runtime_guard`

### 19.4 Route trace и divergence strengthening

- [x] Расширить `RouteSegment` additive-полями:
  - [x] `anchor_symbol`
  - [x] `source_span`
  - [x] `relation_kind`
  - [x] `source_kind`
- [x] Ввести `DivergenceSignal` как отдельный typed object.
- [x] Ввести severity model:
  - [x] `informational`
  - [x] `likely_expected`
  - [x] `suspicious`
  - [x] `high_risk`
- [x] Зафиксировать rule-based differentiation:
  - [x] divergence только по tests/entrypoints не поднимается выше `likely_expected`;
  - [x] divergence по validator/query/constraint axes даёт минимум `suspicious`;
  - [x] конфликт strong constraints или query-path + missing tests даёт `high_risk`.

### 19.5 Docs и operator guidance

- [ ] Обновить `README.md` под новый investigation surface.
- [x] Обновить MCP docs с новыми tools/payloads.
- [x] Описать support matrix по языкам и source classes.
- [x] Добавить examples:
  - [x] `route trace`
  - [x] `backing constraints`
  - [x] `compare concept variants`
- [x] Добавить troubleshooting:
  - [x] stale binary
  - [x] future migration db
  - [x] unsupported source class
  - [x] empty index / preflight expectations

## 20) Что отметить в этом файле после завершения следующей волны

Когда batch из раздела `19` будет реально закончен, в этом roadmap нужно перевести в done следующие вещи:

- [x] Убрать из блока "Что сделано частично" пункт про Windows fresh-start / stale-process protection.
- [ ] Убрать из блока "Что сделано частично" пункт про bounded adapters.
- [ ] Убрать из блока "Что сделано частично" пункт про heuristic-only `route_trace` / `divergence_report`, если severity/richer evidence действительно внедрены.
- [x] Убрать из блока "Что сделано частично" пункт про отсутствие eval dataset / gates / rollout docs.
- [x] В `Этап 0` отметить baseline freeze и dataset артефакты.
- [x] В `Этап 1` отметить `fresh-start launcher`, `operator-facing diagnostics`, `health/preflight`.
- [x] В `Этап 5` отметить new adapters и strong/weak evidence.
- [x] В `Этап 7` отметить `DivergenceSignal` и severity model.
- [x] В `Этап 9` отметить benchmark mode, thresholds, reports и CI gates.
- [x] В `Этап 10` отметить README/MCP docs/support matrix/examples/troubleshooting.
- [x] В `Критерий завершения всего roadmap` перевести в `[x]` пункты про runtime stability, docs/rollout guidance и quality thresholds только после фактической валидации по dataset.
