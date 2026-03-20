# AGENTS.global.example.md

## MCP navigation contract

- For MCP navigation, use only `symbol_lookup_v2`, `symbol_references_v2`, `related_files_v2`.
- Read navigation results from `result.structuredContent.hits`.
- Treat `symbol_lookup`, `symbol_references`, `related_files` as compatibility-only.

Пример глобального `AGENTS.md` (уровень `~/.codex/AGENTS.md`) для работы с RMU.

## 1) RMU bootstrap (обязательно)

- Global MCP server id: `rmu-universal`.
- Для coding/review/planning по codebase сначала использовать MCP `rmu-universal`.
- Не просить человека запускать RMU вручную: агент делает MCP/CLI вызовы сам.

Порядок bootstrap на репозиторий:

1. `tools/call -> set_project_path` (absolute repo root)
2. `tools/call -> agent_bootstrap` с параметрами:
   - `query` (optional)
   - `limit=20` (min 1)
   - `semantic=false` (или `true` для intent-level retrieval)
   - `max_chars=12000` (min 256)
   - `max_tokens=3000` (min 64)

## 2) Политика токен-эффективности

- Предпочитать one-shot `agent_bootstrap`.
- Гранулярные вызовы (`search_candidates`, `build_context_under_budget`, `query_report`) использовать только для уточнения активного расследования.

## 3) Fallback при недоступном MCP

- Если `agent_bootstrap` падает из-за формы аргументов, исправить payload (`arguments` должен быть object) и повторить.
- Если MCP недоступен/таймаутится, выполнять CLI fallback от лица агента:
  - `rmu --project-path <repo> --json agent --query "<intent>" --limit 20`

## 4) Scope и индексация

- Паттерны `include_paths` / `exclude_paths`:
  - `*`, `?`, `**`, `[]`, `{a,b,c}`, `@(a|b)`, `?(a|b)`, `+(a|b)`, `*(a|b)`, `!(a|b)`
- Перед изменением `include_paths` / `exclude_paths` сначала запускать `scope_preview` (MCP) или `scope-preview` (CLI), чтобы проверить effective scope и candidate paths.
- `index` — алиас `semantic_index`.
- При `reindex=false` scoped индексация не должна удалять записи вне scope.
- Не подключать внешний glob-crate для этого слоя без явного запроса.

## 5) Безопасность операций

- Удаление индекса только с явным подтверждением:
  - CLI: `rmu delete-index --yes`
  - MCP: `delete_index {"confirm": true}`

## 6) MCP/JSON-RPC контракт

- request-shape errors в `tools/call` -> JSON-RPC `-32602`
- batch JSON-RPC unsupported -> `-32600`
- `initialize` -> серверный `PROTOCOL_VERSION`
- framing/parse errors -> parse error, при этом серверный цикл продолжает работу
