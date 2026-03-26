# rust-mcp-universal

`rust-mcp-universal` - локальный движок индексации, поиска и навигации по кодовой базе на Rust.

Проект даёт одно и то же ядро для двух поверхностей:

- `rmu-cli` для работы из терминала
- `rmu-mcp-server` для MCP-клиентов и агентских сценариев

`RMU` сделан в первую очередь для агентов.

Его задача простая: дать агенту нормальную рабочую поверхность для разбора репозитория. Не голый текстовый поиск, а локальный индекс, поиск по смыслу, навигацию по символам и связям, сбор контекста, investigation-инструменты и quality-сигналы.

Это нужно там, где агенту мало просто найти строку. Обычно надо понять, какие файлы относятся к задаче, как они связаны между собой, где точка входа, как проходит маршрут вызова и в каких местах код уже тяжёлый.

`MCP` здесь основной способ встраивания в агентский сценарий. `CLI` - тот же движок, только с прямым вызовом из терминала. Это не отдельный режим и не другая логика работы.

## Что умеет

- индексировать репозиторий в локальную базу `.rmu/index.db`
- искать по коду лексически и семантически
- строить короткий обзор проекта через `brief` и `workspace_brief`
- находить символы, ссылки и связанные файлы
- показывать, что именно попадёт в индекс, ещё до запуска индексации
- объяснять, почему retrieval выбрал именно эти файлы
- поднимать quality-отчёты и hotspots по файлам и директориям
- отдавать тот же функционал через MCP

## Когда это полезно

- нужно быстро разобраться в незнакомом репозитории
- нужно дать агенту короткий и релевантный стартовый контекст
- нужен поиск по смыслу, а не только по точному совпадению строки
- нужно одинаковое поведение из терминала и из MCP-клиента

## Требования

- Rust `1.85` или новее

## Сборка

```bash
cargo build --release -p rmu-cli -p rmu-mcp-server
```

После сборки бинарники лежат в `target/release/`.

- Linux и macOS:
  - `rmu-cli`
  - `rmu-mcp-server`
- Windows:
  - `rmu-cli.exe`
  - `rmu-mcp-server.exe`

Проверка:

```bash
target/release/rmu-cli --help
target/release/rmu-mcp-server --help
```

Если бинарники лежат в `PATH`, можно вызывать их по имени:

```bash
rmu-cli --help
rmu-mcp-server --help
```

## Быстрый старт с CLI

Минимальный сценарий обычно выглядит так:

```bash
rmu-cli --project-path . --json preflight
rmu-cli --project-path . --json scope-preview --profile mixed
rmu-cli --project-path . --json semantic-index --profile mixed
rmu-cli --project-path . --json brief
rmu-cli --project-path . --json search --query "attendance" --limit 10
```

Если нужен агентский стартовый пакет:

```bash
rmu-cli --project-path . --json agent --query "где логика авторизации" --semantic --limit 10
```

## Быстрый старт с MCP

Обычно порядок такой:

1. `set_project_path`
2. `workspace_brief` или `agent_bootstrap`
3. при необходимости `query_report`, `scope_preview` и navigation tools

Что чаще всего используют:

- `workspace_brief` - короткий снимок проекта
- `agent_bootstrap` - снимок проекта плюс стартовый контекст под задачу
- `query_report` - объяснение retrieval-пайплайна
- `scope_preview` - проверка будущего индекса
- `symbol_lookup_v2`, `symbol_references_v2`, `related_files_v2` - навигация по коду
- `rule_violations`, `quality_hotspots` - quality-поверхность

Для navigation tools основной результат лежит в `structuredContent.hits`.

## Пример MCP-конфига

Ниже только форма блока. Точный путь зависит от клиента и твоего checkout.

### JSON

```json
{
  "mcpServers": {
    "rmu-universal": {
      "command": "<path-to-rmu-mcp-server>",
      "args": [],
      "timeout": 180000
    }
  }
}
```

### Kilo Code `mcp_settings.json`

Для клиентов, которые читают локальный JSON-конфиг наподобие `mcp_settings.json`, на Windows обычно надёжнее запускать `.cmd` через `cmd /c`.

```json
{
  "mcpServers": {
    "rmu-universal": {
      "type": "stdio",
      "command": "cmd",
      "args": [
        "/c",
        "<path-to-checkout>\\scripts\\rmu-mcp-server-fresh.cmd"
      ],
      "disabled": false,
      "alwaysAllow": []
    }
  }
}
```

Сервер сейчас принимает `2025-06-18`, `2025-03-26` и `2024-11-05`, чтобы не падать на клиентах с более старым MCP handshake.

### TOML

```toml
[mcp_servers.rmu-universal]
enabled = true
command = "<path-to-rmu-mcp-server>"
args = []
startup_timeout_sec = 180
tool_timeout_sec = 180
```

## Windows launcher

На Windows лучше указывать в MCP-конфиге не `rmu-mcp-server.exe`, а launcher:

`scripts/rmu-mcp-server-fresh.cmd`

Он перед стартом завершает старый процесс `rmu-mcp-server.exe` из этого же checkout и только потом запускает новый foreground-процесс. Это помогает не держать висящие старые экземпляры после rebuild.

Пример:

```json
{
  "mcpServers": {
    "rmu-universal": {
      "command": "<path-to-checkout>\\scripts\\rmu-mcp-server-fresh.cmd",
      "args": [],
      "timeout": 180000
    }
  }
}
```

## Полезные команды

Общий статус:

```bash
rmu-cli --project-path . --json status
rmu-cli --project-path . --json brief
rmu-cli --project-path . --json preflight
```

Индексация:

```bash
rmu-cli --project-path . --json scope-preview --profile mixed
rmu-cli --project-path . --json semantic-index --profile mixed
```

Поиск и навигация:

```bash
rmu-cli --project-path . --json search --query "attendance"
rmu-cli --project-path . --json semantic-search --query "authorization flow"
rmu-cli --project-path . --json symbol-lookup --name "AuthService"
rmu-cli --project-path . --json related-files --path "src/auth/service.ts"
```

Investigation surface:

```bash
rmu-cli --project-path . --json symbol-body --seed "src/lib.rs:1" --seed-kind path_line --auto-index
rmu-cli --project-path . --json route-trace --seed "resolve_origin" --seed-kind query --auto-index
rmu-cli --project-path . --json constraint-evidence --seed "resolve_origin" --seed-kind query --auto-index
rmu-cli --project-path . --json divergence-report --seed "resolve_origin" --seed-kind query --auto-index
```

Quality:

```bash
rmu-cli --project-path . --json rule-violations
rmu-cli --project-path . --json quality-hotspots
rmu-cli --project-path . --json quality-hotspots --aggregation directory
```

## Авто `.gitignore`

При первом пользовательском входе через CLI и через `set_project_path` сервер может создать корневой `.gitignore`, если его ещё нет, и поддерживать в нём небольшой RMU-managed блок для служебных каталогов.

Туда обычно попадают:

- `.rmu/`
- `.codex/`
- `.qodo/`
- `.idea/`
- `.vscode/`
- `.DS_Store`
- `Thumbs.db`

Пользовательские правила не удаляются. `RMU` обновляет только свой помеченный блок.

## Структура проекта

- `crates/core` - ядро индексации, retrieval и ранжирования
- `crates/cli` - терминальный интерфейс
- `crates/mcp-server` - MCP-сервер поверх того же ядра
- `schemas` - JSON-схемы результатов
- `scripts` - служебные скрипты
- `docs` - документация и рабочие планы

## Разработка

Сборка:

```bash
cargo build --release -p rmu-cli -p rmu-mcp-server
```

Тесты:

```bash
cargo test -p rmu-core -p rmu-cli -p rmu-mcp-server
```

Линтер:

```bash
cargo clippy -p rmu-core -p rmu-cli -p rmu-mcp-server --all-targets -- -D warnings
```

Быстрая локальная проверка:

```bash
cargo run --locked -p rmu-cli -- --project-path . --json status
```

## Куда идти дальше

- Какой функционал есть в RMU и когда он нужен: [docs/rmu-functionality-guide-ru.md](docs/rmu-functionality-guide-ru.md)
- Quality и hotspot-метрики: [docs/quality-metrics-guide-ru.md](docs/quality-metrics-guide-ru.md)

## Что важно помнить

- индекс хранится локально в `.rmu/`
- проект рассчитан на локальную и в основном офлайн-работу
- `RMU` не заменяет чтение кода, а помогает быстрее дойти до нужных мест
- подробные внутренние планы и stage-артефакты намеренно не выносятся в README

## Лицензия

MIT
