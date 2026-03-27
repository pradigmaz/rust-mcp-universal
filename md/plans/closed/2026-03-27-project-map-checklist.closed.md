# Project Map Checklist

## Статус

- [x] Checklist закрыт по состоянию на 27 марта 2026
- [x] Canonical closed copy сохранена в `md/plans/closed/2026-03-27-project-map-checklist.closed.md`
- [x] Чеклист синхронизирован с текущим кодом
- [x] Различия между `CLI brief`, `MCP workspace_brief` и `MCP agent_bootstrap` зафиксированы
- [x] Базовая пара `workspace_brief + agent_bootstrap` подтверждена кодом и тестами
- [x] Standalone `search` перепроверен и зафиксирован regression-тестом для generic `mod/runtime` query
- [x] Standalone `search` для plain `tests` query закрыт через test-surface fallback и boosted merge
- [x] Для `agent_bootstrap` реализован additive broad-query path с multi-layer diversification
- [x] Для `agent_bootstrap` добавлен optional `query_bundle.followups`
- [x] Собраны наблюдения по `healthy`, `empty`, `incompatible`, `mixed-language`, `no-root-manifest`
- [x] Закрыта неоднозначность `MCP agent_bootstrap` без `query` и без `auto_index`
- [x] Сохранены повторяемые JSON-артефакты по внешним fixture-репозиториям

## Цель

Держать один рабочий checklist для стартовой поверхности `project map` в `rust-mcp-universal`.

Базовые поверхности:

1. `workspace_brief`
2. `agent_bootstrap`

Проверять их нужно в трех разных режимах:

1. `CLI brief`
2. `MCP workspace_brief`
3. `MCP agent_bootstrap`

## Контрактные точки

### Core

- `crates/core/src/engine_brief.rs`
- `crates/core/src/engine/query/agent.rs`
- `crates/core/src/engine/query/pipeline.rs`
- `crates/core/src/engine/query/intent.rs`
- `crates/core/src/engine/query/intent/bootstrap.rs`
- `crates/core/src/model/core/types/agent.rs`

### CLI

- `crates/cli/src/commands/maintenance.rs`
- `crates/cli/src/commands/query/agent.rs`

### MCP

- `crates/mcp-server/src/rpc_tools/dispatch/project.rs`
- `crates/mcp-server/src/rpc_tools/handlers/agent_bootstrap.rs`

## Подтвержденные факты по контракту

- [x] `CLI brief` может auto-index и вернуть полезный snapshot
- [x] `MCP workspace_brief` read-only и не чинит пустой индекс молча
- [x] `MCP workspace_brief` на несовместимом индексе может вернуть `repair_hint`
- [x] `agent_bootstrap` без `query` возвращает `brief` и `query_bundle = null`
- [x] `agent_bootstrap` с `query` возвращает `brief + query_bundle + timings`
- [x] `agent_bootstrap` по умолчанию не тащит `report` и `investigation_summary`
- [x] `agent_bootstrap` поддерживает opt-in через `include_report` и `include_investigation_summary`
- [x] `workspace_brief` несет `languages`, `top_symbols`, `quality_summary`, `recommendations`
- [x] `quality_summary.status` нужно читать по факту. Нормальные состояния в observed path: `ready`, `stale`, `degraded`, `unavailable`
- [x] `agent_bootstrap` поддерживает broad-query diversification по слоям и корням
- [x] `agent_bootstrap` может добавлять optional `query_bundle.followups`
- [x] broad-query diversification работает и для natural-language query, а не только для keyword query
- [x] `MCP agent_bootstrap` без `query` и без `auto_index` больше не materialize пустой `.rmu/index.db`

## Execution Modes

### 1. CLI `brief`

Используется для human/operator path и проверки auto-index поведения.

```powershell
cargo run --locked -p rmu-cli -- --project-path "<REPO>" --json brief
```

### 2. MCP `workspace_brief`

Используется для проверки read-only контракта стартового snapshot.

Инструменты:

1. `set_project_path`
2. `preflight`
3. `workspace_brief`

### 3. MCP `agent_bootstrap`

Используется для проверки агентского стартового payload.

Инструменты:

1. `set_project_path`
2. `preflight`
3. `agent_bootstrap`
4. `agent_bootstrap` с `query`
5. `agent_bootstrap` с `include_report=true` и `include_investigation_summary=true`

## Index State Matrix

### Healthy indexed repo

- [x] `workspace_brief` и `brief` дают стабильный snapshot без неожиданных пустых полей
- [x] `languages` отражают реальный стек
- [x] `quality_summary` интерпретируется по фактическому `status`, а не по ожиданию `ready`

Evidence:

- `platforma_maga`: `files=799`, `typescript=401`, `python=352`, `quality=ready`
- `QubeForge`: `files=199`, `typescript=181`, `quality=ready`
- `kiron_client`: после свежего CLI-прогона `files=73`, `java=61`, `quality=ready`

### Empty index

- [x] `CLI brief` может подготовить индекс и вернуть полезный snapshot
- [x] `MCP workspace_brief` на пустом индексе дает явный fail
- [x] `MCP agent_bootstrap` с `query` и `auto_index=false` не скрывает пустой индекс
- [x] `MCP agent_bootstrap` с `query` и `auto_index=true` подготавливает индекс до payload-сборки
- [x] `MCP agent_bootstrap` без `query` и `auto_index=false` теперь ведет себя как read-only path и не создает `.rmu/index.db`

### Incompatible index

- [x] `MCP workspace_brief` возвращает `repair_hint`, если read-only режим не может чинить индекс
- [x] checklist не путает `repair_hint` с обычным healthy snapshot

### Mixed-language repo

- [x] краткая карта не схлопывается до одного языка
- [x] build/docs/generated шум не доминирует над кодовым ядром
- [x] `agent_bootstrap` по broad query поднимает несколько слоев системы на synthetic fixtures
- [x] `agent_bootstrap` после последнего patch лучше держит `api + domain/services` на service-oriented broad query
- [x] generic broad query на Java mod fixtures после mod-root rebalance держит несколько `mods/<module>` roots и не схлопывается в один модуль

### Repo без root manifest

- [x] `workspace_brief` оценивается отдельно для CLI и MCP
- [x] `agent_bootstrap` проверен как fallback-путь к первому осмысленному `project map`

## Acceptance Criteria

### `workspace_brief`

- [x] корректно работает как стартовый snapshot на healthy index
- [x] не требует `quality_summary.status = ready` как единственно допустимый режим
- [x] `repair_hint` трактуется как отдельный результат, а не как healthy path
- [x] read-only fail на пустом индексе подтвержден
- [x] перепрогнан на внешних fixtures через сохраненные артефакты

### `agent_bootstrap` default path

- [x] без `query` возвращает полезный `brief`, а не пустой контейнер
- [x] no-query MCP path не создает пустой `.rmu/index.db`
- [x] с broad keyword query дает карту, а не случайный shortlist
- [x] с broad natural-language query дает карту, а не случайный shortlist
- [x] `hits` могут поднимать backend, frontend, tests и adjacent layers в одном payload
- [x] `context` укладывается в бюджет и остается полезным
- [x] zero-hit поведение остается явным и не маскируется docs-only noise
- [x] `timings` заполнены
- [x] optional `followups` дают следующий шаг без ломки default payload shape
- [x] broad query на реальных repos закрыт и для generic mod/runtime case через universal mod-root balancing

### `agent_bootstrap` opt-in path

- [x] `include_report=true` реально добавляет `query_bundle.report`
- [x] `include_investigation_summary=true` реально добавляет `query_bundle.investigation_summary`
- [x] opt-in поверхность не ломает default payload shape
- [x] `report.timings` согласованы с top-level `timings`

## Реализованные улучшения

- [x] Добавлена broad-intent классификация для `architecture`, `entrypoints`, `auth`, `tests`
- [x] Добавлен data-driven fallback для broad natural-language query на основе hit distribution
- [x] Добавлена shortlist diversification по root buckets и role buckets
- [x] Добавлены path-based layer signals для `api`, `domain`, `services`, `orchestration`, `rules`
- [x] Support/docs artifacts откладываются до поздней волны и не лезут в top shortlist раньше кодового ядра
- [x] Добавлены `query_bundle.followups`
- [x] Добавлены core и MCP тесты на broad keyword query
- [x] Добавлены core и MCP тесты на broad natural-language query
- [x] Добавлены core и MCP тесты на balanced `api + domain/service` broad query
- [x] Добавлен MCP regression test на no-query path без materialized index
- [x] Добавлены universal scoring и prefix-rebalance для generic `mods/<module>` broad query без repo-specific hardcode
- [x] Добавлен core regression test, подтверждающий тот же universal balancing в standalone `search`
- [x] Добавлены pure-test intent fix и supplement merge, чтобы `search --query tests` поднимал реальные test files, а не только упоминания

## Evidence Artifacts

Артефакты сохранены в `md/plans/project-map-artifacts/<repo-slug>/`.

Для каждого fixture сохранены:

- [x] `brief.json`
- [x] `agent.no_query.json`
- [x] `agent.query.primary.json`
- [x] `agent.query.secondary.json`

## Validation Fixtures

### `D:\универ\platforma_maga`

Observed:

- [x] mixed `frontend/ + backend/`
- [x] `typescript=401`, `python=352`
- [x] есть `.ai`, `.kiro`, `.rmu`, docs noise
- [x] healthy index, `quality=ready`
- [x] primary broad query `frontend pages backend api routes auth flow` теперь держит backend и frontend в одном shortlist
- [x] secondary broad query `auth boundary tests nearby backend frontend` теперь выводит test surface в top hits

Top evidence:

- primary: `backend/app/api/v1/endpoints/auth.py`, `frontend/.../page.tsx`, `backend/app/services/external_api.py`, `frontend/src/hooks/useAutoLogin.ts`, `backend/app/api/v1/api.py`
- secondary: `backend/app/tests/integration/test_db_models.py`, `backend/app/services/bot/auth.py`, `backend/app/tests/test_backup.py`, `backend/tests/conftest.py`, `backend/tests/setup_webhook.py`

### `D:\scripts\QubeForge`

Observed:

- [x] Vite/TypeScript repo
- [x] есть `dist/`, `public/`, `docs/`
- [x] healthy index, `typescript=181`, `quality=ready`
- [x] broad UI query остается внутри кодового UI-ядра
- [x] test adjacency подтверждена

Top evidence:

- primary: `src/ui/menus/visibility.ts`, `src/ui/menus/state.ts`, `src/ui/Menus.ts`
- secondary: `src/ui/SaveCoordinator.test.ts`, `src/ui/SaveCoordinator.ts`, `src/ui/AutoSave.ts`

### `D:\перевод манхв\manhwa_manager`

Observed:

- [x] Python/service-domain layout есть даже без root manifest
- [x] директории `api`, `domain`, `services`, `modules` присутствуют
- [x] `CLI brief` auto-index path подтвержден
- [x] `brief.json` после CLI дает `files=60`, `python=58`, `quality=ready`
- [x] broad service query после patch больше не схлопывается только в `api/*`
- [x] service-oriented shortlist стабильно держит `services/*` рядом с `api/*`, а domain surface поднимается вторым query

Top evidence:

- primary: `services/chapter_service.py`, `api/routes/transfer_routes.py`, `services/storage_service.py`, `api/routes/kanban_routes.py`, `api/routes/settings_routes.py`
- secondary: `domain/ui/menu_handler.py`, `domain/chapter/folder_generator.py`, `domain/chapter/selector.py`, `domain/template/manager.py`

### `E:\Новая папка\kiron_client`

Observed:

- [x] Java/Gradle mod repo
- [x] есть `.gradle`, `build`, `_legacy_src_backup`
- [x] docs/build noise не попадает в top shortlist
- [x] после свежего CLI-прогона `brief.json` дает `quality=ready`
- [x] targeted query `kiron client entrypoint render modules config` держит `mods/kiron_client/*`
- [x] generic query `mod entrypoint mixins runtime hooks config network` после lexical rebalancing держит и `mods/kiron_client/*`, и `mods/veinmining/*` в top window
- [x] followups для generic mod/runtime query теперь module-oriented, а не frontend-oriented

Top evidence:

- primary: `mods/kiron_client/.../MixinPlugin.java`, `mods/veinmining/.../VeinMiningMod.java`, `mods/kiron_client/.../EspMobColorStore.java`, `mods/kiron_client/.../KironClient.java`, `mods/kiron_client/.../Kiron_clientClient.java`
- secondary: `mods/kiron_client/.../MixinSodiumSectionRenderData.java`, `mods/kiron_client/.../KironClient.java`

## Стандартизованный Query Set

### `platforma_maga`

1. `frontend pages backend api routes auth flow`
2. `auth boundary tests nearby backend frontend`

### `QubeForge`

1. `ui entrypoint routing tests state store screens`
2. `ui menus tests save coordinator`

### `manhwa_manager`

1. `orchestration domain rules api service layer`
2. `domain services orchestration translation pipeline rules`

### `kiron_client`

1. `mod entrypoint mixins runtime hooks config network`
2. `kiron client entrypoint render modules config`

## Fail Conditions

- [x] `project map` показывает реальные entrypoints или ближайшие code-bearing proxies
- [x] mixed repo не схлопывается до одного языка, одного слоя или одного `mods/<module>` root
- [x] docs/build/generated/support artifacts не попадают в top shortlist раньше кодового ядра
- [x] checklist не смешивает CLI и MCP как будто это один режим
- [x] `repair_hint` и unhealthy quality states не трактуются как healthy snapshot
- [x] после `brief` и `agent_bootstrap` есть внятный next step через `recommendations` или `followups`

## Итоги текущей волны

### Что закрыто

- no-query MCP bootstrap path выровнен с read-only семантикой
- broad-query ranking стал устойчивее для `api + domain/services` кейсов
- generic `mod/runtime` path закрыт и в `agent_bootstrap`, и в standalone `search`
- plain `tests` query теперь поднимает реальные test files через universal fallback path
- внешний артефактный прогон выполнен и сохранен
- checklist синхронизирован по состоянию на 27 марта 2026 и может считаться закрытым

### Что остается открытым

- открытых блокеров по этому checklist нет

### Следующий приоритет

1. Новые улучшения retrieval заводить уже как отдельный roadmap/checklist, а не как продолжение этого закрытого документа.
2. Повторный fixture-прогон делать только при новых изменениях ranking/indexing, не для повторного открытия этого checklist без новой причины.
