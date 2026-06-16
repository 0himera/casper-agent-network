# Validator Engine

Отдельный Rust-crate (`validator-engine`) для оценки ответов worker-агентов в Casper Agent Network. Использует **LLM-as-a-Judge + детерминированные tools**.

## Документация

| Документ | Язык | Содержание |
|----------|------|------------|
| [**implementation.md**](./implementation.md) | EN | **Текущая имплементация** — API, пайплайн, рубрики, tools, интеграция с backend, тесты |
| [**roadmap.md**](./roadmap.md) | RU | **План разработки** — этапы, бэклог fixture source, матрица тестов |
| [**judge-scenarios.ru.md**](./judge-scenarios.ru.md) | RU | **Сценарии** — когда вызывается judge (benchmark vs live) |
| [**task.md**](./task.md) | RU | **Требования** — домены DeFi/RWA, задачи, правила разработки |
| [**methodics.md**](./methodics.md) | RU | **Методика оценки** — принципы гибридного скоринга, приоритеты, внешние источники |
| [**llm-as-judge-pattern.md**](./llm-as-judge-pattern.md) | RU | **Справочник** — разбор паттерна RubricMiddleware |
| [**README.md**](./README.md) | EN | Эта страница на английском |

## Быстрый старт

```bash
cd backend/validator
VALIDATOR_MOCK_LLM=1 cargo test

# Локальный regression gate (mock LLM)
VALIDATOR_MOCK_LLM=1 ./scripts/regression_gate.sh
```

Публичный API:

```rust
use validator_engine::{evaluate, LlmConfig, ValidationInput, SkillId};

let output = evaluate(input, &LlmConfig::from_env()).await?;
// output.total (0–100), output.criteria, output.verdict, output.recommended_price_motes
```

Подробнее: [implementation.md §4](./implementation.md#4-public-api).

## Статус

**Готово:** 4 skill DeFi/RWA · рубрики + fixtures · гибридный grader (gates, hard из tools, soft через LLM-labels, порог + critical) · 11 реальных deterministic tools · LLM-цепочка (temp=0) + mock · few-shot промпты для judge · per-skill routing + опциональный self-consistency · regression harness + golden · benchmark только v2 · live fixture contract (inline JSON + schema) · E2E assign→execute с injected fixture.

**Впереди:** cutover live `/execute` на v2 · persistence fixture для production-задач · чистка legacy · gating/revision loop.

Подробнее: [implementation.md](./implementation.md) · [roadmap.md](./roadmap.md)

## Changelog

### После baseline гибридного скоринга

- **Deterministic tools:** 11 реальных check-функций по 4 skill (вместо stubs); hard-критерии только из tool evidence.
- **LLM-модуль:** routing-слой, per-skill overrides для judge-модели, опциональный cascade local→API и timeout fallback.
- **Few-shot промпты:** exemplars в `model_configs.yaml` для soft-критериев; calibration harness и A/B-скрипт.
- **Self-consistency:** majority vote при неоднозначном soft-label (настраивается per skill).
- **Fixture contract:** JSON Schema per skill; adapter принимает inline JSON; worker prompt включает блок `<fixture>` при наличии данных.
- **Backend:** явный `skill_id` у tasks; task pipeline и сборка worker prompt; E2E-тест с injected fixture и mock LLM.

## Интеграция с backend

- Crate: `backend/validator/`
- Адаптер: [`backend/src/validator/v2_adapter.rs`](../src/validator/v2_adapter.rs)
- Benchmark: [`backend/src/orchestrator/benchmark.rs`](../src/orchestrator/benchmark.rs)

Неподдержанные skill (например `code_review`) в benchmark **пропускаются**, не оцениваются.
