# Requirements

## Feature

Obsidian-backed project memory server with MCP-only interface.

## Objective

Дать агенту долговременную project memory через Obsidian vault, не превращая систему в chat dump, cloud platform или второй независимый storage.

## Problem Statement

Длинная работа с агентами деградирует из-за трёх провалов:

1. знания о проекте растворяются между сессиями;
2. решения и ограничения теряются или повторно обсуждаются;
3. текущая “правда о проекте” неотделима от эфемерного чата.

MVP должен закрыть эти провалы без тяжёлой инфраструктуры и без потери human-readable source of truth.

## Architectural Drivers

- `Open by default`: человек должен открыть и понять память без специального клиента.
- `Agent-usable`: агент должен читать и обновлять память через MCP.
- `Recoverable`: индекс можно перестроить из vault.
- `Obsidian-native`: graph/backlinks/properties должны работать на стандартном Markdown.
- `Stable roadmap`: архитектура не должна требовать переписывания, если позже добавятся retrieval enhancements.

## Hard Constraints

- Только `MCP`, без `CLI` в MVP.
- `Markdown vault` является единственным source of truth.
- `SQLite` используется только как индекс, кэш выборок и derived state.
- Все данные должны быть открываемы и редактируемы человеком вне MCP.
- Система должна быть `local-first`.
- Один workspace/vault = один memory domain.
- MVP не должен зависеть от внешнего API, облака, Neo4j, ChromaDB или hosted vector backend.

## Non-Negotiable Invariants

- Нельзя хранить canonical truth только в БД.
- Нельзя делать второй parallel truth-store рядом с vault.
- Нельзя смешивать curated project memory и сырой session exhaust в один слой.
- Нельзя расширять MVP новыми сущностями или retrieval-режимами без явного failure against acceptance criteria.
- Нельзя проектировать note schema вокруг hidden server state.
- Нельзя делать формат заметок, который нельзя руками стабильно поддерживать в Obsidian.

## Users

- Агент через MCP
- Человек через Obsidian / файловую систему

## User Stories

### Agent Stories

- Как агент, я хочу быстро получить краткий `project_brief`, чтобы не начинать с нуля.
- Как агент, я хочу найти решения, риски, ограничения и связанные артефакты по теме.
- Как агент, я хочу открыть связанный контекст без чтения всего vault.
- Как агент, я хочу создать новый узел памяти и сразу видеть его в каноническом vault.
- Как агент, я хочу обновить память после изменений проекта без ручного редактирования десятков файлов.
- Как агент, я хочу получить компактный `context_pack` под конкретную задачу.
- Как агент, я хочу увидеть состояние memory/index до начала работы и при сбоях.

### Human Stories

- Как человек, я хочу видеть и править память прямо в Markdown-файлах.
- Как человек, я хочу открывать эти заметки в Obsidian с рабочими `[[links]]`, backlinks и graph view.
- Как человек, я хочу не терять данные, если MCP недоступен.
- Как человек, я хочу понимать, что именно сервер добавил или обновил.

## MVP Scope

- Markdown file model for project memory entities
- Vault scanner + parser + sync engine
- SQLite index
- Graph relations over entities
- MCP read tools
- MCP write tools
- MCP diagnostics/status tools
- Bootstrap/context-pack tools for agent startup
- Deterministic rebuild/recovery path
- Structured tool outputs and structured tool errors

## Explicit Out of Scope

- CLI
- Cloud sync
- Multi-user collaboration semantics
- Generic chat memory for all conversations
- Full transcript warehousing as canonical memory
- Heavy vector infra
- Autonomous background hooks
- AAAK-style compression
- Cross-project federation
- Rich UI beyond Obsidian itself
- Hidden background ingestion daemons
- LLM-in-the-loop indexing or required summarization at ingest time

## Required Entity Types

- `Project`
- `Module`
- `Decision`
- `ArchitectureNote`
- `Task`
- `ProgressEntry`
- `Risk`
- `Constraint`
- `GlossaryTerm`
- `Artifact`

## Required Relation Types

- `relates_to`
- `depends_on`
- `supersedes`
- `documents`
- `blocks`
- `implements`
- `affects`
- `owned_by`
- `derived_from`

## Canonical File Rules

- One entity = one Markdown note.
- Frontmatter must stay flat; nested properties are disallowed in MVP.
- Property names inside a note must be unique.
- Relationship links needed for graph visibility must exist in the note body, not only in frontmatter.
- Internal links in properties, if used, must be quoted per Obsidian behavior.
- Body schema must stay human-readable first and parser-friendly second, never the reverse.

## Functional Requirements

### FR-1. Workspace and Project Binding

- The server must operate against one configured vault root.
- Tools that act on project-specific memory must have deterministic project scoping.
- The server must not rely on ambiguous implicit project detection at call time.

### FR-2. Canonical Note Model

- The server must recognize and validate the canonical note schema.
- Each entity type must map to a deterministic folder home.
- Each note must have stable identity independent of parser session state.
- Title changes must not destroy identity.

### FR-3. Sync and Index Lifecycle

- The server must scan the vault and derive index state from files.
- Sync must detect create/update/delete deltas.
- Sync must be idempotent.
- Full rebuild must be possible using only vault contents.
- The DB must contain no canonical-only knowledge.

### FR-4. Read MCP Surface

- The server must expose read tools sufficient to:
  - search by title/slug/text
  - open entities and linked context
  - inspect graph neighborhoods
  - build project brief

### FR-5. Write MCP Surface

- The server must expose write tools sufficient to:
  - create node
  - update node
  - link/unlink nodes
  - add observation
- Writes must project directly into canonical Markdown.
- Writes must not silently overwrite unsupported or conflicting changes.

### FR-6. Agent Bootstrap Surface

- The server must expose:
  - `recent_changes`
  - `decision_log`
  - `risk_hotspots`
  - `context_pack`
- These tools must return compact task-shaped outputs, not raw dumps.

### FR-7. Diagnostics and Recovery

- The server must expose:
  - `memory_status`
  - `preflight`
  - `index_status`
  - `rebuild_index`
- Diagnostics must distinguish canonical file problems from index problems.
- Recovery path must be explicit and safe.

### FR-8. MCP Contract Quality

- Every tool must define `inputSchema`.
- Tools should define `outputSchema` where practical.
- Tools must return `structuredContent` for machine-usable results.
- Tool-originated errors must be returned in result objects with `isError: true`.
- Tool annotations must be set consistently for read-only, destructive, and idempotent behavior.

## Non-Functional Requirements

### NFR-1. Determinism

- Same vault state must produce same derived index state.
- Same tool call with same inputs against same state must produce same logical result.

### NFR-2. Explainability

- Search and context-pack outputs must be inspectable and attributable to specific notes.
- The user must be able to trace visible memory back to Markdown files.

### NFR-3. Recoverability

- Any derived-state corruption must be recoverable by rebuild from vault.
- Recovery must not require manual DB surgery.

### NFR-4. Obsidian Compatibility

- Notes must remain valid Markdown.
- Obsidian properties must remain editable.
- Graph-visible links must remain standard internal links.

### NFR-5. Safety

- No silent destructive edits.
- No hidden background mutation of canonical notes.
- No required network calls for core memory behavior.

### NFR-6. Performance for MVP

- Project brief generation must avoid full-vault materialization on every call.
- Search over indexed notes must be interactive for a single-project local vault.
- Rebuild may be slower, but must be bounded and deterministic.

### NFR-7. Testability

- Every parser rule must be testable from fixture notes.
- Every MCP tool contract must be testable without UI dependencies.
- Every acceptance criterion must map to at least one verification artifact.

### NFR-8. Privacy by Default

- Tool outputs must avoid leaking unnecessary filesystem detail.
- Sensitive content masking must be possible in status/diagnostic responses.
- No external network activity is allowed in core memory flows.

## Assumptions

- Vault contents are mostly Markdown notes and small metadata files.
- Human edits may happen outside the server at any time.
- The server is allowed to maintain local SQLite state inside its workspace data directory.
- The system is optimized for one user and one local machine in MVP.

## Planning Risks That Must Stay Visible

- Human manual edits can create malformed notes.
- Title/rename behavior can break link integrity if under-specified.
- Derived index can drift from vault if sync semantics are weak.
- Retrieval quality can tempt premature vector scope creep.
- Weak MCP schemas can create model misuse and unstable workflows.

## Acceptance Criteria

1. Агент может получить `project_brief` без чтения всего vault.
2. Агент может найти заметки по теме через `search_memory` и открыть их через `open_nodes`.
3. Агент может создать и обновить узел памяти через MCP без ручного редактирования файла.
4. Агент может связать два узла и добавить observation через MCP.
5. После синка данные в файле и индексе согласованы и диагностируемы.
6. При проблеме индекса сервер умеет сообщить статус, совместимость схемы и безопасный путь восстановления.
7. Человек может открыть все созданные сущности в Obsidian как обычные Markdown notes.
8. MVP не требует ни одного интерфейса кроме MCP.
9. Все MCP tools возвращают предсказуемый structured result shape.
10. Ни один обязательный workflow не опирается на скрытое знание о файловой структуре со стороны агента.

## Acceptance Traceability

- AC1 -> `project_brief`, indexed project summary, sync core
- AC2 -> `search_memory`, `open_nodes`, `read_graph`
- AC3 -> `create_node`, `update_node`
- AC4 -> `link_nodes`, `unlink_nodes`, `add_observation`
- AC5 -> sync engine, `memory_status`, `index_status`
- AC6 -> `preflight`, `rebuild_index`, structured errors
- AC7 -> canonical note model, Obsidian-compatible links/properties
- AC8 -> MCP-only surface, no CLI dependency
- AC9 -> MCP tool contract rules
- AC10 -> project scoping + note/addressing rules

## Change Control

- Любая новая идея идёт сначала в `Post-MVP backlog`, не в активный scope.
- Scope можно расширить только если текущие acceptance criteria нельзя закрыть без расширения.
- Любое расширение должно явно указывать:
  - какую acceptance criterion оно чинит
  - почему текущий scope недостаточен
  - какой риск создаёт
  - какую migration burden добавит

## Definition of Done

MVP готов только когда:

- все acceptance criteria закрыты;
- ни один пункт из out-of-scope не был silently втянут в реализацию;
- canonical vault можно читать и править без MCP;
- derived index можно rebuild из vault;
- MCP workflows верифицированы end-to-end.

## External Verification Notes

This requirements set was cross-checked against:

- Obsidian Help on properties, YAML format, unique property names, flat property limitations, and internal-link behavior in properties.
- Obsidian Help on internal links and graph view, which confirms graph lines come from internal links between notes.
- MCP specification for `inputSchema`, `outputSchema`, `structuredContent`, `isError`, and tool annotations.
- SQLite FTS5 documentation confirming that FTS5 can act as a full-text index over externally stored content.
- Basic Memory docs, which reinforce the `Markdown truth + SQLite index + MCP tools` pattern for shared human/AI knowledge.

### Verified Source URLs

- Obsidian properties: <https://obsidian.md/help/properties>
- MCP schema reference: <https://modelcontextprotocol.io/specification/2025-06-18/schema>
- SQLite FTS5: <https://www.sqlite.org/fts5.html>
- Basic Memory overview: <https://docs.basicmemory.com/start-here/what-is-basic-memory>
- Basic Memory README: <https://raw.githubusercontent.com/basicmachines-co/basic-memory/main/README.md>
- Official MCP memory server README: <https://raw.githubusercontent.com/modelcontextprotocol/servers/main/src/memory/README.md>
- Obsidian Memory MCP README: <https://raw.githubusercontent.com/YuNaga224/obsidian-memory-mcp/main/README.md>
- ConPort README: <https://raw.githubusercontent.com/GreatScottyMac/context-portal/main/README.md>
- Memento MCP README: <https://raw.githubusercontent.com/gannonh/memento-mcp/main/README.md>
- MemPalace README: <https://raw.githubusercontent.com/milla-jovovich/mempalace/main/README.md>
- MemPalace benchmarks: <https://raw.githubusercontent.com/milla-jovovich/mempalace/main/benchmarks/BENCHMARKS.md>
