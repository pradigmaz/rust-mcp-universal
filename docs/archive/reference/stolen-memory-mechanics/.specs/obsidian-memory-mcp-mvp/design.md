# Design

## External Verification Snapshot

Observed facts from primary sources:

- Obsidian properties are YAML at the top of the file, property names must be unique, nested properties are not supported in normal UI, and internal links in properties must be quoted.
- Obsidian graph view renders relationships from internal links between notes.
- MCP tools should define `inputSchema`, may define `outputSchema`, and tool results should use `structuredContent`; tool-originated failures should use `isError`.
- SQLite FTS5 is built to maintain an index over externally stored content and supports prefix, phrase, proximity, and boolean queries.
- Basic Memory demonstrates a proven `Markdown files as truth + SQLite index + MCP tools` architecture for shared human/AI knowledge.

Architectural inference from those facts:

- frontmatter must stay flat and simple;
- graph-relevant relations must exist as body links, not just hidden metadata;
- MCP contracts should be machine-readable from day one;
- lexical search via SQLite FTS5 is enough for MVP and keeps future vector work additive.

## Architecture Decision

Архитектура делится на 4 слоя, чтобы roadmap не расползался:

1. `Canonical layer`
   Markdown files in vault. Только здесь хранится truth.
2. `Operational layer`
   Vault scanning, parsing, sync metadata, SQLite index, consistency checks.
3. `Retrieval layer`
   Search, open, relation traversal, context assembly.
4. `MCP layer`
   Единственная внешняя поверхность системы.

## Why This Stays Stable

- Scope anchored to layers, not wishlist features.
- Every step closes one missing capability in the fixed MVP contract.
- Later enhancements can plug into retrieval layer without rewriting canonical layer.
- All derived-state behavior is rebuildable from files.

## Core Design Decisions

### DD-1. Markdown is the canonical truth

Reason:

- human-readable
- Obsidian-native
- Git/backup friendly
- no lock-in

Consequence:

- DB never becomes source of truth
- every write tool must write Markdown first-class artifacts

### DD-2. SQLite is derived operational state only

Reason:

- enough for indexing/search/relations/diagnostics
- much lighter than graph DB
- rebuildable from files

Consequence:

- schema must only hold derivable facts plus sync metadata
- rebuild must be a first-class operation

### DD-3. Flat frontmatter, rich body

Reason:

- Obsidian properties are YAML-based and nested properties are not well-supported in main properties UI
- body links are the safest path to graph/backlinks visibility

Consequence:

- frontmatter for atomic metadata
- observations/relations in body sections

### DD-4. Retrieval must be layered, not canonical

Reason:

- search strategy will evolve
- file truth must not change because retrieval got smarter

Consequence:

- vector/semantic retrieval stays post-MVP
- `context_pack` consumes indexed graph; it does not redefine truth

### DD-5. MCP contracts should be strongly structured

Reason:

- clients and models work better with explicit schemas
- future compatibility and self-correction improve

Consequence:

- every tool gets input schema
- important tools get output schema
- results return `structuredContent`
- tool failures return `isError`

## System Context

```text
Human <-> Obsidian / filesystem
                |
                v
        Markdown vault (truth)
                |
                v
      scanner -> parser -> validator
                |
                v
      sync engine -> SQLite derived index
                |
                v
        MCP tools for read/write/bootstrap/diagnostics
                |
                v
              Agent
```

## Vault Layout

```text
vault/
  <project-slug>--<hash10>/
    Obsidian Memory MCP.md
    decisions/
    architecture/
    progress/
    risks/
    glossary/
    tasks/
    artifacts/
    modules/
    constraints/
```

## Canonical Note Model

### File Naming

Recommended MVP rule:

- file basename = human-readable title
- stable slug stays in frontmatter and derived index
- project hub note lives as a readable root file, not `_index.md`
- project hub also materializes a fixed set of readable root-level section hubs such as `<Project Title> Tasks.md`
- historical notes may add a readable status suffix when needed to avoid filename collisions
- links render through body wikilinks and keep stable slug targeting for deterministic parsing

Why:

- Obsidian graph labels become readable by default
- stable addressing is preserved by frontmatter slug/id instead of filename
- title changes stay compatible with deterministic file rename behavior
- folder clusters become graph-visible through section-hub notes instead of relying on folders

### Frontmatter

Flat YAML only:

```yaml
---
id: decision-auth-token-strategy
slug: auth-token-strategy
type: Decision
title: Auth Token Strategy
status: active
project: sample-project
created_at: 2026-04-11T10:00:00Z
updated_at: 2026-04-11T10:00:00Z
tags:
  - auth
  - security
aliases:
  - Token Strategy
---
```

Rules:

- no nested objects in MVP
- internal links in properties, if ever used, must be quoted
- `tags` and `aliases` follow Obsidian conventions
- machine slug stays explicit in frontmatter because filenames are now human-readable

### Body

```markdown
# Auth Token Strategy

## Summary
Short human-readable summary.

## Observations
- [decision] Use rotating refresh tokens for long-lived sessions #auth
- [risk] Token theft remains a high-impact failure mode #security

## Relations
- implements [[api-auth-module|API Auth Module]]
- supersedes [[legacy-session-cookies|Legacy Session Cookies]]
- documents [[_index|Obsidian Memory MCP]]
- documents [[section-decisions|Obsidian Memory MCP Decisions]]

## References
- [[security-model]]
- [[incident-2026-03-login-spike]]
```

Rules:

- one fact per observation line
- relation type always explicit in body
- graph-relevant links must live in body
- every non-project note must keep an explicit project-hub relation in body
- every non-project note must also keep an explicit section-hub relation in body
- section hubs are system-managed notes that connect the project hub to all notes of one section

## Entity-Type Guidance

### `Project`

- One per project root
- Holds short project identity + scope summary
- Acts as the central Obsidian graph hub for the whole project memory

### `SectionHub`

- Fixed system-managed note per major section
- Lives in the project root next to the project hub
- Represents folder clusters in Graph view
- Links project hub to leaf notes for one section

### `Module`

- Technical subsystem or bounded implementation area

### `Decision`

- Explicit architectural or process choice
- Must include rationale in summary/references/observations

### `ArchitectureNote`

- Explanatory design note not necessarily a binary decision

### `Task`

- Concrete work item or workstream record

### `ProgressEntry`

- Temporal update about movement, milestone, or discovery

### `Risk`

- Unresolved risk with impact relevance

### `Constraint`

- Hard limitation, policy, dependency boundary, or invariant

### `GlossaryTerm`

- Stable vocabulary entry

### `Artifact`

- Pointer to code/doc/spec/runbook/incident/report

## Relation Semantics

- `relates_to`: weak semantic connection
- `depends_on`: requires other entity for validity or execution
- `supersedes`: replaces prior truth
- `documents`: note describes target
- `blocks`: unresolved obstacle
- `implements`: concrete realization of another entity
- `affects`: change impact relationship
- `owned_by`: responsibility
- `derived_from`: created from another artifact or note

Rule:

- use strongest precise relation, otherwise fallback to `relates_to`

## Addressing Model

Canonical identity:

- stable `id`

Human navigation:

- `title`
- file path
- Obsidian wikilinks

Index resolution keys:

- `id`
- slug
- title
- aliases

MVP decision:

- tools should resolve by `id`, slug, title, or alias
- returned structured results should always include canonical `id`

## Sync Model

### Pipeline

1. Scanner enumerates candidate Markdown notes
2. Parser reads frontmatter/body
3. Validator normalizes and checks canonical schema
4. Sync engine computes delta
5. SQLite derived index updated transactionally

### DB Contents

- `entities`
- `entity_aliases`
- `observations`
- `relations`
- `files`
- `sync_state`
- `schema_version`
- `fts_entities` virtual table using FTS5

### Rebuild Principle

The DB may be deleted and rebuilt entirely from vault.

If that statement becomes false, architecture has drifted.

### Sync Transaction Rules

- one sync pass = one logical transaction boundary
- parser/validation failures are recorded explicitly, not swallowed
- malformed notes do not silently disappear from diagnostics
- successful sync updates `last_successful_sync_at`
- failed sync preserves previous valid derived state until recovery action

### Drift Semantics

Drift classes:

- `none`
- `file_changed_not_synced`
- `index_missing`
- `index_orphan`
- `parse_failed`

These drift classes should appear in diagnostics tools and fixtures.

## Search Model

MVP search is not “fancy RAG platform”.

It must support:

- exact title/slug lookup
- lexical search over summary/observations
- relation-aware open/traverse
- task-shaped context assembly

### MVP Ranking

Order of preference:

1. exact `id`
2. exact slug/title/alias
3. lexical FTS hit
4. relation-neighborhood expansion

### Why FTS5 First

SQLite FTS5 supports:

- token index over externally stored content
- prefix search
- phrase queries
- proximity queries
- boolean queries

This is enough to ship deterministic local search before adding embeddings.

## MCP Surface

### Design Principle

Tools should be narrow, typed, and explicit. Tool descriptions must tell the model what each tool is for. Read/write/destructive/idempotent behavior must be hinted via tool annotations.

### Project / Bootstrap

- `set_project`
- `project_brief`
- `context_pack`

### Read

- `search_memory`
- `open_nodes`
- `recent_changes`
- `decision_log`
- `risk_hotspots`
- `read_graph`

### Write

- `create_node`
- `update_node`
- `link_nodes`
- `unlink_nodes`
- `add_observation`

### Operations / Diagnostics

- `memory_status`
- `sync_vault`
- `preflight`
- `index_status`
- `rebuild_index`

## MCP Contract Rules

### Tool Metadata

- `inputSchema` mandatory
- `outputSchema` for core tools strongly recommended
- `title` and `description` human-readable
- annotations:
  - read tools: `readOnlyHint=true`
  - additive safe writes: `destructiveHint=false`
  - idempotent status/search tools: `idempotentHint=true`
  - memory tools: `openWorldHint=false`

### Tool Results

Every important tool should return:

- `content` for human-readable fallback
- `structuredContent` for model-usable shape

Tool-originated failures should return:

- `isError: true`
- structured error payload with stable error code

## Tool Contract Table

| Tool              | Category        | Read only | Idempotent | Destructive | Required structured fields                                           |
| ----------------- | --------------- | --------: | ---------: | ----------: | -------------------------------------------------------------------- |
| `set_project`     | bootstrap       |        no |        yes |          no | `project`, `project_root`, `status`                                  |
| `project_brief`   | bootstrap/read  |       yes |        yes |          no | `project`, `summary`, `top_decisions`, `top_risks`, `recent_changes` |
| `context_pack`    | bootstrap/read  |       yes |        yes |          no | `seed`, `brief`, `included_nodes`, `recent_changes`, `risks`         |
| `search_memory`   | read            |       yes |        yes |          no | `query`, `hits`                                                      |
| `open_nodes`      | read            |       yes |        yes |          no | `nodes`                                                              |
| `read_graph`      | read            |       yes |        yes |          no | `nodes`, `relations`                                                 |
| `recent_changes`  | read            |       yes |        yes |          no | `changes`                                                            |
| `decision_log`    | read            |       yes |        yes |          no | `decisions`                                                          |
| `risk_hotspots`   | read            |       yes |        yes |          no | `risks`, `constraints`                                               |
| `create_node`     | write           |        no |         no |          no | `node`, `file_path`, `sync_status`                                   |
| `update_node`     | write           |        no |         no |          no | `node`, `updated_fields`, `sync_status`                              |
| `link_nodes`      | write           |        no |        yes |          no | `relation`, `sync_status`                                            |
| `unlink_nodes`    | write           |        no |        yes |         yes | `removed_relation`, `sync_status`                                    |
| `add_observation` | write           |        no |         no |          no | `node`, `observation`, `sync_status`                                 |
| `memory_status`   | diagnostics     |       yes |        yes |          no | `project`, `health`, `counts`, `last_sync`                           |
| `sync_vault`      | diagnostics/ops |        no |        yes |          no | `sync_result`, `changes`, `errors`                                   |
| `preflight`       | diagnostics     |       yes |        yes |          no | `schema_ok`, `index_ok`, `drift`, `recommended_action`               |
| `index_status`    | diagnostics     |       yes |        yes |          no | `counts`, `drift`, `last_sync`, `failures`                           |
| `rebuild_index`   | diagnostics/ops |        no |        yes |         yes | `rebuild_result`, `counts`, `errors`                                 |

## Suggested Tool Output Shapes

### `project_brief`

- project id/title
- summary
- top decisions
- top risks
- recent changes

### `search_memory`

- query
- hits[]
  - id
  - type
  - title
  - slug
  - score/exact_match flag
  - short excerpt
  - matched_fields[]

### `open_nodes`

- nodes[]
  - id
  - type
  - title
  - status
  - file_path_masked
  - summary
  - observations[]
  - relations[]

### `context_pack`

- seed
- brief
- included_nodes[]
- unresolved_risks[]
- recent_changes[]

### `preflight`

- schema_ok
- index_ok
- drift_detected
- recommended_action

## Input Contract Principles

- identity inputs should prefer `id`, then slug/title/alias resolution
- list inputs should be explicit arrays, not comma-separated strings
- write tools must reject ambiguous node resolution
- diagnostics tools should support bounded scope arguments where large output is possible

## Write Semantics

### Create

- write canonical file
- then sync index
- never create DB-only entity

### Update

- patch only supported regions
- preserve canonical ordering of sections
- reject unsupported transformations explicitly
- reject malformed target note before mutation
- never silently rewrite user-authored prose outside supported regions

### Link / Unlink

- operate on explicit relation entries only
- no regex-like destructive body rewriting

### Observation Append

- append to observations section in canonical form
- duplicate handling deterministic

## Manual Editing Contract

Supported manual edits:

- editing summary prose
- editing observation lines
- editing relation lines using canonical syntax
- changing tags/aliases/status/title in flat frontmatter

Unsupported or guarded edits:

- removing required frontmatter keys
- changing entity `id`
- introducing nested frontmatter objects
- rewriting note into arbitrary layout while expecting guaranteed parser compatibility

MVP rule:

- if manual edits leave canonical contract, server reports parse/validation error rather than guessing

## Conflict Model

Supported conflict classes:

- note missing
- malformed note
- stale index
- target ambiguity
- unsupported patch
- concurrent/manual drift

MVP rule:

- prefer safe rejection over risky merge

## Safety Rules

- No silent overwrite of existing file content without conflict detection
- No DB-only mutations that cannot be projected back to vault
- No deletion without clear target resolution
- Sensitive paths/content should be maskable in outputs
- No automatic note rewrites outside explicitly targeted sections

## Diagnostics Model

### `memory_status`

- high-level health snapshot

### `preflight`

- compatibility and risk checks before normal use

### `index_status`

- derived-state details for debugging

### `rebuild_index`

- controlled recovery action

## Risk Register

| Risk                          | Why it matters                        | MVP mitigation                                |
| ----------------------------- | ------------------------------------- | --------------------------------------------- |
| Hidden session state in tools | behavior becomes non-deterministic    | explicit project binding + structured results |
| Title rename drift            | links and identity can diverge        | stable id/slug + rename semantics             |
| Parser heuristic creep        | sync becomes fragile                  | canonical schema + validation errors          |
| Search disappointment         | pressure to bloat MVP with vectors    | FTS5-first boundary                           |
| Human distrust                | MCP overwrites notes unexpectedly     | limited write regions + conflict rejection    |
| Ops opacity                   | local-first system fails mysteriously | status/preflight/index diagnostics + rebuild  |

## Traceability to Requirements

- FR-1 -> project binding, addressing model, tool scoping
- FR-2 -> canonical note model, frontmatter/body rules, manual editing contract
- FR-3 -> sync model, transaction rules, drift semantics, rebuild principle
- FR-4 -> read tools and output shapes
- FR-5 -> write semantics and safety rules
- FR-6 -> bootstrap/context outputs
- FR-7 -> diagnostics model and risk register
- FR-8 -> MCP contract rules and tool contract table

## Validation Strategy

- parser tests
- file-to-index sync tests
- MCP contract tests
- recovery/preflight tests
- Obsidian compatibility smoke checks on produced Markdown files
- acceptance traceability review

## Alternatives Considered

### DB-first graph backend

- rejected for MVP
- too much operational weight
- weak human inspectability

### Session transcript memory as canonical layer

- rejected for MVP
- too noisy
- wrong truth granularity

### Vector-first retrieval in MVP

- rejected for MVP
- increases infra and tuning burden
- not needed before canonical/sync discipline is proven

## Post-MVP Extension Slots

- semantic/vector retrieval
- session-memory bridge
- mempalace-style wake-up enhancer
- compression/summarization layer
- cloud sync
- optional `memory://` resource/URI layer

These are extension slots only. They must not distort canonical layer.
