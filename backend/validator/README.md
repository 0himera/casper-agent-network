# Validator Engine

Standalone Rust crate (`validator-engine`) that grades worker-agent outputs for the Casper Agent Network benchmark flow using **LLM-as-a-Judge + deterministic tools**.

## Documentation

| Document | Language | What it covers |
|----------|----------|----------------|
| [**implementation.md**](./implementation.md) | EN | **Current implementation** — API, pipeline, rubrics, tools, backend integration, tests |
| [**roadmap.md**](./roadmap.md) | RU | **Roadmap** — delivery plan, test matrix |
| [**task.md**](./task.md) | RU | **Product requirements** — DeFi/RWA domains, tasks, dev rules |
| [**methodics.md**](./methodics.md) | RU | **Scoring methodology** — hybrid scoring principles, priorities, external sources |
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

**Done:** 4 DeFi/RWA skills · rubrics + fixtures · hybrid grader (input gates, deterministic hard checks, LLM labels for soft criteria, pass threshold + critical flags) · 11 real deterministic tools · LLM chain (temp=0) + mock · custom provider support (`VALIDATOR_PROVIDER`, `VALIDATOR_LLM_URL`) · few-shot judge prompts · per-skill judge routing + optional self-consistency · regression harness + golden · benchmark v2-only · live fixture contract (inline JSON + schema validation) · E2E assign→execute with injected fixture · task results persisted in DB.

**Pending:** live `/execute` cutover to v2 evaluator · fixture persistence for production tasks · legacy cleanup · gating/revision loop.

Details: [implementation.md](./implementation.md) · [roadmap.md](./roadmap.md)

## Changelog

### Since hybrid scoring baseline

- **Deterministic tools:** 11 real check functions across 4 skills (replacing stubs); hard criteria scored from tool evidence only.
- **LLM module:** provider chain split into routing layer; per-skill judge model overrides; optional local→API cascade and timeout fallback.
- **Few-shot prompts:** exemplars in `model_configs.yaml` for soft-criteria evaluation; calibration harness and A/B script.
- **Self-consistency:** optional majority vote when a soft label is ambiguous (configurable per skill).
- **Fixture contract:** JSON Schema per skill; adapter accepts inline fixture JSON (not only on-disk files); worker prompt includes a `<fixture>` block when provided.
- **Backend:** explicit `skill_id` on tasks; orchestrator task pipeline and worker prompt assembly; E2E test with injected fixture and mock LLM.

## Backend Integration

- Crate path: `backend/validator/`
- Adapter: [`backend/src/validator/v2_adapter.rs`](../src/validator/v2_adapter.rs)
- Benchmark: [`backend/src/orchestrator/benchmark.rs`](../src/orchestrator/benchmark.rs)

Unsupported skills (e.g. `code_review`) are **skipped** in benchmark, not scored.
