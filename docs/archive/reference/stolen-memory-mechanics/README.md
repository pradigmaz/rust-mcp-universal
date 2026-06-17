# Obsidian Memory MCP

Rust workspace for an Obsidian-backed project memory MCP server.

Current workspace shape:

- `crates/core`
  - Markdown note scan
  - flat frontmatter parsing
  - wikilink relation extraction
  - canonical Markdown writes
  - SQLite derived index rebuild
  - bootstrap queries, diagnostics, search, and graph reads
- `crates/mcp-server`
  - stdio MCP transport
  - JSON-RPC protocol shell
  - schema-first tool registry
  - structured tool results and errors

Current tool surface:

- Bootstrap and read:
  - `set_project`
  - `project_brief`
  - `recent_changes`
  - `decision_log`
  - `risk_hotspots`
  - `context_pack`
  - `search_memory`
  - `open_nodes`
  - `read_graph`
- Diagnostics and operations:
  - `memory_status`
  - `preflight`
  - `index_status`
  - `rebuild_index`
  - `migrate_memory_root`
- Write:
  - `create_node`
  - `update_node`
  - `link_nodes`
  - `unlink_nodes`
  - `add_observation`

Write tools mutate canonical Markdown in the vault first and then resync the derived SQLite index. They do not write DB-only canonical state.

Current write/read contract:

- `create_node` enforces one current node per concept
  - exact slug collisions are blocked
  - same-type normalized title collisions are blocked while the existing node is current/open
  - the error payload returns `candidates[]` so callers can switch to `update_node`
- `create_node` writes Obsidian-facing filenames from note titles, not from slugs
  - project hub note lives as a human-readable root file such as `Workspace.md`
  - non-project notes live under type directories with human-readable filenames such as `tasks/Fix Legacy Root Ambiguity.md`
  - historical statuses (`superseded`, `obsolete`, `closed`, `accepted`, `resolved`, `done`) rename files with a readable suffix such as `Shared Task (Superseded).md`
- `update_node` is the primary path for keeping a node current
- project graph is now materialized as `project hub -> section hubs -> leaf notes`
  - MCP always ensures a fixed set of root-level section hubs such as `Workspace Tasks.md`, `Workspace Risks.md`, and `Workspace Decisions.md`
  - section hubs are system-managed notes; MCP owns and rewrites their summary, relations, and references
  - project hub keeps explicit body links to every section hub
  - every non-project note keeps explicit body links to both the project hub and its section hub
  - section hubs remain searchable/openable, but default bootstrap surfaces do not promote them as primary memory content
- slugs remain stable machine identity in frontmatter and the derived index
- historical nodes are kept in canonical Markdown and should use statuses such as `superseded`, `obsolete`, or `closed`
- `supersedes` relations are the canonical replacement link between old and new truth
- current-oriented bootstrap tools (`project_brief`, `decision_log`, `context_pack`, `risk_hotspots`) exclude closed historical nodes by default
- historical nodes remain available through `search_memory`, `open_nodes`, and `read_graph`

Storage model:

- MCP now separates `project_root` from `memory_root`
- repo-local stable binding is persisted in `.codex/project-memory.json`
  - this pins one repo to one stable `project_key`
  - codex memory roots keep resolving to the same `$CODEX_HOME/memory/<project-key>/` even if the repository path changes later
- default mode is `codex`
  - canonical memory lives under visible central vault folder `$CODEX_HOME/memory/<project-slug>--<hash10>/`
  - open `$CODEX_HOME/memory/` as an Obsidian vault to see all project clusters in one place
- optional `project` mode writes under `<project_root>/memory/`
- `initialize` accepts optional `storageMode`
- `set_project` accepts optional `storage_mode`
- `migrate_memory_root` explicitly moves legacy canonical Markdown from project-local roots into the selected storage root without silent merge
  - ambiguous legacy roots require explicit `source_root`
  - dry runs return `candidate_sources[]` so callers can choose deterministically

Design constraints:

- Markdown vault is canonical truth
- SQLite is derived state only
- no CLI crate
- no vector-first retrieval in bootstrap
- Obsidian-facing filenames are human-readable
- project memory graph is centered on one project hub note
- folder clusters in Obsidian are represented by explicit section-hub notes, not by folders alone
- graph-visible links live in note bodies, not only in metadata

Local verification paths:

- Windows native:
  - supported when MSVC linker tools are already available or `VsDevCmd.bat` can be discovered
  - repo-local entrypoint: `.\scripts\verify-rust.ps1 -Mode Native`
- WSL:
  - stable fallback and unblocker path
  - repo-local entrypoint: `.\scripts\verify-rust.ps1 -Mode Wsl`

Quick local smoke:

- `.\scripts\smoke-mcp.ps1 -Mode Auto`

Fresh launcher for build-on-change and stale-process cleanup:

- Windows:
  - `.\scripts\obsidian-memory-mcp-server-fresh.cmd`
  - `powershell -ExecutionPolicy Bypass -File .\scripts\obsidian-memory-mcp-server-fresh.ps1`
- Linux/macOS:
  - `./scripts/obsidian-memory-mcp-server-fresh.sh`

Fresh launcher behavior:

- kills stale `obsidian-memory-mcp-server` processes started from this checkout's `target/`
- rebuilds release or debug binary only when source files changed
- publishes a runtime copy under `target/runtime/` before launch, so the foreground server does not hold locks on `target/debug` or `target/release`
- falls back from release to debug if release build is unavailable
