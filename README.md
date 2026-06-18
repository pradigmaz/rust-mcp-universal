# rust-mcp-universal

`rust-mcp-universal` (RMU) - локальный MCP-сервер для индексации, поиска и навигации по коду.

Он нужен агенту, который заходит в незнакомый репозиторий и должен быстро понять:

- какие файлы важны для задачи;
- где лежат символы, ссылки и связанные файлы;
- почему retrieval выбрал именно эти кандидаты;
- где в проекте накапливается structural risk;
- свежий ли локальный индекс.

RMU универсален: он не подгоняется под один стек, один репозиторий или один стиль кода.

## Возможности

- локальный SQLite-индекс в `.rmu/index.db`;
- FTS-поиск по файлам и символам;
- navigation graph: symbols, refs, related files, call paths;
- task bootstrap для агентов через `agent_bootstrap`;
- quality surface: `rule_violations`, `quality_hotspots`, `quality_snapshot`;
- privacy-oriented output: пути и чувствительные строки не должны утекать в ответ без необходимости;
- fresh launcher, который пересобирает stale binary перед запуском MCP.

## Требования

- Rust `1.85+`;
- MCP-клиент с поддержкой stdio servers.

## Сборка

```bash
cargo build --release -p rmu-mcp-server
```

Проверка:

```bash
target/release/rmu-mcp-server --help
```

На Windows бинарник будет `target/release/rmu-mcp-server.exe`.

## Быстрый старт MCP

Обычный путь:

1. MCP-клиент запускает RMU через fresh launcher.
2. RMU получает workspace root из MCP `initialize`.
3. Агент начинает с `agent_bootstrap` с `profile=fast`.
4. Если `deepen_available=true`, агент добирает контекст через `profile=full` или navigation tools.

`set_project_path` - fallback. Он нужен, если клиент не передал workspace root или надо вручную сменить проект.

## Подключение

### Kilo Code

Windows:

```json
{
  "mcpServers": {
    "rmu-universal": {
      "command": "E:\\\\path\\\\to\\\\rust-mcp-universal\\\\scripts\\\\rmu-mcp-server-fresh.cmd",
      "args": [],
      "disabled": false,
      "alwaysAllow": []
    }
  }
}
```

Linux/macOS:

```json
{
  "mcpServers": {
    "rmu-universal": {
      "command": "/path/to/rust-mcp-universal/scripts/rmu-mcp-server-fresh.sh",
      "args": [],
      "disabled": false,
      "alwaysAllow": []
    }
  }
}
```

### Codex

Установить fresh binary в `~/.codex/bin/rmu-mcp-server`:

```bash
bash scripts/install-codex-rmu-bridge.sh
```

Windows:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/install-codex-rmu-bridge.ps1
```

### WSL paths

Если RMU запущен из Windows, а проект лежит в WSL, передавайте root как:

- `\\wsl.localhost\<Distro>\home\<user>\repo`;
- `file://wsl.localhost/<Distro>/home/<user>/repo`.

Если RMU запущен внутри WSL, используйте обычный Linux path.

## Основные MCP tools

Старт и индекс:

- `agent_bootstrap` - главный вход для агента: task-aware обзор проекта;
- `workspace_brief` - короткий снимок индекса и quality summary;
- `index_status` - состояние индекса;
- `index` - построить/обновить индекс;
- `semantic_index` - подготовить semantic layer, если он включен;
- `scope_preview` - посмотреть, что попадет в индекс;
- `install_ignore_rules` - добавить RMU-managed ignore block;
- `db_maintenance` - обслуживание базы;
- `delete_index` - удалить локальный индекс.

Навигация:

- `symbol_lookup_v2`;
- `symbol_references_v2`;
- `symbol_body`;
- `related_files_v2`;
- `call_path`;
- `route_trace`;
- `constraint_evidence`;
- `concept_cluster`;
- `contract_trace`;
- `divergence_report`.

Поиск и отчеты:

- `search_candidates`;
- `semantic_search`;
- `query_report`;
- `query_benchmark`;
- `build_context_under_budget`;
- `context_pack`.

Quality и privacy:

- `rule_violations`;
- `quality_hotspots`;
- `quality_snapshot`;
- `sensitive_data`;
- `signal_memory`;
- `mark_signal_memory`.

## Agent flow

Минимальный сценарий:

```text
agent_bootstrap(profile=fast, query="<task>", auto_index=true)
```

Дальше:

- если кандидатов хватает - читать найденные файлы;
- если `deepen_available=true` - вызвать `agent_bootstrap(profile=full)` или точечные navigation tools;
- если нужен audit retrieval - вызвать `query_report`;
- если задача про качество/рефакторинг - начать с `quality_hotspots` и `rule_violations`.

## Индекс

RMU хранит служебные файлы в `.rmu/`.

Основные таблицы:

- files/chunks FTS для lexical retrieval;
- `symbols` и `symbols_fts` для поиска по именам;
- refs/edges для навигационного графа;
- quality tables для risk и hotspot surfaces.

Служебные каталоги RMU можно добавить в `.gitignore` через `install_ignore_rules`.

## Структура

```text
crates/core        indexing, retrieval, schema, ranking, quality
crates/mcp-server  MCP/JSON-RPC stdio server
schemas            JSON schemas for tool outputs
scripts            fresh launchers and local install helpers
docs               durable project docs
```

## Разработка

Форматирование:

```bash
cargo fmt --all --check
```

Тесты:

```bash
cargo test -p rmu-core -p rmu-mcp-server
```

Lint:

```bash
cargo clippy -p rmu-core -p rmu-mcp-server --all-targets -- -D warnings
```

Production build:

```bash
cargo build --release -p rmu-mcp-server
```

## Документация

- `docs/quality-metrics-guide-ru.md` - как читать quality surface;
- `schemas/` - machine-readable output contracts;
- `md/plans/` - локальные рабочие планы, не для git.

## Лицензия

MIT
