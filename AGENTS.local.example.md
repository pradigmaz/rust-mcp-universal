# AGENTS.local.example.md

## MCP navigation contract

- For MCP navigation, use only `symbol_lookup_v2`, `symbol_references_v2`, `related_files_v2`.
- Read navigation results from `result.structuredContent.hits`.
- Treat `symbol_lookup`, `symbol_references`, `related_files` as compatibility-only.

Пример локального `AGENTS.md` (уровень репозитория), который дополняет глобальный.

## 1) Приоритет для этого проекта

- Для retrieval/index задач использовать `rmu-universal` в режиме MCP-first.
- Если MCP недоступен, агент делает CLI fallback сам, без запроса к человеку.

## 2) Обязательный порядок работы

1. `set_project_path` (корень текущего репозитория)
2. `agent_bootstrap` (`limit=20`, `max_chars=12000`, `max_tokens=3000`, `semantic` по задаче)
3. При необходимости уточнения: `search_candidates` / `build_context_under_budget` / `query_report`

## 3) Политика контекста

- Для intent-level запросов использовать гибрид `lexical + semantic`.
- Контекст собирать только в пределах явного бюджета (`max_chars`, `max_tokens`).
- Отдавать предпочтение shortlist высокого качества вместо длинного сырого контекста.

## 4) Scope и destructive операции

- Разрешенные scope-паттерны:
  - `*`, `?`, `**`, `[]`, `{a,b}`, `@(…)`, `?(…)`, `+(…)`, `*(…)`, `!(…)`
- Перед изменением `include_paths` / `exclude_paths` запускать `scope_preview` или `scope-preview`, чтобы проверить effective scope и candidate buckets.
- Удаление индекса только с явным подтверждением:
  - CLI: `rmu delete-index --yes`
  - MCP: `delete_index {"confirm": true}`

## 5) Формат результата для пользователя

- Кратко: что найдено, почему это релевантно, какой следующий шаг.
- Не перекладывать запуск RMU на пользователя.
