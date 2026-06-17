# Tasks

## How To Use This Plan

- Делать строго сверху вниз.
- Не открывать следующую фазу, пока не закрыт `Exit Gate` текущей.
- Любая новая идея идёт в `Post-MVP Backlog`, а не в активный scope.
- Если подзадача не ведёт к закрытию acceptance criteria или exit gate, она не входит в MVP.

## Global Anti-Drift Rules

- Нельзя менять canonical architecture после старта `Phase 2`, кроме bugfix-level уточнений.
- Нельзя добавлять новый MCP tool, если он не закрывает явно одну из acceptance criteria.
- Нельзя добавлять semantic/vector слой до завершения `Phase 6`.
- Нельзя перепрыгивать через validation в конце фазы.
- Если в середине реализации появляется “хочется сделать красивее/умнее”, работа останавливается и идея уходит в backlog.

## Delivery Model

Каждая фаза должна оставлять после себя:

- frozen decisions
- explicit artifacts
- verification evidence
- narrow handoff into next phase

## Traceability Rule

Каждая задача должна быть оправдана хотя бы одним из:

- functional requirement
- non-functional requirement
- acceptance criterion
- exit gate текущей фазы

---

## Phase 0. Freeze MVP Contract

### Goal

Убрать архитектурную двусмысленность до начала реализации.

### Why This Phase Exists

Если scope не заморожен сейчас, дальше roadmap начнёт расползаться из-за retrieval-идей, memory-taxonomy идей и “раз уж делаем, давайте ещё”.

### Inputs

- [requirements.md](D:\mcp\obsidian-mcp-memory\.specs\obsidian-memory-mcp-mvp\requirements.md)
- [design.md](D:\mcp\obsidian-mcp-memory\.specs\obsidian-memory-mcp-mvp\design.md)

### Task 0.1. Freeze hard constraints

#### Work

- Зафиксировать:
  - `MCP-only`
  - `Markdown truth`
  - `SQLite derived state`
  - `no CLI`
  - `local-first`
- Проверить, что constraints одинаково сформулированы в requirements и design.

#### Deliverables

- final text in `requirements.md`
- no conflicting wording in `design.md`

#### Validation

- каждый hard constraint встречается в spec явно
- нет текста, допускающего DB-first или dual-truth interpretation

### Task 0.2. Freeze vocabulary

#### Work

- Подтвердить точный список entity types.
- Подтвердить точный список relation types.
- Проверить, что vocabulary достаточен для MVP user stories.

#### Deliverables

- canonical list in `requirements.md`

#### Validation

- ни одна user story не требует нового типа сущности
- нет пересекающихся relation types с одинаковым смыслом

### Task 0.3. Freeze out-of-scope boundary

#### Work

- Проверить, что все tempting extras вынесены в explicit out-of-scope.
- Отдельно зафиксировать:
  - no cloud
  - no transcript warehouse
  - no vector infra in MVP
  - no hooks
  - no cross-project federation

#### Deliverables

- out-of-scope section in `requirements.md`
- post-MVP backlog placeholders in this file

#### Validation

- любой “дополнительный” feature из обсуждения может быть классифицирован либо как MVP, либо как backlog

### Exit Gate

- Команда не спорит, что именно строится.
- Нет двух трактовок source of truth.
- Нет скрытых “скорее всего потом ещё добавим в MVP”.

---

## Phase 1. Canonical File Model

### Goal

Определить, как truth выглядит в vault без участия индекса.

### Why This Phase Exists

Если note model не заморожена до parser/sync, потом начнутся миграции, special-cases и расхождение между тем, что пишет MCP, и тем, что удобно человеку в Obsidian.

### Dependencies

- Phase 0 complete

### Task 1.1. Define vault layout

#### Work

- Зафиксировать folder layout:
  - `<project-slug>--<hash10>/<Human Project Hub>.md`
  - `decisions/`
  - `architecture/`
  - `progress/`
  - `risks/`
  - `glossary/`
  - `tasks/`
  - `artifacts/`
  - `modules/`
  - `constraints/`
- Определить, какие types живут в каких папках.
- Определить роль `_index.md`.

#### Deliverables

- folder mapping table in design or companion note templates

#### Validation

- каждый entity type имеет единственный canonical home
- folder choice не зависит от client/tool behavior

### Task 1.2. Define canonical note schema

#### Work

- Зафиксировать frontmatter fields:
  - `id`
  - `slug`
  - `type`
  - `title`
  - `status`
  - `created_at`
  - `updated_at`
  - `project`
- Зафиксировать body sections:
  - summary
  - observations
  - relations
  - references
- Определить required vs optional fields.

#### Deliverables

- canonical per-type note templates

#### Validation

- любую заметку можно валидировать purely by file contents
- schema readable by human without MCP help

### Task 1.3. Define id/slug/title rules

#### Work

- Определить:
  - stable ID format
  - human-readable file naming strategy
  - slug normalization
  - what happens when title changes
- Принять решение: human title mutable, stable id immutable.

#### Deliverables

- slug/id conventions
- rename behavior spec

#### Validation

- note can be renamed without losing identity
- parser can re-open same entity after file rename

### Task 1.4. Define relation encoding

#### Work

- Зафиксировать exact relation rendering in Markdown.
- Решить:
  - relation list as structured bullets
  - `[[wikilinks]]` for Obsidian graph visibility
  - relation type placement format
  - explicit project-hub link for every non-project note
- Ensure format parseable without heuristics explosion.

#### Deliverables

- exact syntax examples for `relates_to`, `blocks`, `supersedes`, etc.

#### Validation

- relation line is human-readable
- relation line is machine-parsable
- relation remains visible in Obsidian graph

### Task 1.5. Define manual authoring contract

#### Work

- Описать, что человек может safely редактировать руками.
- Описать, что MCP обязан сохранять.
- Описать, какие edits считаются supported, а какие undefined for MVP.

#### Deliverables

- “manual editing contract” note in design or follow-up doc

#### Validation

- supported manual changes do not break sync semantics

### Exit Gate

- Любая сущность может быть вручную создана человеком в vault.
- Сервер сможет её однозначно распознать.
- MCP writing format не конфликтует с human editing.

---

## Phase 2. Parser + Sync Core

### Goal

Построить надёжный derived layer поверх vault.

### Why This Phase Exists

Это настоящий фундамент системы. Если parser/sync unreliable, весь MCP surface станет “иногда работает”.

### Dependencies

- Phase 1 complete

### Task 2.1. Build vault scanner

#### Work

- Рекурсивный обход root vault path.
- Include only supported note files.
- Ignore:
  - hidden/system files
  - unsupported extensions
  - temp artifacts
- Deterministic traversal order.

#### Deliverables

- scanner module
- ignore rules

#### Validation

- same vault => same scan order/results
- unsupported files never leak into entity model

### Task 2.2. Build Markdown parser

#### Work

- Parse frontmatter.
- Parse canonical sections.
- Parse observations.
- Parse relation bullets and wikilinks.
- Parse references.
- Return normalized entity document model.

#### Deliverables

- parser module
- normalized in-memory structure

#### Validation

- parser accepts all canonical templates
- parser rejects malformed core fields clearly
- parser does not silently invent missing data

### Task 2.3. Build normalization/validation layer

#### Work

- Validate required fields.
- Normalize timestamps.
- Normalize ids/slugs.
- Normalize relation targets.
- Emit structured validation errors.

#### Deliverables

- validator module
- error taxonomy

#### Validation

- malformed notes fail predictably
- valid notes normalized identically on repeat parse

### Task 2.4. Design SQLite schema

#### Work

- Tables for:
  - entities
  - observations
  - relations
  - files
  - sync_state
  - schema_version
- Indexes for:
  - entity lookup by id/slug/title
  - relation traversal
  - recent changes
  - lexical search fields

#### Deliverables

- schema definition
- migration/versioning model

#### Validation

- schema represents every MVP concept
- rebuild possible from vault only

### Task 2.5. Build sync engine

#### Work

- Compute create/update/delete delta from vault snapshot.
- Upsert normalized entities into DB.
- Track file fingerprints.
- Track last sync state.
- Ensure idempotency.

#### Deliverables

- sync engine
- delta logic

#### Validation

- repeated sync without file changes produces no logical changes
- file edits propagate correctly
- deleted files are handled deterministically

### Task 2.6. Build rebuild flow

#### Work

- Drop/recreate derived index safely.
- Rebuild entirely from vault.
- Preserve no irreplaceable state in DB.

#### Deliverables

- rebuild implementation

#### Validation

- rebuilt index equals clean fresh index from same vault

### Exit Gate

- Full rebuild from vault works without information loss.
- Sync is idempotent.
- DB contains no canonical-only information.

---

## Phase 3. Core Read MCP

### Goal

Дать агенту базовую read-only полезность без write complexity.

### Why This Phase Exists

Read surface должен доказать, что memory уже полезна до того, как мы дадим write APIs.

### Dependencies

- Phase 2 complete

### Task 3.1. Implement `set_project`

#### Work

- Bind MCP session to vault/project root.
- Validate project existence.
- Load current project metadata.

#### Deliverables

- tool schema
- session binding behavior

#### Validation

- invalid project rejected clearly
- valid project context available to later tools

### Task 3.2. Implement `project_brief`

Status: implemented and later aligned during Phase 5 bootstrap work.

#### Work

- Return compact project summary from indexed entities.
- Pull from:
  - `Project`
  - recent `Decision`
  - active `Risk`
  - recent `ProgressEntry`

#### Deliverables

- brief builder

#### Validation

- response compact
- no raw vault dump
- enough context to orient agent quickly

### Task 3.3. Implement `search_memory`

#### Work

- Support:
  - title/slug lookup
  - lexical search over summary/observations
  - type filters
  - project-scoped results

#### Deliverables

- search tool
- ranking rules for MVP

#### Validation

- exact title hits rank first
- results stable and explainable

### Task 3.4. Implement `open_nodes`

#### Work

- Open entities by id/slug/name.
- Return note content + key metadata + linked relations.

#### Deliverables

- open tool

#### Validation

- entity resolution deterministic
- not-found handled clearly

### Task 3.5. Implement `read_graph`

#### Work

- Return graph neighborhood for selected entities or scoped project view.
- Avoid huge uncontrolled dumps.

#### Deliverables

- graph read tool

#### Validation

- graph view bounded
- relation labels preserved

### Exit Gate

- Агент может войти в проект, найти нужное и открыть связи.
- Это возможно без знания файловой структуры vault.

---

## Phase 4. Core Write MCP

### Goal

Дать агенту право безопасно поддерживать canonical memory.

### Why This Phase Exists

Если write surface появится раньше safety rules, MCP начнёт ломать vault format и human trust.

### Dependencies

- Phase 3 complete

### Task 4.1. Implement `create_node`

#### Work

- Resolve target folder by type.
- Generate canonical note file.
- Populate required frontmatter/sections.
- Trigger sync/update index.

#### Deliverables

- create tool
- note creation path

#### Validation

- file created in canonical location
- note opens in Obsidian as normal Markdown
- index reflects new entity

### Task 4.2. Implement `update_node`

#### Work

- Load canonical note.
- Patch supported fields/sections only.
- Preserve formatting contract.
- Detect unsupported/conflicting updates.

#### Deliverables

- update tool
- patch strategy

#### Validation

- updates do not corrupt note structure
- title/body/metadata changes remain parseable

### Task 4.3. Implement `link_nodes`

#### Work

- Resolve both node identities.
- Add relation in canonical Markdown format.
- Re-sync index.

#### Deliverables

- link tool

#### Validation

- link visible in file
- link visible in graph/index

### Task 4.4. Implement `unlink_nodes`

#### Work

- Remove exact relation, not fuzzy similar text.
- Re-sync index.

#### Deliverables

- unlink tool

#### Validation

- only targeted relation removed
- no collateral note damage

### Task 4.5. Implement `add_observation`

#### Work

- Append observation to proper section.
- Avoid duplicate insertion policy ambiguity.

#### Deliverables

- observation tool

#### Validation

- observation appears in file and index
- duplicate behavior deterministic

### Task 4.6. Implement write safety guards

#### Work

- conflict detection
- unsupported patch rejection
- stable file write strategy
- no silent destructive overwrite

#### Deliverables

- write safety policy inside tool layer

#### Validation

- concurrent/manual drift produces safe error or predictable reconciliation rule

### Exit Gate

- Агент может safely поддерживать память через MCP.
- Любая write operation сохраняет canonical file discipline.

---

## Phase 5. Agent Bootstrap MCP

Status: implemented in code.
Verification note: `cargo fmt --check` and `cargo metadata --no-deps --format-version 1` passed, but full `cargo check` / `cargo test` remain blocked on this machine by missing MSVC/WSL Rust toolchain pieces.

### Goal

Сделать memory удобной именно для агентного старта и task-shaped retrieval.

### Why This Phase Exists

Просто search/open недостаточно. Агенту нужен curated startup context, иначе он либо flood'ит vault, либо недобирает нужное.

### Dependencies

- Phase 4 complete

### Task 5.1. Implement `recent_changes`

Status: implemented.

#### Work

- Return recent updated nodes with type, title, timestamp, brief reason/context if derivable.

#### Deliverables

- recent changes tool

#### Validation

- result useful for session resume
- ordering stable by update timestamp

### Task 5.2. Implement `decision_log`

Status: implemented.

#### Work

- Return ordered decisions.
- Support project-scoped and optionally topic-filtered output.

#### Deliverables

- decision log tool

#### Validation

- critical architecture decisions discoverable without broad search

### Task 5.3. Implement `risk_hotspots`

Status: implemented.

#### Work

- Aggregate unresolved risks and constraints.
- Highlight blockers and affected entities.

#### Deliverables

- risk tool

#### Validation

- hotspot output actionable, not dump-like

### Task 5.4. Implement `context_pack`

Status: implemented.

#### Work

- Accept task/topic seed.
- Assemble compact package from:
  - project brief
  - relevant decisions
  - related modules/artifacts
  - unresolved risks
  - recent changes
- Explicit token/size budget behavior.

#### Deliverables

- context pack tool
- packing heuristics for MVP

#### Validation

- output compact
- output connected
- output task-shaped, not generic corpus sample

### Exit Gate

- Новая агентная сессия быстро получает рабочий контекст.
- Vault flooding не требуется.

---

## Phase 6. Diagnostics MCP

### Goal

Сделать систему operable и recoverable.

### Why This Phase Exists

Local-first memory без diagnostics быстро превращается в “непонятно, что именно сломалось”.

### Dependencies

- Phase 5 complete

### Task 6.1. Implement `memory_status`

#### Work

- High-level health:
  - selected project
  - entity counts
  - last sync
  - parser/index health summary

#### Deliverables

- status tool

#### Validation

- one-shot overview enough for quick health check

### Task 6.2. Implement `preflight`

#### Work

- Check:
  - schema compatibility
  - index version compatibility
  - stale derived state risks
  - missing required project files

#### Deliverables

- preflight tool

#### Validation

- likely operational breakages surfaced before normal use

### Task 6.3. Implement `index_status`

#### Work

- Detailed index state:
  - row counts
  - fingerprint drift
  - last successful sync
  - pending inconsistencies

#### Deliverables

- index diagnostics tool

#### Validation

- enough detail to distinguish vault issue vs index issue

### Task 6.4. Implement `rebuild_index`

#### Work

- Controlled rebuild entrypoint.
- Safe confirmation semantics at MCP layer if needed.
- Recompute from vault only.

#### Deliverables

- rebuild tool

#### Validation

- rebuild path deterministic
- no canonical data loss possible

### Task 6.5. Implement structured error surface

#### Work

- Standardize errors for:
  - invalid input
  - malformed note
  - sync conflict
  - not found
  - stale index
  - rebuild required
  - schema mismatch

#### Deliverables

- error contract across all MCP tools

#### Validation

- same class of failure returns same shape of error

### Exit Gate

- Сервер operable without hidden state.
- Recovery path exists and is understandable.

---

## Validation + Hardening

### Goal

Доказать, что MVP закрыт не на словах, а на evidence.

### Why This Work Exists

Без финального hardening roadmap почти всегда “номинально готов”, но реально unstable.

### Dependencies

- Phase 6 complete

### Task 7.1. Parser tests

#### Work

- valid canonical notes
- malformed frontmatter
- malformed sections
- relation parsing edge cases

#### Exit Evidence

- parser accepts valid notes, rejects bad input predictably

### Task 7.2. Sync tests

#### Work

- initial sync
- repeated sync idempotency
- file update propagation
- delete propagation
- rebuild consistency

#### Exit Evidence

- no drift under repeated operations

### Task 7.3. MCP contract tests

#### Work

- tool schemas
- read flow tests
- write flow tests
- diagnostics flow tests

#### Exit Evidence

- MCP-only workflow fully executable

### Task 7.4. Recovery tests

#### Work

- stale index
- malformed note
- partial rebuild scenario
- schema mismatch handling

#### Exit Evidence

- system fails loudly and recoverably

### Task 7.5. Obsidian compatibility smoke tests

#### Work

- generated notes open as normal Markdown
- wikilinks preserved
- graph-visible relations preserved

#### Exit Evidence

- human-facing vault usability intact

### Task 7.6. Acceptance criteria closure

#### Work

- map each criterion from requirements to test or verified behavior

#### Exit Evidence

- explicit checklist proving MVP done

### Exit Gate

- All acceptance criteria closed with verification evidence.
- No MVP scope violation introduced during implementation.

---

## Post-MVP Backlog

### Retrieval Enhancements

- vector/semantic retrieval
- chunking strategy
- local embedding provider abstraction
- retrieval diagnostics

### Memory Enhancements

- transcript ingest
- mempalace-style raw recall bridge
- session bootstrap enhancer
- diary/compression layer

### Platform Enhancements

- cloud sync
- multi-project federation
- richer UI/workflows beyond MCP

---

## Section-Hub Graph Update

- Graph layout is now materialized as `project hub -> section hubs -> leaf notes`.
- Fixed system-managed section hubs must exist for tasks, risks, decisions, modules, progress, architecture, constraints, artifacts, and glossary.
- Section hubs are not user-managed write targets; MCP rebuilds them after writes and migration.
- Default bootstrap surfaces should stay focused on domain notes and avoid surfacing section hubs unless explicitly requested.

## Anti-Drift Checklist For Every Review Point

- Это закрывает current phase exit gate?
- Это требуется acceptance criteria?
- Это изменяет canonical layer?
- Это можно безопасно отложить в backlog?
- Это создаёт migration burden прямо сейчас?

Если хотя бы на один из первых двух вопросов ответ `нет`, задача не входит в текущую фазу.
