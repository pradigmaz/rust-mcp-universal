# rust-mcp-universal

`rust-mcp-universal` - локальный движок индексации, поиска и навигации по кодовой базе на Rust.

Проект даёт одно ядро через MCP-поверхность:

- `rmu-mcp-server` для MCP-клиентов и агентских сценариев

`RMU` сделан в первую очередь для агентов.

Его задача простая: дать агенту нормальную рабочую поверхность для разбора репозитория. Не голый текстовый поиск, а локальный индекс, поиск по смыслу, навигацию по символам и связям, сбор контекста, investigation-инструменты и quality-сигналы.

Это нужно там, где агенту мало просто найти строку. Обычно надо понять, какие файлы относятся к задаче, как они связаны между собой, где точка входа, как проходит маршрут вызова и в каких местах код уже тяжёлый.

`MCP` здесь основной способ встраивания в агентский сценарий.

## Что умеет

- индексировать репозиторий в локальную базу `.rmu/index.db`
- искать по коду лексически и семантически
- строить короткий обзор проекта через `brief` и `workspace_brief`
- находить символы, ссылки и связанные файлы
- показывать, что именно попадёт в индекс, ещё до запуска индексации
- объяснять, почему retrieval выбрал именно эти файлы
- поднимать quality-отчёты и hotspots по файлам и директориям
- показывать structural risk через `rule_violations`, `quality_hotspots` и `quality_snapshot`
- отдавать функционал через MCP

## Когда это полезно

- нужно быстро разобраться в незнакомом репозитории
- нужно дать агенту короткий и релевантный стартовый контекст
- нужен поиск по смыслу, а не только по точному совпадению строки
- нужен агентский доступ к локальному индексу и quality-сигналам

## Quality Surface

Wave 3 расширяет quality surface от symptom-only сигналов к structural risk:

- `layering` для зон, направлений зависимостей и cross-layer нарушений
- `git_risk` для churn, ownership concentration и change coupling
- `test_risk` для статического test evidence вокруг public и hotspot-путей

Отдельных top-level команд не добавлялось. Эти сигналы выходят через уже существующие `rule_violations`, `quality_hotspots` и `quality_snapshot`.

Для quality policy используется версия `4`. Ключ `structural` заменён на `layering`, а рядом появились блоки `git_risk` и `test_risk`.

## Требования

- Rust `1.85` или новее

## Сборка

```bash
cargo build --release -p rmu-mcp-server
```

После сборки бинарь лежит в `target/release/`.

- Linux и macOS:
  - `rmu-mcp-server`
- Windows:
  - `rmu-mcp-server.exe`

Проверка:

```bash
target/release/rmu-mcp-server --help
```

Если бинарники лежат в `PATH`, можно вызывать их по имени:

```bash
rmu-mcp-server --help
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
- `quality_snapshot` - debt-wave snapshot, baseline и regression gate

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

### WSL on Windows

Если `rmu-mcp-server` запущен из Windows, а workspace лежит в WSL, передавайте root как:

- UNC path: `\\wsl.localhost\<Distro>\home\<user>\repo`
- file URI: `file://wsl.localhost/<Distro>/home/<user>/repo`

`set_project_path` теперь принимает те же формы. Если сервер RMU запускается внутри самой WSL-среды, используйте обычную Linux/macOS настройку.

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

## Полезные MCP tools

Общий статус:

- `preflight`
- `index_status`
- `workspace_brief`
- `agent_bootstrap`

Индексация:

- `scope_preview`
- `index`
- `semantic_index`

Поиск и навигация:

- `search_candidates`
- `semantic_search`
- `symbol_lookup_v2`
- `symbol_references_v2`
- `related_files_v2`
- `call_path`

Investigation surface:

- `symbol_body`
- `route_trace`
- `constraint_evidence`
- `concept_cluster`
- `contract_trace`
- `divergence_report`

Quality:

- `rule_violations`
- `quality_hotspots`
- `quality_snapshot`

## Авто `.gitignore`

При первом пользовательском входе через `set_project_path` сервер может создать корневой `.gitignore`, если его ещё нет, и поддерживать в нём небольшой RMU-managed блок для служебных каталогов.

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
- `crates/mcp-server` - MCP-сервер поверх того же ядра
- `schemas` - JSON-схемы результатов
- `scripts` - служебные скрипты
- `docs` - документация и рабочие планы

## Разработка

Сборка:

```bash
cargo build --release -p rmu-mcp-server
```

Тесты:

```bash
cargo test -p rmu-core -p rmu-mcp-server
```

Линтер:

```bash
cargo clippy -p rmu-core -p rmu-mcp-server --all-targets -- -D warnings
```

## Куда идти дальше

- Какой функционал есть в RMU и когда он нужен: [docs/rmu-functionality-guide-ru.md](docs/rmu-functionality-guide-ru.md)
- Quality и hotspot-метрики: [docs/quality-metrics-guide-ru.md](docs/quality-metrics-guide-ru.md)

## Что важно помнить

- индекс хранится локально в `.rmu/`
- проект рассчитан на локальную и в основном офлайн-работу
- `RMU` не заменяет чтение кода, а помогает быстрее дойти до нужных мест
- подробные внутренние планы и stage-артефакты намеренно не выносятся в README

## Wave 4 surfaces

- `quality/dead_code` и `quality/security_smells` теперь идут как long-tail warning lanes внутри quality outputs.
- `quality/security_smells` не участвует в ordinary numeric quality score.
- `security/sensitive_data` доступен как отдельная security surface.
- repo-local signal memory хранится в `.rmu/signal-memory.json` и доступна через MCP inspect/mark flows.

## Лицензия

MIT
