# Два Детальных Roadmap: Quality Rules И Визуальный UI

## Исходные репозитории для проверки

### Для roadmap 1: quality rules

| Репозиторий      | Путь                                     | Основные языки / тип             |
| ---------------- | ---------------------------------------- | -------------------------------- |
| platforma_maga   | `D:\универ\platforma_maga`               | `TSX`, `TS`, `Python`            |
| ficbook-enhancer | `D:\scripts\ficbook_js\ficbook-enhancer` | `JS`, `JSX`, browser extension   |
| QubeForge        | `D:\scripts\QubeForge`                   | `TS`, `JS`, Vite UI              |
| QubeForgeRust    | `C:\Users\Zaikana\Desktop\QubeForgeRust` | `Rust` workspace                 |
| kiron_client     | `E:\Новая папка\kiron_client`            | `Java`, Gradle/project artifacts |

### Для roadmap 2: UI визуализация

| Репозиторий    | Путь                       | Почему нужен                                                       |
| -------------- | -------------------------- | ------------------------------------------------------------------ |
| platforma_maga | `D:\универ\platforma_maga` | большой смешанный репозиторий, где легко увидеть скрытую сложность |
| QubeForge      | `D:\scripts\QubeForge`     | более компактный TS-проект для быстрого UX-цикла                   |

### Для roadmap 3: docs indexing

| Репозиторий        | Путь                                     | Что проверять                                                        |
| ------------------ | ---------------------------------------- | -------------------------------------------------------------------- |
| platforma_maga     | `D:\универ\platforma_maga`               | `skip-if-missing`, если специальной docs-папки нет                   |
| ficbook-enhancer   | `D:\scripts\ficbook_js\ficbook-enhancer` | отсутствие ложных ошибок на небольшом JS-репозитории                 |
| QubeForge          | `D:\scripts\QubeForge`                   | nested docs-папки и удобство дальнейшего подключения к UI            |
| QubeForgeRust      | `C:\Users\Zaikana\Desktop\QubeForgeRust` | корректная работа рядом с Rust code-surface                          |
| kiron_client       | `E:\Новая папка\kiron_client`            | отсутствие деградации на Java/Gradle-репозитории                     |
| synthetic fixtures | internal test fixtures                   | сценарии `missing/flat/nested/renamed/deleted/malformed/unsupported` |

## Текущее состояние quality слоя

Уже есть:

- file-level quality snapshot;
- `workspace_brief` с `quality_summary`;
- `rule_violations` с фильтрацией, метриками и location;
- policy overrides через `rmu-quality-policy.json`;
- базовые правила:
  - `max_size_bytes`
  - `max_non_empty_lines_default`
  - `max_non_empty_lines_test`
  - `max_non_empty_lines_config`
  - `max_import_count`
  - `max_line_length`
  - `max_symbol_count_per_file`
  - `max_ref_count_per_file`
  - `max_module_dep_count_per_file`
  - `max_graph_edge_out_count`

Главный следующий шаг: перейти от "нескольких полезных file-level checks" к нормальной системе engineering hotspots и structural smells.

---

# Roadmap 1. Довести quality rules до практического уровня

## Цель

Сделать quality слой достаточно сильным, чтобы он:

- показывал реальные проблемные зоны;
- работал на разных типах репозиториев;
- не шумел бессмысленными срабатываниями;
- позволял человеку и агенту быстро видеть "где архитектурно и кодово плохо".

## Нецели

- не строить LLM-judge систему вместо детерминированных правил;
- не тащить markdown/text-heavy анализ в quality rules;
- не делать autofix как обязательную часть первой версии.

## Этап 0. Базовая матрица валидации

### Задачи

- Зафиксировать набор тестовых репозиториев и их роль.
- Добавить скрипт или check-flow, который гоняет:
  - `workspace_brief`
  - `rule_violations`
  - `brief + refresh_quality_if_needed`
  - top hotspots по нескольким сортировкам
- Собирать per-repo artifact:
  - summary JSON
  - top rules
  - top metrics
  - top hot files

### Deliverables

- единый validation playbook для 5 репозиториев;
- baseline artifacts до новых правил.

### Acceptance criteria

- ни один из 5 репозиториев не валит quality flow;
- статусы `ready/stale/degraded/unavailable` ведут себя предсказуемо;
- можно повторно сравнить до/после внедрения новых правил.

## Этап 1. Закрыть обязательные file-level hotspots

### Что добавить

- `max_function_lines`
- `max_nesting_depth`
- `max_parameters_per_function`
- `max_export_count_per_file`
- `max_class_member_count`
- `max_todo_count_per_file`

### Почему это важно

Сейчас система хорошо видит размер файла, длину строки, количество импортов и graph pressure, но плохо ловит типичный "внутри жесть":

- файл умеренно большой, но внутри одна гигантская функция;
- вложенность убивает читаемость;
- один модуль экспортирует слишком много;
- класс или компонент тащит слишком много обязанностей.

### Технический подход

- Сначала parser-light правила для `TS/JS/Python/Rust`.
- Потом language adapters для `Java`.
- Где AST нет или он слишком дорогой:
  - допускать conservative heuristic mode;
  - помечать это в `message` или `source`.

### Репозитории проверки

- `platforma_maga` для `TSX/TS/Python`
- `ficbook-enhancer` для `JS/JSX`
- `QubeForge` для `TS`
- `QubeForgeRust` для `Rust`
- `kiron_client` для `Java`

### Acceptance criteria

- новые правила выдают hotspot-файлы, которые человек признает реально подозрительными;
- false-positive rate приемлем на всех 5 репозиториях;
- location выдается хотя бы для самых полезных правил.

## Этап 2. Structural и graph-aware rules

### Что добавить

- `max_fan_in_per_file`
- `max_fan_out_per_file`
- `module_cycle_member`
- `cross_layer_dependency`
- `hub_module`
- `orphan_module`

### Почему это важно

Это уже слой "снаружи работает, а внутри связность ужасная". Именно эти правила показывают:

- неожиданные центры управления;
- циклы;
- нарушение границ слоев;
- модули, от которых зависит слишком многое;
- модули, которые тянут слишком много зависимостей сами.

### Технический подход

- использовать уже существующий graph/storage слой;
- добавить конфиг layer-политики:
  - expected zones
  - allowed directions
  - forbidden edges
- сначала на уровне файлов/директорий, потом при необходимости на уровне символов.

### Репозитории проверки

- `platforma_maga` как mixed repo с высокой связностью;
- `QubeForge` как более компактный UI-проект;
- `QubeForgeRust` как Rust workspace.

### Acceptance criteria

- на каждом из этих репозиториев система находит хотя бы 3-5 реально полезных structural hotspots;
- человек может глазами подтвердить, что это не шум, а реальные проблемные места;
- cross-layer rules не ломают mixed/profile-aware indexing.

## Этап 3. Policy и noise-control

### Что добавить

- path-scoped thresholds:
  - `src/`
  - `tests/`
  - `scripts/`
  - `config/`
  - `migrations/`
- suppressions/allowlist с причиной;
- per-rule severity;
- per-rule category:
  - `style`
  - `maintainability`
  - `risk`
  - `performance`
  - `architecture`

### Почему это важно

Без этого quality слой быстро превращается в шумогенератор. Разные зоны проекта нельзя мерить одной линейкой.

### Acceptance criteria

- policy можно выразить без правки Rust-кода;
- suppression не скрывает rule полностью, а остается аудитируемой;
- один и тот же rule может иметь разные thresholds по path scope.

## Этап 4. Hotspot scoring и агрегаты

### Что добавить

- `risk_score` на файл;
- `hotspot_score` на директорию;
- агрегаты по модулям;
- "новые нарушения" и "ухудшение после изменений".

### Формула первого приближения

`risk_score = weighted(violation_count, severity, fan_in, fan_out, size, nesting, function_length)`

### Почему это важно

Пользователю и агенту почти всегда нужен не полный лист проблем, а короткий список мест, куда идти в первую очередь.

### Acceptance criteria

- top-10 hotspots по score выглядят правдоподобно на `platforma_maga` и `QubeForge`;
- score можно объяснить;
- score стабилен между refresh, если проект не менялся.

## Этап 5. Репозиторный прогон и hardening

### Задачи

- Прогнать всю матрицу репозиториев.
- Сравнить:
  - latency
  - memory
  - noise
  - ценность выдачи
- Доработать деградации по битым данным.
- Зафиксировать contract tests под каждый крупный класс правил.

### Финальные acceptance criteria

- 5 репозиториев проходят без падений;
- полезность rules подтверждается хотя бы в ручной review-сессии;
- output стабилен и пригоден для UI.

---

# Roadmap 3. Отдельная индексация документации репозитория

## Цель

Сделать отдельный docs-surface, который:

- индексирует только специальную docs-папку репозитория;
- не смешивает документацию с основным code-surface;
- поддерживает подпапки без дополнительных настроек;
- молча пропускает репозиторий, если docs-папки нет;
- позже может быть использован и агентом, и human UI.

## Нецели

- не индексировать весь `*.md` / `*.txt` / `*.rst` по всему репозиторию по умолчанию;
- не использовать `docs-heavy` как основной режим для этой задачи;
- не смешивать docs retrieval с code retrieval в одном топе результатов без явного запроса;
- не превращать docs indexing в обязательное условие работы всего MCP по репозиторию.

## Базовая идея

Нужен отдельный режим: "в репозитории есть специальная папка с человеческой документацией". Если папка есть, мы индексируем только ее. Если папки нет, ничего не ломаем и просто считаем, что docs-surface для этого репозитория отсутствует.

Рекомендуемая папка по умолчанию:

- `.rmu-docs/`

Рекомендуемые подпапки внутри:

- `.rmu-docs/architecture/`
- `.rmu-docs/domain/`
- `.rmu-docs/runbooks/`
- `.rmu-docs/glossary/`
- `.rmu-docs/decisions/`
- `.rmu-docs/integrations/`

## Этап 0. Контракт и поведение по умолчанию

### Что добавить

- отдельный конфиг docs-root:
  - default: `.rmu-docs/`
  - override через project config при необходимости;
- режим `skip-if-missing`;
- поддержка nested docs-подпапок;
- whitelist форматов:
  - `*.md`
  - `*.mdx`
  - `*.txt`
  - `*.rst`
  - опционально `*.adoc`
- отдельный docs status:
  - `missing`
  - `ready`
  - `stale`
  - `degraded`

### Почему это важно

Именно этот контракт не даст системе случайно начать индексировать весь текстовый шум репозитория. Пользователь сам кладет документацию туда, где она должна жить, а система понимает: это отдельный слой знания, а не часть code graph.

### Acceptance criteria

- репозиторий без `.rmu-docs/` не выдает ошибки;
- отсутствие docs-папки не влияет на обычный code indexing;
- наличие nested подпапок не требует ручного перечисления путей.

## Этап 1. Отдельный индексный surface

### Что добавить

- отдельный индексный профиль, например `repo-docs`;
- отдельный retrieval surface `docs`, отделенный от `code`;
- отдельный namespace в storage:
  - минимально: общий storage с обязательным `surface = docs`
  - предпочтительно: отдельные docs-таблицы или отдельный docs-индексный слой;
- отдельные chunking rules для документации:
  - крупнее chunk size
  - мягче overlap
  - явное сохранение заголовков и секций

### Почему это важно

Тут главный смысл не в расширении индекса, а в разделении смыслов. Код должен отвечать на вопросы про реализацию. Docs-surface должен отвечать на вопросы про архитектуру, домен, решения, runbooks и термины. Смешивание этих двух плоскостей снова приведет к шуму.

### Acceptance criteria

- docs-поиск не тянет code-chunks по умолчанию;
- code-поиск не тянет docs-chunks по умолчанию;
- удаление или переименование docs-файлов корректно обновляет docs surface.

## Этап 2. MCP-инструменты для docs-surface

### Что добавить

- `docs_status`
- `docs_index`
- `docs_search`
- `docs_context_pack`
- `docs_brief`

### Минимальные требования к ним

- все работают только по специальной docs-папке;
- отсутствие docs-папки возвращает пустой/neutral результат, а не ошибку;
- `docs_context_pack` собирает только документационные chunks;
- `docs_brief` умеет показывать:
  - сколько docs-файлов есть
  - какие секции/подпапки покрыты
  - какие документы самые крупные
  - какие документы давно не обновлялись

### Acceptance criteria

- можно отдельно спросить MCP про документацию репозитория, не смешивая это с кодом;
- можно построить поверх этого отдельный human-facing docs panel;
- агент получает документационный контекст только тогда, когда он реально нужен.

## Этап 3. Валидация и hardening

### Сценарии проверки

- репозиторий без `.rmu-docs/`;
- репозиторий с плоской `.rmu-docs/`;
- репозиторий с вложенной `.rmu-docs/`;
- rename/move/delete документов;
- неподдерживаемые расширения внутри `.rmu-docs/`;
- частично битый docs-index;
- большой документ с несколькими крупными секциями.

### Репозитории проверки

- реальные репозитории:
  - `platforma_maga`
  - `ficbook-enhancer`
  - `QubeForge`
  - `QubeForgeRust`
  - `kiron_client`
- synthetic fixtures для детерминированных edge-cases.

### Acceptance criteria

- на всех реальных репозиториях отсутствие docs-папки не считается ошибкой;
- на репозиториях с `.rmu-docs/` surface индексируется предсказуемо;
- contract tests покрывают `missing/flat/nested/stale/degraded`.

## Этап 4. Связь с human UI

### Что добавить позже

- docs panel в визуальном UI;
- карта docs coverage по разделам;
- переход:
  - из graph hotspot в связанные docs
  - из docs-секции в соответствующие модули/директории;
- отдельный docs health summary:
  - есть ли архитектурная документация
  - есть ли runbooks
  - есть ли glossary/decisions
  - где пробелы

### Почему это важно

Если граф показывает, что проект связан ужасно, человеку почти сразу нужен второй вопрос: "а это вообще где-то описано?" Отдельный docs-surface должен позволять быстро увидеть не только кодовую реальность, но и документированность этой реальности.

### Acceptance criteria

- UI умеет показать, какие зоны проекта покрыты документацией, а какие нет;
- docs-связи не засоряют основной dependency graph;
- docs-panel реально помогает расследованию, а не дублирует файловый браузер.

---

# Roadmap 2. Визуальный UI для человека

## Цель

Сделать отдельный не-консольный UI-слой, который позволяет человеку:

- понять реальную структуру репозитория;
- увидеть, где архитектура не соответствует ожиданиям;
- увидеть, насколько все плохо и где именно;
- переходить от summary к graph и от graph к конкретной таблице проблем.

## Нецели

- не превращать это в "красивую игрушку";
- не строить сначала symbol-level космос;
- не рисовать все связи сразу без фильтрации;
- не делать Mermaid как основной UX.

## Рекомендуемая архитектура

### Backend

MCP остается backend/data-provider.

### UI

Отдельный web UI.

### Рекомендуемый стек

- `React`
- `Cytoscape.js` или `React Flow`
- `TanStack Table`
- `ECharts`

### Почему не консоль

- нужна интерактивность;
- нужны фильтры;
- нужен hover;
- нужен side panel;
- нужен переход от узла к таблице и обратно.

## Этап 0. Визуальный контракт данных

### Новые MCP surfaces

- `visual_dashboard`
- `visual_table`
- `visual_graph`
- `visual_export`

### Минимальный контракт

#### `visual_dashboard`

- `repo_summary`
- `quality_summary`
- `top_hotspots`
- `top_rules`
- `top_metrics`
- `graph_preview`

#### `visual_table`

- `columns`
- `rows`
- `summary`
- `sort`
- `filters`

#### `visual_graph`

- `nodes`
- `edges`
- `clusters`
- `legend`
- `summary`

### Acceptance criteria

- UI можно строить без парсинга произвольного JSON;
- одни и те же payload пригодны и для web UI, и для HTML export.

## Этап 1. Первый рабочий экран: Quality + Hotspots Dashboard

### Что на экране

- сверху summary cards;
- слева фильтры;
- справа short hotspot panel;
- снизу большая таблица нарушений и проблемных мест.

### Первые таблицы

- `violations`
- `hotspots`
- `oversized_files`
- `architectural_hubs`

### Первые графики

- top rules
- top metrics
- distribution по severity/category

### Репозитории проверки

- сначала `QubeForge`
- затем `platforma_maga`

### Acceptance criteria

- по `QubeForge` UX остается быстрым и понятным;
- по `platforma_maga` экран не разваливается от объема данных;
- можно за 2-3 минуты понять, какие файлы самые проблемные.

## Этап 2. Графы структуры проекта

### Нужные режимы графа

- `directory_graph`
- `file_dependency_graph`
- `hub_graph`
- `quality_overlay_graph`

### Что обязательно уметь

- zoom/pan
- collapse/expand cluster
- hover summary
- click node -> detail panel
- click node -> related table rows
- фильтры:
  - language
  - severity
  - rule
  - path prefix
  - edge type

### Главная идея

Граф должен показывать не "красивую сетку", а скрытую архитектурную правду:

- где слишком много зависимостей;
- где центральные узлы;
- где аномальные связи;
- где quality-проблемы совпадают с graph-проблемами.

### Acceptance criteria

- `QubeForge`: удобно читать структуру;
- `platforma_maga`: видно скрытые hubs и перегруженные зоны;
- quality overlay действительно помогает, а не мешает.

## Этап 3. Detail panel и расследование

### При выборе узла показывать

- path
- node type
- fan-in
- fan-out
- violation count
- top rules
- key metrics
- neighbors
- related files
- shortest suspicious paths

### Связанные действия

- открыть таблицу нарушений по узлу;
- открыть граф соседей;
- переключиться на call-path view;
- открыть export карточки узла.

### Acceptance criteria

- человек может использовать UI как investigative tool, а не только как dashboard;
- из любой "красной" точки можно быстро понять причину.

## Этап 4. Архитектурные представления

### Что добавить

- layer view
- cycle view
- expected vs actual dependency view
- module health view

### Особо полезно для твоего сценария

Этот этап отвечает на вопрос:

"оно работает, но почему ощущение, что всё держится на честном слове?"

Потому что тут уже видно:

- неправильные слои;
- паразитные связи;
- модули, через которые проходит все;
- места, где архитектура на бумаге и архитектура в коде — две разные вещи.

### Acceptance criteria

- на `platforma_maga` видно хотя бы несколько архитектурных smell-зон;
- на `QubeForge` view не перегружен и остается читаемым.

## Этап 5. Экспорт и human reports

### Что добавить

- HTML export
- PNG/SVG snapshot
- markdown summary
- shareable “investigation report”

### Форматы отчетов

- `Repo health report`
- `Hotspot report`
- `Architecture drift report`
- `Quality delta report`

### Acceptance criteria

- можно сохранить investigation в человекочитаемом виде;
- экспорт повторяем и не зависит от консоли.

## Этап 6. Внешний вид и usability polish

### Что важно

- красиво, но без перегруза;
- функционально важнее декоративности;
- цвет = риск, а не украшение;
- темная и светлая тема;
- плавные, но редкие анимации;
- типографика читаемая, не дефолтная.

### Что нельзя

- force-layout ради красоты;
- 300 узлов на экране без фильтра;
- кислотная палитра без смысла;
- таблица без sticky header, сортировок и фильтров.

---

# Рекомендуемый порядок работ

## Сначала

1. roadmap 1, этапы 0-3
2. roadmap 3, этапы 0-2
3. roadmap 2, этапы 0-1

## Потом

4. roadmap 1, этап 4
5. roadmap 2, этапы 2-3
6. roadmap 3, этап 4

## Потом

7. roadmap 1, этап 5
8. roadmap 3, этап 3
9. roadmap 2, этапы 4-6

---

# Практический итог

Если резать по реальной ценности, то лучший ближайший MVP такой:

## MVP quality

- function length
- nesting depth
- params per function
- fan-in / fan-out
- hotspot score
- path-scoped policy

## MVP UI

- dashboard
- hotspot table
- directory/file graph
- quality overlay
- detail panel

## MVP docs indexing

- специальная папка `.rmu-docs/`
- nested docs-подпапки
- `skip-if-missing`
- отдельный docs search/context pack
- отдельный docs brief/status

Именно эта комбинация даст не "красивую технодемку", а полезный инструмент, который реально помогает понять:

- где проект связан не так, как ожидается;
- где скрытые центры сложности;
- где качество кода и архитектурная проблема совпадают;
- насколько всё плохо и с чего начинать разбор.
