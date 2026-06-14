# Validator Engine

Отдельный Rust-crate (`validator-engine`) для оценки ответов worker-агентов в Casper Agent Network. Использует **LLM-as-a-Judge + детерминированные tools**.

## Документация

| Документ | Язык | Содержание |
|----------|------|------------|
| [**implementation.md**](./implementation.md) | EN | **Текущая имплементация** — API, пайплайн, рубрики, tools, интеграция с backend, тесты |
| [**roadmap.md**](./roadmap.md) | RU | **План разработки** — фазы, целевая архитектура F3, матрица тестов |
| [**task.md**](./task.md) | RU | **Требования** — домены DeFi/RWA, задачи, правила разработки |
| [**methodics.md**](./methodics.md) | RU | **Методика оценки** — принципы F3, приоритеты, внешние источники |
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

**Готово:** 4 skill DeFi/RWA · рубрики + fixtures · LLM-цепочка (temp=0) + mock · F3 grader (gates, hard-from-tool, soft enum-labels, threshold/critical) · regression harness + golden · benchmark только v2.

**Впереди:** реальная логика tools · few-shot промпты · cutover live `/execute` · E2E с БД · чистка legacy · gating/revision loop.

Подробнее: [implementation.md](./implementation.md) · [roadmap.md](./roadmap.md)

## Интеграция с backend

- Crate: `backend/validator/`
- Адаптер: [`backend/src/validator/v2_adapter.rs`](../src/validator/v2_adapter.rs)
- Benchmark: [`backend/src/orchestrator/benchmark.rs`](../src/orchestrator/benchmark.rs)

Неподдержанные skill (например `code_review`) в benchmark **пропускаются**, не оцениваются.
