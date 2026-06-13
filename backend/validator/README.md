# Validator Engine

`validator-engine` is the Rust crate used by the backend benchmark flow to grade agent outputs with an LLM-as-a-Judge pipeline plus deterministic grader tools.

The crate is intentionally independent from the backend runtime. The backend passes a `ValidationInput` and an `LlmConfig`, then receives a structured `ValidationOutput` with per-criterion results, evidence, total score, verdict, explanation, and recommended price.

## Current State

Implemented:

- Four v1 skills:
  - `defi_yield_routing`
  - `defi_protocol_risk`
  - `rwa_appraisal`
  - `rwa_compliance`
- Static Rust rubrics in `src/rubric.rs`.
- Structured types in `src/types.rs`.
- LLM provider chain in `src/llm.rs`:
  - Cloudflare
  - OpenAI
  - Claude
  - Ollama
  - deterministic mock mode via `VALIDATOR_MOCK_LLM=1`
- Grader pipeline in `src/grader.rs`:
  - collect tool evidence
  - build judge prompt
  - parse per-criterion JSON
  - enforce consistency invariants
  - calculate total score and recommended price
- Fixtures and golden tests for every supported skill.
- Backend integration via `backend/src/validator/v2_adapter.rs`.

The backend benchmark path now attempts v2 validation for supported skills. Unsupported legacy skills, such as `code_review`, keep using the legacy evaluator as a compatibility fallback. Live task execution remains on the legacy evaluator until there is a fixture/input contract for live tasks.

## Tool Behavior

Tools are still stubs by design for phases 1-3.

For every known tool name, `src/tools.rs` returns:

```json
{
  "tool": "<tool name>",
  "ok": true,
  "details": {
    "stub": true
  }
}
```

Unknown tools return:

```json
{
  "tool": "<tool name>",
  "ok": false,
  "details": {
    "error": "unknown tool"
  }
}
```

This keeps the evidence schema stable while real deterministic checks are implemented in phase 4.

## Public API

```rust
pub async fn evaluate(
    input: ValidationInput,
    config: &LlmConfig,
) -> Result<ValidationOutput, ValidatorError>;
```

Important input fields:

- `skill`: one of the supported `SkillId` variants.
- `task_prompt`: original task given to the worker agent.
- `agent_output`: worker agent response to grade.
- `fixture`: deterministic benchmark data used by tools and included in the judge prompt.
- `processing_time_ms`: used by the pricing formula.

Important output fields:

- `criteria`: per-criterion pass/fail, score, gap, and tool evidence.
- `total`: sum of criterion scores, 0-100.
- `verdict`: `satisfied` only when every criterion passes.
- `recommended_price_motes`: quality and speed adjusted price.

## Development Commands

Run all validator tests without external API keys:

```bash
VALIDATOR_MOCK_LLM=1 cargo test
```

Run from the backend package after integration:

```bash
cd ../
VALIDATOR_MOCK_LLM=1 cargo test
```

## Remaining Work: Phase 4+

### Phase 4: Replace Stub Tools

Implement real deterministic tools one vertical slice at a time.

Recommended order:

1. `defi_yield_routing`
   - `check_allocation_sum`
   - `validate_apy`
   - `check_fees`
   - `validate_il`
2. `defi_protocol_risk`
   - `validate_revert_rate`
   - `check_risk_thresholds`
3. `rwa_appraisal`
   - `validate_outliers`
   - `check_sources`
   - `validate_price_derivation`
4. `rwa_compliance`
   - `classify_news`
   - `validate_collateral_logic`

Per tool checklist:

- Keep the tool as a pure function over `fixture` and `agent_output`.
- Add unit tests for passing, failing, missing-field, and malformed-output cases.
- Replace only that tool's stub branch in dispatch.
- Add or update a golden test that shows the tool evidence affects the final criterion result.
- Preserve the `ToolResult` schema.

When `src/tools.rs` becomes too large, split it mechanically into `src/tools/` by skill while keeping the public `run_tool(name, fixture, agent_output)` API unchanged.

### Phase 5: Live Task Input Contract

Benchmark validation uses fixtures. Live task execution currently does not have a stable fixture contract, so it stays on the legacy evaluator.

Before moving live tasks to v2:

- Define where domain input data comes from:
  - task metadata URI
  - request payload
  - indexed on-chain data
  - backend fixture fallback
- Define validation behavior when required input data is missing.
- Add integration tests for `POST /api/tasks/:id/execute`.

### Phase 6: Legacy Cleanup

After benchmark and live paths both use v2 safely:

- Remove legacy `llm_judge.rs` only after there is no runtime caller.
- Migrate any consumers expecting legacy `RubricScores`.
- Decide whether `code_review` should receive a v2 rubric or remain unsupported.
- Update API documentation for the final `benchmark_runs.rubric_scores` JSON shape.

### Phase 7: Gating / Revision Loop

Current validation is post-hoc scoring. A future revision loop can use the existing per-criterion `gap` output to ask worker agents for corrections before final scoring.

Open decisions:

- Add a `NeedsRevision` verdict or keep revision state outside this crate.
- Limit revision attempts and time budget.
- Store revision history in benchmark/task records.
