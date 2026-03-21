# Quality Follow-ups

## Важно

Этот список не считается закрытым только по локальным тестам.

Новый `quality`-слой обязательно нужно **проверить на реальных проектах** для выявления косяков в процессе:

- на маленьких репозиториях;
- на средних рабочих сервисах;
- на больших монорепах;
- на проектах с нестабильной структурой файлов, частыми удалениями и инкрементальной индексацией.

Цель этой проверки:

- поймать деградации latency;
- поймать ложные `stale/degraded/unavailable` состояния;
- поймать ошибки quality-refresh, которые не видны на synthetic tests;
- убедиться, что core search/indexing/navigation не страдает при проблемах quality-подсистемы.

## Что уже сделано

### Последняя серия изменений

- `set_project_path` переведён в attach-only режим без неявной правки ignore-файлов;
- добавлен явный install-flow для ignore-правил:
  - CLI: `install-ignore-rules`;
  - MCP: `install_ignore_rules`;
  - default target: `.git/info/exclude`;
  - root `.gitignore` оставлен как явный opt-in;
- canonical ignore patterns вынесены в отдельный ресурс, а не оставлены захардкоженными в одном большом куске логики.

- quality-подсистема доведена до consumer-facing состояния:
  - наружу выведены `rule_violations` и обновлённый quality summary;
  - `workspace_brief` интегрирован с quality status/summary;
  - добавлены schema/storage/test paths для quality state;
  - обновлены MCP handlers, registry schemas и tool tests под quality surfaces.

- закрыты крупные maintainability hotspots:
  - `dispatch.rs` разрезан на подмодули по зонам ответственности;
  - `preview.rs` разрезан на `state_load`, `walk_summary`, `deleted_paths`;
  - `fusion.rs` разрезан на отдельные модули accumulation / anchors / score / types;
  - `model/core/types/index.rs` разрезан на indexing / maintenance / profiles;
  - `graph/tests.rs` и `scope_and_storage.rs` разрезаны на тематические test-модули.

- навигационный и graph-контур тоже был усилен:
  - extractor policy вынесен в отдельный модуль;
  - добавлены Python navigation integration/contract tests;
  - обновлены связанные contract tests для CLI и MCP.

- в финале были закрыты quality-gate блокеры:
  - убраны clippy-падения в `process_probe`, `engine_quality` и `utils/path`;
  - форматирование выровнено;
  - код повторно прогнан через `clippy -D warnings`.

### Финальная верификация

- `cargo fmt --all --check` проходит;
- `cargo clippy -p rmu-core -p rmu-cli -p rmu-mcp-server --all-targets -- -D warnings` проходит;
- `cargo test -p rmu-core -p rmu-cli -p rmu-mcp-server` проходит;
- sanity-check через `cargo run --locked -p rmu-cli -- --project-path . --json status` проходит.

### Miri

- установлен nightly toolchain + `miri` + `rust-src`;
- под `miri` успешно прогнаны чистые targeted tests для:
  - `rebuild_lock::tests::process_probe_parser_maps_known_tokens`;
  - `utils::path::tests::normalize_path_avoids_percent_collision_between_utf8_and_raw_bytes`.

- quality-тест через `miri` упирается не в UB текущих правок, а в ограничения инструмента:
  - сначала в isolation на `SystemTime::now`;
  - затем, при отключённой isolation, в неподдерживаемый SQLite FFI (`rusqlite` / `sqlite3_threadsafe`).

### Git-состояние

- изменения разложены на два коммита:
  - `feat: add explicit ignore installation and quality reporting`;
  - `chore: refresh benchmark and rollout baselines`;
- изменения уже запушены в `origin/main`;
- на момент последней проверки локальная `main` и `origin/main` синхронизированы, рабочее дерево чистое.

### Архитектура

- quality вынесен из core indexing pass в отдельный fail-open контур;
- core indexing теперь не должен зависеть от успешной записи quality-данных;
- quality completeness больше не участвует в core completeness;
- старый inline путь записи quality удалён.

### Quality state и поведение

- введён явный status quality-подсистемы: `ready`, `stale`, `degraded`, `unavailable`;
- `workspace_brief` теперь делает quality-only refresh вместо обычного reindex;
- `rule_violations` и quality summary больше не должны быть жёсткой точкой отказа для основного движка;
- quality-only проблемы теперь должны деградировать в статус, а не валить основной сценарий.

### Storage и schema

- добавлена отдельная таблица `file_quality_metrics`;
- quality storage отделён от core retrieval storage по смыслу, хотя остаётся в той же SQLite БД;
- quality ruleset/version вынесен в отдельный compatibility-контур;
- quality stale/missing состояние теперь должно чиниться quality-only refresh-путём.

### Rule engine

- старый монолитный quality-код разрезан на отдельные модули;
- введён модульный evaluation pipeline для text/indexed quality metrics;
- сохранены старые file-level правила по:
  - размеру файла;
  - количеству непустых строк;
  - import count;
- добавлена первая новая волна правил:
  - `max_line_length`;
  - `max_symbol_count_per_file`;
  - `max_ref_count_per_file`;
  - `max_module_dep_count_per_file`;
  - `max_graph_edge_out_count`.

### Интеграция с продуктовым слоем

- обновлены типы quality summary и rule violations;
- наружу выведен `QualityStatus`;
- `workspace_brief` и `rule_violations` обновлены под новую status-модель;
- MCP-тесты и контрактные ожидания подстроены под новый ruleset и новый status.

### Проверка

- сборка проходит;
- unit и integration тесты по `core`, `cli`, `mcp-server` проходят;
- `clippy -D warnings` проходит;
- sanity-check через `rmu-cli --json status` проходит.

## Что осталось допилить

### 1. Добить аварийные сценарии quality

Нужно добавить/усилить тесты и ручные прогоны на случаи:

- падение записи в `file_quality`;
- падение записи в `file_rule_violations`;
- падение записи в `file_quality_metrics`;
- отсутствие quality-таблиц;
- частично сломанная quality-схема;
- неконсистентные meta-ключи quality state.

Ожидаемое поведение:

- core indexing завершается успешно;
- search/navigation/query не ломаются;
- quality уходит в `degraded` или `unavailable`, но не валит основной сценарий.

### 2. Проверить и, возможно, оптимизировать `refresh_quality_only`

Сейчас core-path уже изолирован, но quality refresh всё ещё может быть дорогим на больших репозиториях.

Нужно проверить на реальных проектах:

- сколько файлов quality refresh реально перечитывает;
- как ведёт себя `workspace_brief` на больших кодовых базах;
- не возникает ли заметного UX-лага из-за stale quality и auto-refresh.

Если подтвердится проблема, следующий шаг:

- сузить refresh до реально затронутых путей;
- хранить больше пригодных для quality-refresh инкрементальных маркеров.

### 3. Вытащить `file_quality_metrics` наружу

Сейчас метрики уже считаются и пишутся, но наружу в summary/reporting выведены слабо.

Полезно добавить:

- richer summary в `workspace_brief`;
- сортировки и фильтры по metric ids в `rule_violations`;
- отдельные top hot-spots не только по violations, но и по метрикам.

Иначе часть пользы нового storage-слоя остаётся внутренней.

### 4. Решить, нужны ли line-level нарушения

Сейчас quality работает на file-level, и это правильно для текущего этапа.

Но если нужна практическая польза для рефакторинга, следующий уровень:

- line-level violations;
- region-level violations;
- указание точного места длинной строки, проблемного блока или перегруженного участка.

Это не обязательно делать сейчас, но важно не путать текущую file-level систему с полноценным linting.

### 5. Вынести policy/thresholds в конфиг

Сейчас thresholds захардкожены.

Следующий разумный шаг:

- отделить ruleset от конфигурации порогов;
- дать project-level policy без перепрошивки кода;
- сохранить при этом fail-open и совместимость quality-only refresh.

Это особенно важно перед массовой проверкой на реальных проектах, потому что реальные репозитории быстро покажут, где текущие лимиты слишком жёсткие или слишком мягкие.

## Что обязательно проверить на реальных проектах

### Поведение

- не запускается ли неожиданный полный reindex;
- не тормозит ли `workspace_brief`;
- не ломается ли `rule_violations` на частично битой quality-базе;
- не остаётся ли quality навсегда в `stale`;
- корректно ли восстанавливается `ready` после normal refresh.

### Нагрузка

- время первого quality-refresh;
- время повторного quality-refresh;
- влияние на большие репозитории;
- влияние на read-only сценарии.

### Корректность данных

- совпадают ли violations с реальной структурной проблемой файла;
- нет ли ложных срабатываний на тестовые, generated или config-файлы;
- хватает ли текущих правил для реальной пользы, а не только для synthetic coverage.

## Текущий вывод

Критического хвоста после текущего рефактора нет.

Главное, что осталось теперь, это не срочный rescue-fix, а **обкатка на реальных проектах** и добивка quality-подсистемы по результатам этой обкатки.
