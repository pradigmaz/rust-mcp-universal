# AGENTS.local.example.md

## Scope
- Local repo instructions extend global rules.
- Keep this file short; move details to `.agents/docs/<project>/README.md`.
- Put project skill routing in `.agents/skills/<project>/SKILL.md`.

## RMU MCP Flow
1. `set_project_path` when client did not bind workspace root.
2. `agent_bootstrap(profile=fast, query="<task>")`.
3. Use focused tools only when needed: `query_report`, `symbol_lookup_v2`, `related_files_v2`, `rule_violations`.

## Guardrails
- Run `scope_preview` before scoped indexing changes.
- Delete index only with explicit confirmation.
- Report what was found, why relevant, and next step.
