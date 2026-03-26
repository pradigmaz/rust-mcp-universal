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

1. MCP-клиент поднимает `rmu-universal` через fresh launcher
2. сервер сам привязывается к workspace root из MCP `initialize`
3. клиент вызывает `workspace_brief` или `agent_bootstrap`
4. при необходимости используются `query_report`, `scope_preview` и navigation tools

`set_project_path` теперь fallback, а не основной путь. Он нужен только если клиент не передал workspace roots или если надо вручную переопределить auto-bind.

Что чаще всего используют:

- `workspace_brief` - короткий снимок проекта
- `agent_bootstrap` - снимок проекта плюс стартовый контекст под задачу
- `query_report` - объяснение retrieval-пайплайна
- `scope_preview` - проверка будущего индекса
- `symbol_lookup_v2`, `symbol_references_v2`, `related_files_v2` - навигация по коду
- `rule_violations`, `quality_hotspots` - quality-поверхность

Для navigation tools основной результат лежит в `structuredContent.hits`.

## Подключение к MCP

Рекомендуемый вариант для Kilo Code и похожих клиентов: указывать fresh launcher, а не напрямую `rmu-mcp-server`. Это закрывает stale-binary сценарий и не требует прописывать `--project-path` в MCP-конфиге.

### Kilo Code `mcp_settings.json`

Windows:

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

Linux и macOS:

```json
{
  "mcpServers": {
    "rmu-universal": {
      "type": "stdio",
      "command": "/absolute/path/to/checkout/scripts/rmu-mcp-server-fresh.sh",
      "args": [],
      "disabled": false,
      "alwaysAllow": []
    }
  }
}
```

Fresh launcher:

- Windows: `scripts/rmu-mcp-server-fresh.cmd`
- Linux/macOS: `scripts/rmu-mcp-server-fresh.sh`

Оба launcher'а перед стартом завершают все процессы `rmu-mcp-server` из `target/` этого же checkout, при необходимости пересобирают release binary, затем публикуют отдельную runtime-копию в `target/runtime/` и только потом запускают новый foreground-процесс. Это закрывает сценарий, когда индекс уже мигрирован новым кодом, а MCP-клиент всё ещё поднимает старый бинарь, и заодно убирает lock на `target/release/rmu-mcp-server`, пока сервер работает.

Сервер принимает `2025-06-18`, `2025-03-26` и `2024-11-05`, чтобы не падать на клиентах с более старым MCP handshake.

### Codex (`~/.codex/bin/rmu-mcp-server`)

Для Codex надёжнее использовать не bridge, а обычный standalone binary, который installer копирует в `~/.codex/bin/rmu-mcp-server`. Это убирает отдельный слой stdio-proxy между Codex и RMU и не требует поиска checkout'а по дискам.

Установка из этого checkout:

- Windows: `powershell -ExecutionPolicy Bypass -File scripts/install-codex-rmu-bridge.ps1`
- Linux/macOS: `bash scripts/install-codex-rmu-bridge.sh`

Installer берёт свежий binary из этого checkout и копирует его в `~/.codex/bin`, чтобы Codex продолжал работать по стабильному пути из config, но уже без stale binary.

Если installer выводит `pending_restart=true`, это ожидаемо: он не стал перетирать активный `~/.codex/bin/rmu-mcp-server` из живой Codex-сессии. В таком состоянии нужен полный restart Codex app, потом повторный запуск installer; новый чат сам по себе не пересоздаёт app-global MCP transport.

Это убирает две проблемы сразу:

- Codex больше не держится за устаревший глобальный binary
- Codex не зависит от bridge-перепрыгивания в другой процесс перед MCP handshake

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
