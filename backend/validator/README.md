# Validator Engine

Standalone Rust crate (`validator-engine`) that grades worker-agent outputs for the Casper Agent Network benchmark flow using **LLM-as-a-Judge + deterministic tools**.

## Documentation

| Document | Language | What it covers |
|----------|----------|----------------|
| [**implementation.md**](./implementation.md) | EN | **Current implementation** — API, pipeline, rubrics, tools, backend integration, tests |
| [**roadmap.md**](./roadmap.md) | RU | **Roadmap** — phases, target F3 architecture, test matrix |
| [**task.md**](./task.md) | RU | **Product requirements** — DeFi/RWA domains, tasks, dev rules |
| [**methodics.md**](./methodics.md) | RU | **Scoring methodology** — F3 principles, priorities, external sources |
| [**llm-as-judge-pattern.md**](./llm-as-judge-pattern.md) | RU | **Reference** — RubricMiddleware pattern analysis |
| [**README.ru.md**](./README.ru.md) | RU | This page in Russian |

## Quick Start

```bash
cd backend/validator
VALIDATOR_MOCK_LLM=1 cargo test

# Local regression gate (mock LLM, golden + harness)
VALIDATOR_MOCK_LLM=1 ./scripts/regression_gate.sh
```

Public API:

```rust
use validator_engine::{evaluate, LlmConfig, ValidationInput, SkillId};

let output = evaluate(input, &LlmConfig::from_env()).await?;
// output.total (0–100), output.criteria, output.verdict, output.recommended_price_motes
```

Details: [implementation.md §4](./implementation.md#4-public-api).

## Status

**Done:**
- 4 DeFi/RWA skills · rubrics + embedded JSON fixtures (`include_str!`).
- LLM chain (temp=0) + mock mode.
- F3 grader (gates, hard-from-tool, soft enum-labels, threshold/critical).
- Regression harness + golden datasets.
- Generic custom LLM provider support (`VALIDATOR_PROVIDER`, `VALIDATOR_LLM_URL`, etc.).
- Live `/execute` cutover and integration with Rust Axum orchestrator.
- E2E persistence: raw task results stored in the MySQL database tasks table.

**Pending:**
- real tool logic · few-shot prompts · legacy cleanup · gating/revision loop.

Details: [implementation.md](./implementation.md) · [roadmap.md](./roadmap.md)

## Backend Integration

- Crate path: `backend/validator/`
- Adapter: [`backend/src/validator/v2_adapter.rs`](../src/validator/v2_adapter.rs)
- Benchmark: [`backend/src/orchestrator/benchmark.rs`](../src/orchestrator/benchmark.rs)

Unsupported skills (e.g. `code_review`) are **skipped** in benchmark, not scored.
