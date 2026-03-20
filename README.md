# rust-mcp-universal

`rust-mcp-universal` — локальный движок индексации и поиска по кодовой базе на Rust.
Проект даёт один и тот же retrieval-контур для CLI и MCP-сервера: индексирование, поиск, навигацию по символам и подготовку контекста для ИИ-агентов.

## Что умеет

- индексировать кодовый проект в локальную базу `.rmu/index.db`;
- работать в профилях индексации, включая code-only `mixed`;
- искать по коду лексически и семантически;
- поднимать summary по репозиторию через `workspace_brief`;
- собирать агентский стартовый контекст через `agent_bootstrap`;
- объяснять retrieval-пайплайн через `query_report`;
- находить символы и связи через `symbol_lookup_v2`, `symbol_references_v2`, `related_files_v2`;
- показывать effective scope до индексации через `scope_preview`.

## Когда это полезно

- нужно быстро понять устройство незнакомого репозитория;
- нужно дать ИИ-агенту короткий и релевантный контекст вместо грубого чтения файлов;
- нужно искать по смыслу, а не только по точному текстовому совпадению;
- нужно одинаковое поведение из терминала и из MCP-клиента.

## Требования

- Rust `1.85` или новее

## Сборка

Команда одинакова для Linux, macOS и Windows:

```bash
cargo build --release -p rmu-cli -p rmu-mcp-server
```

После сборки бинарники будут лежать в `target/release/`:

- Linux и macOS:
  - `rmu-cli`
  - `rmu-mcp-server`
- Windows:
  - `rmu-cli.exe`
  - `rmu-mcp-server.exe`

## Быстрая проверка

```bash
target/release/rmu-cli --help
target/release/rmu-mcp-server --help
```

Если бинарники добавлены в `PATH`, можно вызывать их по имени:

```bash
rmu-cli --help
rmu-mcp-server --help
```

## Базовый запуск CLI

```bash
rmu-cli --project-path . status --json
rmu-cli --project-path . scope-preview --profile mixed
rmu-cli --project-path . semantic-index --profile mixed
rmu-cli --project-path . search --query "attendance" --limit 10
rmu-cli --project-path . agent --query "где логика авторизации" --semantic --limit 10
```

## Базовый запуск MCP

Минимальный порядок вызовов:

1. `set_project_path`
2. `workspace_brief` или `agent_bootstrap`
3. при необходимости `query_report`, `scope_preview` и навигационные `*_v2` tools

Для summary по репозиторию:

- `workspace_brief` — короткое структурированное описание проекта;
- `agent_bootstrap` — summary плюс стартовый контекст под конкретный запрос;
- `query_report` — подробный разбор retrieval-пайплайна и выбранного контекста.

## Пример конфигурации MCP-клиента

Точный путь к конфигурационному файлу зависит от клиента.
Ниже только абстрактные примеры блока конфигурации без привязки к домашним каталогам.

### JSON-вариант

```json
{
  "mcpServers": {
    "rmu-universal": {
      "command": "<путь-к-rmu-mcp-server>",
      "args": [],
      "timeout": 180000
    }
  }
}
```

### TOML-вариант

```toml
[mcp_servers.rmu-universal]
enabled = true
command = "<путь-к-rmu-mcp-server>"
args = []
startup_timeout_sec = 180
tool_timeout_sec = 180
```

## Основные MCP tools

- `set_project_path` — привязать сервер к нужному репозиторию;
- `workspace_brief` — получить summary проекта;
- `agent_bootstrap` — получить summary и релевантный стартовый контекст под задачу;
- `search_candidates` — быстрый shortlist файлов;
- `query_report` — объяснить, почему retrieval выбрал именно эти файлы;
- `scope_preview` — посмотреть, что реально попадёт в индекс;
- `symbol_lookup_v2`, `symbol_references_v2`, `related_files_v2` — навигация по коду.

Для navigation tools результат нужно читать из `result.structuredContent.hits`.

## Структура проекта

- `crates/core` — ядро индексации, retrieval и ранжирования;
- `crates/cli` — терминальный интерфейс;
- `crates/mcp-server` — MCP-сервер поверх того же ядра;
- `schemas` — JSON-схемы результатов;
- `scripts` — вспомогательные скрипты проверки.

## Что важно помнить

- индекс хранится локально в `.rmu/`;
- проект ориентирован на офлайн-работу;
- MCP полезнее обычного text-search, когда нужен не просто первый матч, а хороший кодовый контекст;
- подробные внутренние заметки и stage-артефакты намеренно не вынесены в этот `README`.
