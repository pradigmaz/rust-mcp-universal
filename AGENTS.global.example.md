# AGENTS.global.example.md

## RMU MCP Navigation
- Prefer `agent_bootstrap(profile=fast)` as first codebase map.
- Use `symbol_lookup_v2`, `symbol_references_v2`, `related_files_v2` for navigation.
- Read navigation results from `result.structuredContent.hits`.
- Treat `symbol_lookup`, `symbol_references`, `related_files` as compatibility-only.

## Scope
- `rmu-universal` is general-purpose code intelligence.
- Do not tune global rules for one repository.
- Use repo-local `AGENTS.md` and `.agents/docs/<project>/README.md` for project details.

## Safety
- Run `scope_preview` before changing `include_paths` / `exclude_paths`.
- Delete index only with explicit confirmation:
  - CLI: `rmu delete-index --yes`
  - MCP: `delete_index {"confirm": true}`
