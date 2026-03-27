# Чего не хватает движку и что реально может помочь

Этот файл про то, какие модули ещё стоит добавить в `RMU`, если развивать именно quality и structural analysis, а не смешивать всё с retrieval и investigation.

## Что уже доведено до ума и не надо снова открывать без новой причины

Отдельно зафиксировано, что к 27 марта 2026 уже закрыт и не относится к списку "чего не хватает":

- `project map` стартовая поверхность;
- различение `CLI brief`, `MCP workspace_brief` и `MCP agent_bootstrap`;
- broad-query diversification для `entrypoints`, `auth/tests`, `api + domain/services`, `mod/runtime`;
- optional `query_bundle.followups` как next-step surface;
- standalone `search` для generic `mod/runtime` и plain `tests` query;
- suppression build/docs/generated/support noise в top shortlist;
- read-only / repair semantics для `workspace_brief` и no-query MCP bootstrap path.

Canonical closed checklist для этого блока лежит в:

- `md/plans/closed/2026-03-27-project-map-checklist.closed.md`

То есть ниже перечисляются не эти уже закрытые retrieval/bootstrap задачи, а именно то, чего ещё не хватает движку сверх уже доведённого `project map` и базового agent-facing retrieval surface.

## Где тут граница с RAG

Полноценный `RAG` не должен становиться core-идеей `RMU`.

Для `RMU` уместно оставлять то, что относится к code retrieval и grounded context:

- ingestion и индексирование кода;
- lexical / semantic retrieval;
- reranking;
- chunk selection;
- `context_pack` и context under budget;
- retrieval explainability;
- provenance и confidence;
- investigation и quality/risk signals как усилители retrieval.

То есть `RMU` должен быть сильным retrieval backend для кода.

А вот это в `RMU` тащить не стоит:

- first-class answer synthesis;
- долговременную память решений и договорённостей;
- canonical knowledge summary;
- temporal memory;
- Obsidian / vault semantics;
- knowledge-first assistant behavior.

Это уже не code intelligence engine, а отдельный memory / knowledge контур.

## Короткий ответ

Да, ты в целом правильно понял.

Большая часть из того, что имеет смысл добавить, это новые quality-правила и новые источники сигналов для quality surface.

Но тут есть важное разделение:

1. есть модули, которые сами дают новые `rules`
2. есть модули, которые собирают новые `metrics` и `facts`
3. есть модули, которые на основе этих фактов считают `risk` и `hotspots`

То есть это не всё подряд просто "ещё правила". Часть вещей - это именно поставщики данных для правил.

## Что уже есть сейчас

Сейчас quality surface уже умеет смотреть на:

- размер файла
- длину строк
- длину функций
- число импортов
- число символов и ссылок
- fan-in и fan-out
- циклы зависимостей
- hub-модули
- cross-layer dependency
- orphan modules

Этого уже хватает, чтобы ловить базовый structural debt. Но есть несколько дыр, из-за которых часть действительно тяжёлых мест ещё не видна.

## Что стоит добавить в первую очередь

## 1. Модуль сложности функций

### Что добавить

- cyclomatic complexity
- cognitive complexity
- branch count
- early return count

### Почему это полезно

Сейчас длинная функция видна, а ветвистая - не всегда.

Бывает короткая функция на 35 строк, но внутри у неё:

- куча `if`
- несколько `match`
- fallback paths
- ранние выходы
- исключения

По длине она может не выглядеть страшно. По факту читать и менять её тяжело.

### К чему относится

Это quality-модуль с новыми метриками и новыми правилами.

## 2. Модуль дублирования

### Что добавить

- прямые дубли блоков
- near-duplicates
- повторяющиеся паттерны между файлами

### Почему это полезно

Повтор логики сильно портит поддержку:

- правка в одном месте забывается в другом
- одинаковые баги живут в копиях
- cleanup становится дорогим

Это особенно полезно для:

- endpoint-слоя
- service-слоя
- schema/helpers
- UI-компонентов

### К чему относится

Это quality-модуль с отдельным анализатором и правилами поверх него.

## 3. Модуль git-risk

### Что добавить

- file churn
- число авторов файла
- частота изменений
- coupling по совместным коммитам

### Почему это полезно

Если файл:

- уже сложный
- часто меняется
- трогается разными людьми

то это реальная зона риска. Такие места ломаются не потому, что у них плохая длина строки, а потому что они постоянно под движением.

### К чему относится

Это не чистое rule-layer решение. Это модуль сигналов, который потом влияет на risk score и hotspots.

## 4. Модуль test-risk

### Что добавить

- рядом нет тестов
- public surface без покрытия
- сложный файл без integration tests
- hotspot без тестового следа

### Почему это полезно

Сам по себе сложный код - это неприятно. Сложный код без тестов - это уже реальный риск.

Этот модуль даёт хороший практический сигнал для приоритизации.

### К чему относится

Это модуль сигналов и правил поверх coverage/test evidence.

## 5. Модуль layering и boundary violations

### Что добавить

- явные правила слоёв
- запрещённые зависимости между директориями
- контракты вида `ui -> domain`, `domain -> infra` и так далее

### Почему это полезно

`cross_layer_dependency` уже есть, но его можно сильно усилить.

Сейчас видно сам факт плохой связи. Дальше полезно иметь отдельный модуль, который понимает слой как правило, а не просто как симптом.

### К чему относится

Это quality-модуль с архитектурными правилами.

## Что стоит добавить вторым эшелоном

## 6. Модуль API surface

### Что добавить

- слишком широкие public exports
- unstable public hubs
- файлы, которые тащат наружу слишком много контрактов

### Почему это полезно

Это помогает понять blast radius от правок в публичных модулях.

### К чему относится

Это quality-модуль с новыми правилами и метриками по exports/public surface.

## 7. Модуль dead code

### Что добавить

- почти неиспользуемые экспорты
- сироты
- слабосвязанные utility-файлы

### Почему это полезно

Помогает чистить кодовую базу и уменьшать шум.

### Риск

Тут больше ложных срабатываний. Особенно на reflection, dynamic imports и внешних entrypoints.

### К чему относится

Это quality-модуль, но с осторожным rollout.

## 8. Модуль security-smells

### Что добавить

- shell exec
- path traversal smells
- raw SQL
- опасные deserialization patterns
- secrets-like literals

### Почему это полезно

Это уже не про обычный cleanup, а про безопасность. Но как отдельная поверхность это может быть очень полезно.

### К чему относится

Это лучше держать отдельно от обычного quality, чтобы не смешивать поддержку кода с security analysis.

## 9. Модуль sensitive-data detection

### Что добавить

- зашитые API keys
- токены
- секреты в env-like строках
- приватные ключи
- webhook secrets
- database credentials
- access tokens в тестах, fixtures и примерах

### Почему это полезно

Это уже не просто smell. Это прямой риск утечки.

Если в коде лежит зашитый токен, агент не должен молча пройти мимо. Такие места надо сразу поднимать как проблемные.

Это особенно важно именно для агентского сценария. Агент много читает код и конфиги, и если у него под рукой есть такой модуль, он может не только разбирать репозиторий, но и сразу подсвечивать реальные секреты, которые нельзя тащить в git.

### Как это должно работать

Этот модуль лучше делать отдельным.

Он должен:

- сканировать код, конфиги, примеры, fixtures и env-файлы
- различать реальный секрет и явный placeholder
- отдавать список подозрительных мест с уровнем уверенности
- подсвечивать такие находки через `MCP`
- уметь давать агенту явный warning в ответе или в tool payload

То есть при использовании `MCP` агент должен сразу видеть:

- путь к файлу
- тип подозрения
- короткий фрагмент
- почему это похоже на секрет
- насколько это уверенный матч

### К чему относится

Это отдельный security-модуль со своими detectors, своими правилами и своим noise-control.

Его не стоит смешивать с обычным structural quality.

## Что из этого даст самую быструю пользу

Если идти по отдаче, я бы ставил так:

1. `complexity`
2. `duplication`
3. `git_risk`
4. `test_risk`
5. `layering`

Если смотреть именно на безопасность и защиту репозитория, то `sensitive-data detection` я бы вообще держал отдельным приоритетом, не смешивая его с этим списком.

Именно эта пятёрка даст самый сильный прирост к текущему quality surface.

## Как это лучше раскладывать по модулям

Я бы не тащил всё в один большой `quality.rs`.

Нормальная схема такая:

- `quality/complexity`
- `quality/duplication`
- `quality/git_risk`
- `quality/test_risk`
- `quality/layering`
- `quality/api_surface`
- `quality/dead_code`
- `quality/security_smells`
- `security/sensitive_data`

Тогда их можно:

- включать поэтапно
- тестировать по отдельности
- держать разные noise-профили
- не смешивать базовые правила с дорогими анализаторами

## Что здесь является "правилами", а что нет

### Это в основном новые правила

- cyclomatic complexity threshold
- cognitive complexity threshold
- duplication threshold
- boundary violation rules
- export/public surface rules
- secret-detection rules

### Это скорее новые источники сигналов

- git churn
- ownership spread
- test presence
- coverage evidence
- secret detectors и confidence signals

### Это уже агрегаторы

- risk score
- hotspot score
- directory hotspot ranking

То есть правильная модель такая:

сначала модуль собирает факты -> потом поверх них срабатывают rules -> потом scoring собирает общую картину.

## Чего ещё не хватает именно для агентской практической работы

Выше - в основном про `quality surface`.

Но если смотреть именно глазами coding agent, который должен быстро понять репозиторий, найти точку входа, оценить риск и не утонуть в шуме, то не хватает ещё нескольких универсальных слоёв.

Это тоже должно быть language-agnostic:

- для `Rust`
- для `Java`
- для `TypeScript`
- для `Python`
- для смешанных репозиториев

То есть это не про один стек, а про форму работы с кодовой базой.

## 10. Provenance и basis surface

### Что добавить

- явную пометку, на чём основан ответ:
  - `indexed`
  - `live file`
  - `graph-derived`
  - `heuristic`
  - `fallback`
- явную свежесть источника
- понятный confidence не только у retrieval, но и у итогового вывода

### Почему это полезно

Агенту мало просто "получить ответ".

Ему нужно понимать:

- это точный факт из живого файла или старый индекс
- это сильное evidence или эвристика
- можно ли на это опираться для правки

Без этого даже хороший ответ трудно использовать как основу для уверенного engineering decision.

### К чему относится

Это не quality-rule.

Это meta-surface для explainability и trust.

## 11. Fast-path для больших репозиториев

### Что добавить

- быстрый shortlist-режим для широких вопросов
- двухфазный режим:
  - сначала cheap answer
  - потом deepen-on-demand
- timeout-aware degradation вместо полного провала

### Почему это полезно

На маленьком проекте можно позволить себе более дорогой retrieval.

На большом mixed-language репозитории агентские вопросы часто широкие:

- где entrypoints
- как течёт data flow
- где auth boundary
- какие тесты рядом

Если такие запросы просто упираются в timeout, инструмент теряет ценность именно в тот момент, когда он больше всего нужен.

### К чему относится

Это retrieval/orchestration layer.

## 12. Intent-aware режимы для агентских вопросов

### Что добавить

- отдельные режимы не только по форме поиска, но и по задаче:
  - `entrypoint-map`
  - `test-map`
  - `review-prep`
  - `api-contract-map`
  - `runtime-surface`
  - `refactor-surface`

### Почему это полезно

Сейчас один и тот же retrieval pipeline может одинаково обрабатывать очень разные вопросы.

Но агентские вопросы реально разные по цели:

- найти точку входа
- понять blast radius
- собрать соседние тесты
- подготовить ревью
- найти безопасный seam для split

У таких вопросов должен быть разный ranking и разный budget allocation.

### К чему относится

Это orchestration layer поверх retrieval и investigation.

## 13. Framework-aware normalization

### Что добавить

- нормализацию route/path surface под популярные framework patterns
- нормализацию dynamic path conventions
- нормализацию background/runtime actors:
  - workers
  - schedulers
  - bots
  - webhooks
  - message handlers

### Почему это полезно

Файл может быть найден правильно, но представлен неудобно для человека и агента.

Если route path, runtime actor или framework boundary отданы в сыром виде, понимание проседает.

Инструмент должен уметь показывать не только путь к файлу, но и его роль в системе в более нормализованном виде.

### К чему относится

Это navigation/explainability layer.

## 14. Cross-language contract tracing

### Что добавить

- трассировку контрактов между слоями и языками
- путь вида:
  - schema/model
  - backend endpoint
  - generated client
  - frontend consumer
  - tests
- пометку разрыва, если цепочка не закрывается evidence

### Почему это полезно

Много реальных проектов давно не mono-language.

Именно на стыках чаще всего и происходят agent mistakes:

- backend поменяли
- frontend consumer не нашли
- generated слой приняли за source of truth
- тестовый surface собрали неполно

Если инструмент умеет first-class показывать такие цепочки, это резко поднимает качество impact analysis.

### К чему относится

Это investigation + contract surface.

## 15. Generated lineage и source-of-truth surfacing

### Что добавить

- пометку, что файл generated
- указание, откуда он произошёл
- при возможности - переход к source of truth
- понижение приоритета generated-артефактов в review и refactor ranking

### Почему это полезно

Generated code часто попадает в shortlist, хотя менять надо не его, а генератор, schema или source module.

Без этого агент легко приходит не в то место и начинает чинить следствие, а не причину.

### К чему относится

Это retrieval ranking + explainability.

## 16. Role-aware hotspot ranking

### Что добавить

- различение ролей файлов:
  - shared utility
  - ui primitive
  - generated file
  - migration
  - adapter
  - public contract
  - feature module
- отдельный ranking policy по ролям

### Почему это полезно

Самые горячие файлы по fan-in не всегда самые полезные для action.

Иногда это просто:

- `utils`
- UI primitives
- shared constants
- framework glue

Без role-awareness hotspot ranking начинает переоценивать центральные, но не всегда practically actionable узлы.

### К чему относится

Это scoring/ranking layer.

## 17. Actionability layer

### Что добавить

- не только "где больно", но и:
  - где safest split seam
  - какие соседние файлы затронет change
  - какие тесты релевантны
  - какие проверки запускать
  - где вероятный rollback-sensitive участок
- короткий next-step plan без генерации кода

### Почему это полезно

Агенту мало знать hotspot.

Ему нужен operationally useful ответ:

- куда идти
- что не забыть
- чем подтвердить
- какой риск у правки

Именно этот слой превращает code intelligence в практический execution assistant.

### К чему относится

Это orchestrator-facing surface.

## 18. Noise memory и feedback loop

### Что добавить

- repo-local knowledge о том, что считается шумом
- suppressions с объяснением
- память о принятых исключениях
- память о том, какие сигналы уже признавались полезными или бесполезными

### Почему это полезно

Универсальные эвристики без feedback loop почти всегда со временем начинают шуметь.

Один и тот же сигнал в одном репозитории полезен, а в другом - почти бесполезен.

Если инструмент умеет запоминать локальные исключения и локальные паттерны шума, он становится заметно практичнее без потери универсальности.

### К чему относится

Это meta-layer для ranking, suppressions и future retrieval quality.

## Что я бы не делал сейчас

Не стоит сразу тащить:

- naming-style эвристику
- слабые readability rules
- слишком много cosmetic checks

Они шумят, но мало помогают принять инженерное решение.

Сначала нужны модули, которые реально отвечают на вопрос:

"Где код больной и почему его тяжело менять?"

## Вывод

Для движка сейчас важнее всего не новый косметический lint, а более сильные structural сигналы.

Если упрощать до одной строки:

движку больше всего не хватает `complexity`, `duplication`, `git_risk`, `test_risk` и нормального `layering` как отдельных модулей.

А если смотреть именно на agent workflow, то поверх этого ещё критично нужны:

- `provenance`
- `fast-path retrieval`
- `intent-aware modes`
- `cross-language contract tracing`
- `generated lineage`
- `role-aware ranking`
- `actionability`
- `noise memory`
