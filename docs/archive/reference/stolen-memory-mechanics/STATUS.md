# Current Status

## Where We Stopped

- Storage model was rewritten toward one central Obsidian-visible vault under `.codex`:
  - default MCP storage mode is now `codex`
  - default canonical memory root is now `$CODEX_HOME/memory/<project-slug>--<hash10>/`
  - `project` mode remains available as an explicit opt-in fallback under `<project_root>/memory/`
  - repo-local stable binding is now persisted in `.codex/project-memory.json`
- Obsidian visibility issue was addressed at the storage-model level:
  - canonical Markdown is intended to live in visible `memory/` folders, not hidden `.memory/`
  - the central vault to open in Obsidian is `$CODEX_HOME/memory`
  - project memory now pivots around one human-readable project hub note at the root of each project memory folder
  - project graph now materializes a fixed second layer of system-managed section hubs (`Tasks`, `Risks`, `Decisions`, `Modules`, `Progress`, `Architecture`, `Constraints`, `Artifacts`, `Glossary`)
  - note filenames are now derived from human titles, not machine slugs
  - every non-project note now keeps explicit project-hub and section-hub body links for graph visibility
- Legacy discovery and migration paths were expanded:
  - repo-root legacy layout is still detected
  - visible project-local `memory/` is now treated as a migration source
  - hidden project-local `.memory/` is now treated as a migration source
  - ambiguous legacy layouts now require explicit `source_root` on apply instead of guessing
- Test and smoke coverage was updated for the new default:
  - MCP default binding now validates `codex` storage
  - explicit `project` mode remains covered in smoke and test flows
  - MCP test harness now isolates `CODEX_HOME` into a temp directory so tests do not write into the real user vault
  - duplicate-create blocking and current-only bootstrap behavior are now covered in core + MCP tests
  - section-hub graph sync, protected required links, and system-node rejection are now covered in core + MCP tests
- Acceptance evidence for this refactor lives in:
  - `.codex/project-map/tasks/central-codex-vault.md`

## Verification State

- Windows native:
  - `powershell -ExecutionPolicy Bypass -File .\scripts\verify-rust.ps1 -Mode Auto`
  - current result: pass
- Live MCP smoke:
  - `powershell -ExecutionPolicy Bypass -File .\scripts\smoke-mcp.ps1 -Mode Auto`
  - current result: pass
- Covered behavior now includes:
  - default `set_project` binding to central `codex` storage
  - explicit `set_project(..., storage_mode = "project")`
  - rebuild, write, read, bootstrap, and diagnostics flows in both storage modes
  - `create_node` duplicate blocking with candidate payloads
  - current/open filtering in `project_brief`, `decision_log`, `context_pack`, and `risk_hotspots`
  - explicit-source migration flow for ambiguous legacy roots

## Current State

- The code now matches the desired architecture direction better:
  - one central vault for all project memories
  - per-project roots directly under `<vault>/<project-key>/`
  - visible Markdown suitable for Obsidian graph/index browsing
  - one human-readable project hub note per project memory root
  - one fixed set of human-readable section-hub notes per project memory root
  - human-readable filenames such as `Auth Module.md` instead of slug filenames
  - one current node per concept in the write path
  - historical memory retained in-place through status transitions instead of archive/delete
  - repo-to-codex mapping no longer depends only on hashing the current absolute repo path
- The repo is not yet fully migrated at the data level:
  - old notes still exist in this checkout under `D:\mcp\obsidian-mcp-memory\.memory`
  - a real migration still needs one explicit `source_root` choice because this repo currently has more than one detected legacy root
  - `.memory-mcp/` still exists here as leftover derived state from the previous layout
- This means code and tests are ready, but the real project data still needs one explicit migration step.
- Legacy project data migration now rewrites legacy `_index.md` / slug-based notes into the new Obsidian-first layout and injects missing project-hub links during the move.
- Legacy project data migration now also creates section hubs and rewrites graph links into the `project -> section -> leaf` layout.

## Next Step

- Keep validating the live repo vault against the new graph contract:
  - project hub should link to section hubs directly
  - section hubs should link to their member notes
  - bootstrap surfaces should stay focused on domain notes, not structural hubs
