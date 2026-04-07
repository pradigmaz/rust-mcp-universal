# Какой функционал есть в RMU

Этот файл про то, что умеет `RMU` и для каких задач это вообще держать под рукой.

## Что такое RMU

`RMU` - локальный движок для индексации, поиска и навигации по коду.

У него две точки входа:

- `rmu-cli`
- `rmu-mcp-server`

Внутри это один и тот же движок. Разница только в способе работы.

## CLI и MCP

### `rmu-cli`

Нужен для прямой работы из терминала.

Его обычно используют, когда надо:

- проверить статус проекта
- собрать или обновить индекс
- запустить поиск
- снять quality-отчёт
- прогнать benchmark

### `rmu-mcp-server`

Нужен для MCP-клиентов и агентских сценариев.

Его используют, когда теми же возможностями должен пользоваться агент, IDE или внешний клиент, который работает через tools.

По сути:

- `CLI` - ручная работа
- `MCP` - агентский и интеграционный доступ

## 1. Индексация

Без неё остальной функционал либо не работает, либо работает частично.

Основные команды:

- `preflight`
- `scope-preview`
- `semantic-index`
- `status`

### Для чего это нужно

- проверить, готов ли проект к работе
- понять, что вообще попадёт в индекс
- пересобрать индекс после крупных изменений

## 2. Быстрый обзор проекта

Это стартовый слой, когда надо быстро понять проект сверху.

Основные команды:

- `brief`
- `workspace_brief`
- `agent_bootstrap`

### Для чего это нужно

Этот слой нужен, когда открываешь чужой репозиторий и сначала хочешь понять:

- какие здесь основные директории
- какой стек
- где лежат главные точки входа
- с какого места разумно начинать разбор

### Agent-facing contract: modes, provenance, degradation

`agent_bootstrap` и `query_report` уже не только heuristic helpers. У них есть явный agent-facing contract.

Что surface обещает наружу:

- `resolved_mode` - какой intent mode реально сработал
- `mode_source` - откуда он взялся: `explicit`, `inferred`, `default`
- `provenance` - canonical vocabulary для происхождения сигнала
- `degradation_reasons` - machine-readable причины, почему surface что-то урезал, пропустил или отдал fallback
- `deepen_available` и `deepen_hint` - явный путь, как углубить ответ

#### Intent modes

- `entrypoint_map`
  - Обещание: показать entrypoints, API boundary и ближайший service layer.
  - Surface держит видимыми маршруты, handlers, entrypoint-файлы, а support/docs artifacts не должны всплывать первыми.
- `test_map`
  - Обещание: держать рядом test surface и исполнимое покрытие вокруг темы.
  - Surface поднимает tests рядом с feature/service/API, а не только production код.
- `review_prep`
  - Обещание: собрать cross-layer поверхность перед review или change-impact разбором.
  - Surface тянет backend/frontend/database/tests и не ограничивается одним слоем.
- `api_contract_map`
  - Обещание: подсветить request/response boundary, routes/endpoints и соседний service contract.
  - Surface приоритизирует API- и schema-adjacent точки, а не произвольные внутренние модули.
- `runtime_surface`
  - Обещание: показать runtime actors и execution surface.
  - Surface держит видимыми runtime/module/mixin/hook/worker-like файлы.
- `refactor_surface`
  - Обещание: показать зоны, где change-impact и refactor risk будут максимальны.
  - Surface тянет service/domain/orchestration hubs и соседние tests, а не только entrypoints.

#### Canonical provenance vocabulary

- `basis`
  - `indexed`, `preview_fallback`, `graph_derived`, `heuristic`, `mixed`
- `derivation`
  - имя surface, который собрал итоговый вывод, например `query_report`, `agent_query_bundle`, `investigation_summary`
- `freshness`
  - `index_snapshot`, `live_read`, `unknown`
- `strength`
  - `strong`, `moderate`, `weak`, `fallback_only`
- `reasons`
  - короткие machine-readable markers, которые объясняют, почему provenance именно такой

Эта vocabulary считается canonical. Новые agent-facing surfaces должны переиспользовать её, а не заводить локальные ad-hoc поля вроде собственного `source_kind + confidence + freshness` набора без маппинга.

#### Bootstrap profiles and degradation contract

- `fast`
  - Default cheap surface.
  - Даёт `brief + hits + context`, но не включает report и investigation summary.
- `investigation_summary`
  - Даёт bootstrap surface плюс отдельный embedded investigation summary.
- `report`
  - Даёт bootstrap surface плюс full `query_report`.
- `full`
  - Даёт и report, и отдельный investigation summary.

Expected degradation vocabulary:

- `semantic_fail_open`
- `semantic_low_signal_skip`
- `chunk_preview_fallback`
- `budget_truncated`
- `profile_limited`
- `unsupported_sources_present`

Expected deepen path:

- если причина в `profile_limited`, canonical hint - rerun `agent_bootstrap` с `profile=full`
- если причина в budget, canonical hint - поднять `max_chars` или `max_tokens`
- если причина в weak/low-signal semantic path, canonical hint - сузить query или передать explicit `mode`
- если причина в preview fallback или unsupported sources, canonical hint - обновить индекс или перейти в deeper investigation tools

Важно: в текущем public contract деградация описана через `profile`, budget, semantic/fallback и unsupported-sources причины. Отдельный transport-level timeout policy не оформлен как самостоятельный внешний knob и не должен маскироваться ad-hoc полями.

## 3. Поиск

Это слой для поиска логики и связанных фрагментов кода.

Основные команды:

- `search`
- `semantic-search`
- `query-report`

### Для чего это нужно

Обычный поиск хорошо работает, когда ты знаешь точное имя. Если точного имени нет, он быстро перестаёт помогать.

`RMU` полезен, когда у тебя запрос уровня:

- где логика авторизации
- где считаются отчёты
- где проверяются права

В таких случаях смысловой поиск даёт более внятную стартовую точку.

## 4. Навигация по коду

Этот слой нужен после поиска. Он показывает связи вокруг найденного места.

Основные команды:

- `symbol-lookup`
- `symbol_references`
- `related-files`
- `call_path`

### Для чего это нужно

Одна найденная функция редко даёт полную картину. Обычно дальше надо понять:

- где она объявлена
- кто её вызывает
- что лежит рядом по зависимостям
- через какие вызовы проходит путь

Навигация нужна именно для этого.

## 5. Investigation surface

Это слой для более точного разбора одного механизма или одной темы.

Основные команды:

- `symbol-body`
- `route-trace`
- `constraint-evidence`
- `concept-cluster`
- `divergence-report`
- `investigation-benchmark`

### Для чего это нужно

Этот слой нужен, когда уже мало просто найти файл. Надо понять, как именно работает кусок системы.

Типичные задачи:

- вытащить тело символа
- проследить маршрут выполнения
- найти ограничения и guards
- собрать несколько близких вариантов вокруг одной темы
- увидеть расхождения в объяснении

## 6. Quality surface

Это слой для structural debt и hotspot-анализа.

Основные команды:

- `rule-violations`
- `quality-hotspots`
- `quality-snapshot`

### Для чего это нужно

Этот слой помогает понять:

- какие файлы уже перегружены
- где появился хаб зависимостей
- какой слой первым просится на cleanup
- откуда лучше начинать рефакторинг

Он не ищет runtime-баги. Он показывает проблемные зоны в структуре проекта.

## 7. Benchmark

Это слой для проверки самого качества `RMU` после изменений.

Основные команды:

- `query-benchmark`
- `investigation-benchmark`

### Для чего это нужно

Если меняется retrieval, ranking или investigation-логика, нужен повторяемый способ проверить, стало лучше или хуже.

Benchmark нужен, чтобы:

- не ломать уже рабочее поведение
- сравнивать новые изменения с baseline

## Когда что использовать

- нужен быстрый ручной проход по проекту - `CLI`
- нужен доступ для агента или клиента - `MCP`
- нужен стартовый снимок проекта - `brief` или `workspace_brief`
- нужен поиск по смыслу - `search` и `semantic-search`
- нужны связи вокруг найденного места - navigation tools
- нужен разбор одного механизма - investigation surface
- нужен приоритетный список проблемных зон - quality surface
- нужен before/after след по debt wave и regression gate - `quality-snapshot`

## Contract maintenance

Если меняется semantics у `mode`, `profile`, canonical provenance vocabulary или `degradation_reasons`, надо обновлять:

- `docs/rmu-functionality-guide-ru.md`
- summary-описание surface в `README.md`

В этой волне `README.md` намеренно не менялся по прямому ограничению задачи, но trigger на его синхронизацию остаётся обязательным.

## Чего RMU не делает

`RMU` не заменяет чтение кода и не принимает архитектурные решения за человека.

Его задача проще: быстро довести тебя до нужных мест в проекте и сократить время на первый разбор.
