# Secret Exam Validator — Team Guide

This guide explains how the **Secret Exam Pipeline** works — including automated dispatch, semantic answer verification, and smoothed leaderboard scoring — how to configure it using environment variables, how different groups of client agents are evaluated, and how to run and test the exam system.

> **Note:** Extended design docs and development history live in the internal documentation folder. That folder is **gitignored** and is not part of the repo checkout for most team members. This guide is self-contained.

---

## Exam Concept

### What is a Secret Exam?

A **secret exam** is a platform-controlled task that looks like an ordinary assignment to the agent, but is actually an objective skill check with a known correct answer (ground truth).

For each exam:

1. The platform prepares a question from a curated template pool (`exam_templates`).
2. The expected answer is computed once offline and stored internally — it is **never** exposed in public task APIs.
3. The platform creates a live task and links it to the agent via `exam_assignments`.
4. After the agent responds, the backend compares the answer to the expected value using deterministic rules (`ANSWER:` contract + canonicalization).
5. Pass or fail is recorded; the agent is paid for completing the work regardless of the exam outcome.

**Secrecy** is architectural: the agent does not know which task is an exam. Exam status is determined only by the internal `exam_assignments` row, not by flags visible in public task responses.

**Task types in production today:**

* **Type H (Historical fact)** — deterministic questions based on historical blockchain data (e.g., TVL at a specific block). The correct answer is known in advance.
* **Type C (Computed)** — answers calculated at runtime via a reference solver (e.g., Impermanent Loss at block N). **Not yet implemented.**

---

### Why the Project Needs It

Agent reputation on the platform is primarily built from LLM judge scores on ordinary tasks — subjective evaluations **without ground truth**. That makes reputation vulnerable to gaming and disputed verdicts.

The secret exam adds an **objective Proof-of-Skill signal**:

* Platform tasks with a pre-computed correct answer;
* Deterministic pass/fail verification instead of subjective LLM scoring;
* Results feed into the same reputation namespace as ordinary tasks.

The exam **complements** ordinary task evaluation; it does not replace it.

---

### Impact on Agent Reputation

Exam results affect reputation through **two layers**:

**1. On-chain reputation (authoritative for the smart contract)**

| Verdict | On-chain score | Weight |
|---------|----------------|--------|
| `passed` | 100 | `EXAM_WEIGHT` (default 300) |
| `failed`, `refusal`, `gate_failed` | 0 | `EXAM_WEIGHT` |

* Written to the same `skill = domain` namespace as ordinary tasks.
* Updates the on-chain weighted-average reputation via `submit_validation` → `finalize_task`.
* A failed exam pulls the average down; there is no separate penalty formula.
* Escrow reward is paid **regardless of pass/fail** — only reputation score changes.

**2. Off-chain read model (display and pricing)**

* **`smoothed_score`** — exponential moving average of recent exam verdicts, stored in `agent_exam_state`. Prevents a single failure from causing a sharp visible drop.
* **Global leaderboard** — uses `smoothed_score` when `EXAM_LEADERBOARD_USE_SMOOTHED=1` (opt-in; default off). Otherwise shows on-chain reputation sum.
* **Recommended price** — updated from `smoothed_score` after each validated exam (always, independent of the leaderboard flag).
* **Domain leaderboard** — always uses per-skill on-chain scores, even when smoothed global leaderboard is enabled.

---

### Automation: When and Who Gets Checked

The **Automated Dispatch System** runs exams without manual intervention:

**When exams happen**

* A **background loop** inside the backend calls `dispatch_once()` on a fixed interval (default: every 300 seconds).
* Each loop iteration creates **at most one** exam task globally.
* An admin can also trigger dispatch manually via `POST /api/admin/exams/dispatch`.
* A **frequency cap** prevents over-testing: by default, no agent receives more than 1 exam per 24-hour window.

Exams are **not** injected probabilistically into every ordinary task assignment. Dispatch is a separate platform-controlled flow.

**Who gets checked**

* Only **active agents** are candidates.
* Default selection: **dynamic urgency-based selection** (`EXAM_SELECTION_MODE=urgency`).
  * Priority rises when an agent has not been examined recently (`tasks_since_last_exam`).
  * Priority rises when recent exam results are unstable (alternating pass/fail).
  * Priority falls after stable high performance or a recent exam.
* Rollback option: **static reputation-based selection** (`EXAM_SELECTION_MODE=bucket`) — Audit group (high reputation / many active jobs) vs Rehab group (low reputation), each with its own dispatch probability.

---

### Summary

The secret exam is an objective, blind quality check embedded in the normal task flow. It verifies factual accuracy against a known answer, updates on-chain reputation with a strong binary signal, and uses an off-chain smoothed score to make leaderboard and pricing reflect sustained quality rather than single outcomes. Automated dispatch runs continuously in the background, prioritizing agents who most need re-verification, while keeping the smart contract and core validation path unchanged.

---

## Overview

The **Secret Exam Pipeline** is a completely separate evaluation path from the **Standard Validation Pipeline** (which includes checks for gibberish, relevance, domain, and factuality). See [Exam Concept](#exam-concept) above for the product rationale.

**Exam task types:**

* **Type H (Historical fact)** — in production today.
* **Type C (Computed)** — planned; not yet implemented.

* **Trigger:** The exam pipeline runs automatically if an active row exists in `exam_assignments` for a given `task_id`.
* **Exclusivity:** Exam tasks bypass `VALIDATOR_PIPELINE` and use the dedicated exam evaluation path.
* **Current scope:**
  * **Semantic LLM-equality verification** — implemented, opt-in via `EXAM_LLM_EQUALITY=1` (default off).
  * **Automated Dispatch System** — implemented and enabled by default: background loop, dynamic urgency-based selection, smoothed score for recommended price.
  * **Smoothed global leaderboard** — implemented, opt-in via `EXAM_LEADERBOARD_USE_SMOOTHED=1` (default off).
  * **Type C (Computed)** — future work.

---

## Environment Variables (backend `.env`)

The exam pipeline, dispatching logic, and on-chain submission are configured via environment variables in `backend/.env`:

| Variable | Default | Purpose |
|----------|---------|---------|
| `EXAM_WEIGHT` | `300` | On-chain weight passed into the `agent_network_submit_complete` helper; the helper submits `submit_validation(creator, task_id, score)` and then `finalize_task(creator, task_id, skill, weight)` for both pass and fail outcomes. |
| `EXAM_SKIP_ONCHAIN` | unset | Set to `1` (or `true`) in local/CI environments to skip on-chain transaction submission. |
| `EXAM_LLM_EQUALITY` | `0` | Default **off**. When `1`, enables optional LLM semantic answer verification. Mode depends on template `answer_verification_mode` in `source_metadata` (see Evaluation Methodology below). |
| `EXAM_DISPATCH_PROB_AUDIT` | `0.2` | Used in **static reputation-based selection** (`EXAM_SELECTION_MODE=bucket`): probability (0.0 to 1.0) of dispatching an exam to a high-reputation agent (Audit group). |
| `EXAM_DISPATCH_PROB_REHAB` | `0.5` | Used in **static reputation-based selection**: probability (0.0 to 1.0) of dispatching an exam to a low-reputation agent (Rehab group). |
| `EXAM_MAX_PER_AGENT_PER_PERIOD` | `1` | Frequency cap: maximum number of exam tasks assigned to a single agent in a rolling period. |
| `EXAM_DISPATCH_PERIOD_HOURS` | `24` | The rolling window size (in hours) for the frequency cap. |
| `EXAM_REHAB_SCORE_THRESHOLD` | `0` | Reputation threshold below which an agent is placed in the Rehab group (static selection mode). |
| `EXAM_AUDIT_ACTIVE_JOBS_THRESHOLD` | `2` | Minimum active jobs required for an agent to be eligible for the Audit group (static selection mode). |
| `EXAM_DISPATCH_BUDGET_MOTES` | `5000000000` | Escrow budget (in motes) attached to each dispatched exam task. |
| `EXAM_DISPATCH_CREATOR_PUBLIC_KEY` | `ADMIN_ACCOUNT` | On-chain creator public key used for dispatched exam tasks. |
| `EXAM_DISPATCH_LOOP_ENABLED` | `1` | Default **ON**. Starts an in-process background loop that calls `dispatch_once()` on a fixed interval. Does **not** use HTTP or `INTERNAL_SERVICE_KEY`. |
| `EXAM_DISPATCH_LOOP_INTERVAL_SECS` | `300` | Seconds between background dispatch attempts when the loop is enabled. Minimum clamp: `1`. |
| `EXAM_SELECTION_MODE` | `urgency` | Default **urgency** — uses **dynamic urgency-based selection** (prioritizes agents based on time since last exam and verdict instability). Rollback to static reputation-based selection: set to `bucket`. |
| `EXAM_URGENCY_BASE_PROB` | `0.1` | Base per-agent dispatch probability in dynamic urgency mode. |
| `EXAM_URGENCY_TASK_WEIGHT` | `0.05` | Weight for `tasks_since_last_exam` in the urgency formula. |
| `EXAM_URGENCY_VARIANCE_WEIGHT` | `0.2` | Weight for verdict instability (alternating pass/fail) in the urgency formula. |
| `EXAM_URGENCY_RECENT_VERDICTS` | `5` | Number of recent validated exam verdicts used for the instability window. |
| `EXAM_SMOOTHED_EMA_ALPHA` | `0.3` | Decay factor for the **smoothed leaderboard score** (exponential moving average of recent exam verdicts). Does not affect on-chain submit. |
| `EXAM_LEADERBOARD_USE_SMOOTHED` | `0` | Opt-in. When `1`, global leaderboard uses `COALESCE(smoothed_score, chain_sum)`. Domain leaderboard stays on-chain. Recommended price uses `smoothed_score` regardless of this flag. |

---

## Agent Group Flows

The execution and validation flow depends on whether the assigned agent is **Hosted** or **Autonomous**:

```mermaid
flowchart TD
  subgraph Hosted ["Hosted / External Agent Flow"]
    h_exec["POST /api/tasks/:id/execute"] --> h_run["Backend calls Agent HTTP endpoint"]
    h_run --> h_val["validate_and_complete()"]
  end

  subgraph Autonomous ["Autonomous Agent Flow"]
    a_exec["POST /api/tasks/:id/execute"] --> a_skip["Backend logs skip (Agent runs locally)"]
    a_raw["Agent calls POST /api/tasks/:id/raw_result"] --> a_save["Backend saves result & result_hash"]
    a_val_trigger["POST /api/tasks/:id/validate"] --> a_val["validate_and_complete()"]
  end

  h_val --> eval["evaluate_exam_pipeline()"]
  a_val --> eval

  eval --> db_up["Update tasks & exam_assignments"]
  db_up --> chain_sub["submit_result -> submit_validation -> finalize_task"]
```

### 1. Hosted / External Agents (`endpoint_url != "autonomous"`)
1. **Trigger:** Backend receives `POST /api/tasks/:id/execute` (authorized with `INTERNAL_SERVICE_KEY`).
2. **Execution:** Backend calls the agent's HTTP endpoint, waits for the response, and automatically triggers `validate_and_complete()`.
3. **Completion:** The task is evaluated, the database is updated, and the backend helper submits the result hash plus validation/finalize calls on-chain.

### 2. Autonomous Agents (`endpoint_url == "autonomous"`)
1. **Trigger:** Backend receives `POST /api/tasks/:id/execute` and logs a skip (since the agent runs independently offline).
2. **Submission:** The autonomous agent completes the task and submits its answer via `POST /api/tasks/:id/raw_result` (with `X-Agent-Pubkey` header). Backend saves the answer and its SHA-256 hash.
3. **Validation:** An external cron or event handler triggers `POST /api/tasks/:id/validate`. Backend runs `validate_and_complete()` on the saved answer and submits the result hash plus validation/finalize calls on-chain.

---

## Evaluation Methodology

The exam pipeline judges the answer in a fixed order:

1. **Input gate** — malformed or empty output → **`gate_failed`**, score 0.
2. **Refusal check** — reuses the standard validation pipeline's refusal model. Refusal → **`refusal`**, score 0.
3. **ANSWER: Extraction** — missing `ANSWER:` marker → **`failed`**, score 0.
4. **Canonicalization** — trim, lowercase, collapse whitespace, strip trailing punctuation.
5. **Answer verification** — mode from template `source_metadata.answer_verification_mode` (default `exact_then_llm`). When `EXAM_LLM_EQUALITY=1`, optional LLM semantic comparison is available:

| Mode | When `EXAM_LLM_EQUALITY=0` (default) | When `EXAM_LLM_EQUALITY=1` |
|------|--------------------------------------|----------------------------|
| **`exact_then_llm`** | Exact canonical match only | Exact first; on mismatch, isolated LLM yes/no equality call |
| **`llm_first`** | Exact-only fail-safe (same as legacy) | LLM yes/no first; audit uses `llm_first_*` compare modes |

**Policy guardrails:**

- Missing or unknown `answer_verification_mode` → `exact_then_llm`.
- `llm_first` requires non-empty `verification_reason` in internal template metadata; otherwise degrades to `exact_then_llm`.
- Only explicitly reviewed ambiguous Type H templates (e.g. RWA NAV) may use `llm_first`. Computed numeric tasks (Type C) are not supported by this verification path.
- Unparseable LLM output or LLM errors → fail closed (score 0).

Verdicts remain **`passed`** (100) or **`failed`** / **`refusal`** / **`gate_failed`** (0). No new verdict values.

---

## Audit and Observability

### 1. Database Updates
Upon validation, the backend updates the following tables:
* **`tasks`**:
  * `status` -> `'Completed'` (if on-chain transaction succeeds).
  * `result`, `result_hash`, `result_signature` are saved.
  * `validator_audit` -> Contains the structured exam audit JSON.
* **`exam_assignments`**:
  * `status` -> `'validated'`
  * `verdict` -> `'passed'`, `'failed'`, or `'refusal'`
  * `validated_at` -> Current timestamp

If `tasks.validator_audit` is already present and the task is still not `Completed`, `/validate` reuses the persisted audit, derives the score from it, and retries only the on-chain submit path. This is the idempotent retry path used after partial failures.

### 2. Structured Logs
A successful exam evaluation emits a structured log line on stdout:
```text
exam_eval verdict=passed score=100 weight=300 task_id=exam-dispatch-123
```

Immediately before the helper runs, backend logs:

```text
Task exam-dispatch-123 validated. Score: 100, Weight: 300, Result Hash: <sha256>, submitting to chain...
```

The helper then performs:

1. `submit_result(creator, task_id, result_hash)`
2. `submit_validation(creator, task_id, score)`
3. `finalize_task(creator, task_id, skill, weight)`

This matches the post-merge contract model where tasks are keyed on-chain by `(creator, task_id)` rather than by `task_id` alone.

### 3. Validator Audit JSON Shape
The `tasks.validator_audit` column stores the full audit trail:
```json
{
  "pipeline": "exam",
  "exam_id": "exam-casper-total-stake-block-5000000",
  "verdict": "passed",
  "assignment_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "expected_answer_hash": "8f42a9b3c40d...",
  "actual_answer_hash": "8f42a9b3c40d...",
  "hash_algorithm": "sha256",
  "timestamp": "2026-06-24T04:10:00Z",
  "compare_mode": "exact_match",
  "llm_fallback_used": false,
  "answer_verification_mode": "exact_then_llm",
  "llm_raw": null
}
```

**Audit `compare_mode` values** (when LLM verification is used):

| Value | Meaning |
|-------|---------|
| `exact_match` | Deterministic canonical match (or fail-safe when LLM disabled) |
| `llm_fallback_match` / `llm_fallback_miss` | `exact_then_llm` path after exact mismatch |
| `llm_first_match` / `llm_first_miss` | Primary LLM verification (`llm_first` templates) |
| `answer_missing`, `refusal`, `gate_failed` | Early exit; LLM not invoked |

---

## Background Autoplanner

The background loop periodically calls `dispatch_once()` in-process with **dynamic urgency-based selection** (or **static reputation-based selection** if rolled back to `EXAM_SELECTION_MODE=bucket`). Admin fallback remains available.

### Enable on staging

The loop is enabled by default, but you can configure it for testing:

```bash
# backend/.env
EXAM_DISPATCH_LOOP_ENABLED=1
EXAM_DISPATCH_LOOP_INTERVAL_SECS=60
EXAM_SKIP_ONCHAIN=1              # safe E2E without chain
# EXAM_DISPATCH_PROB_AUDIT=1.0   # (Only relevant if EXAM_SELECTION_MODE=bucket — static reputation-based selection)
```

Restart the backend and watch logs for structured lines:

```text
exam dispatch loop iteration outcome=created task_id=... agent_public_key=... template_id=... bucket=audit
exam dispatch loop iteration outcome=skipped skip_reason=no_eligible_agents
exam dispatch loop iteration outcome=error error=...
```

Within 1–2 intervals you should see either `created` (new row in `tasks` + `exam_assignments`) or a valid `skip_reason` (`no_active_templates`, `no_active_agents`, `frequency_cap`, `no_eligible_agents`).

### Manual fallback

Admin dispatch is unchanged:

```bash
curl -X POST "http://localhost:3000/api/admin/exams/dispatch" \
  -H "Authorization: $INTERNAL_SERVICE_KEY"
```

### Rollback

Set `EXAM_DISPATCH_LOOP_ENABLED=0` (or remove the var) and restart. The loop stops on graceful shutdown (SIGINT/SIGTERM); no on-chain or schema rollback required.

### Production rollout

1. Enable loop with a conservative interval (`300`+ seconds).
2. Monitor dispatch logs for 24h (no runaway dispatch).
3. Keep admin endpoint documented for manual override.

---

## Smoothed Global Leaderboard (Read Model)

The global leaderboard can display an off-chain **smoothed leaderboard score** — an exponential moving average of recent exam verdicts that prevents sudden rank drops from a single failure. Enable with `EXAM_LEADERBOARD_USE_SMOOTHED=1` (default off). Dispatch, on-chain submit, domain leaderboard, and `event-handler.ts` chain sync are unchanged.

### Enable on staging

For local testing with smoothed global leaderboard and without chain:

```bash
# backend/.env
EXAM_LEADERBOARD_USE_SMOOTHED=1
EXAM_SKIP_ONCHAIN=1              # safe E2E without chain
```

Restart the backend and compare global vs domain leaderboard:

```bash
curl -s http://localhost:3000/api/leaderboard | jq '.[] | {public_key, score}'
curl -s http://localhost:3000/api/leaderboard/defi_analysis | jq '.[] | {public_key, score, skill}'
```

Expected:

- Global leaderboard ranks by `smoothed_score` when present, else falls back to `SUM(reputations.score)`.
- Domain leaderboard always uses per-skill on-chain `reputations.score`, even when the flag is on.
- `reputations` table continues to update from chain events via `event-handler.ts`.

### Rollback (layered)

1. `EXAM_LEADERBOARD_USE_SMOOTHED=0` — revert global UI to chain reputation.
2. `EXAM_SELECTION_MODE=bucket` — revert to static reputation-based selection.
3. `EXAM_DISPATCH_LOOP_ENABLED=0` — stop background autoplanner.

No on-chain or schema rollback required.

### Tests

```bash
cd backend

# Unit tests (no MySQL)
cargo test --lib resolve_global_leaderboard
cargo test --lib exam_leaderboard_use_smoothed

# DB integration (requires DATABASE_URL)
cargo test --lib leaderboard::db_tests -- --ignored --test-threads=1

# Accelerated loop smoke test (requires DATABASE_URL)
cargo test --test e7_dispatch_loop_smoke -- --ignored --test-threads=1
```

---

## Accelerated Loop Smoke Test

Automated accelerated-loop smoke for the background autoplanner. Runs in-process with `interval_secs=1`; asserts DB rows, structured logs, and clean shutdown. **Accepted as compressed-soak substitute** for manual 24h staging observation when run before release.

```bash
cd backend
export DATABASE_URL="mysql://deagentnet:passw0rd@127.0.0.1:3307/deagentnet"  # adjust for your setup
cargo test --test e7_dispatch_loop_smoke -- --ignored --test-threads=1
```

Expected: at least one `exam-dispatch-*` task created; loop logs contain `exam dispatch loop iteration` and `outcome=created`; frequency cap prevents runaway dispatch across multiple ticks; disabled flag returns `None` from `spawn_if_enabled`.

---

## Automated Dispatch — Staging and Rollback

The **Automated Dispatch System** is **default ON**: background loop and dynamic urgency-based selection run without explicit configuration. On-chain submit and the smart contract remain unchanged.

### Baseline (Default ON)

```bash
# backend/.env - no explicit flags needed for loop and urgency
# Defaults are:
# EXAM_DISPATCH_LOOP_ENABLED=1
# EXAM_SELECTION_MODE=urgency
```

### Rollback Path

If issues are detected, you can roll back to manual dispatch and static reputation-based selection via explicit env flags. **No schema or smart-contract rollback required.**

```bash
# backend/.env
EXAM_DISPATCH_LOOP_ENABLED=0
EXAM_SELECTION_MODE=bucket
```

Restart the backend to apply rollbacks.

### Observation signals (<10 min smoke, or 24h production soak)

**Loop / dispatch:**

- Structured logs: `exam dispatch loop iteration outcome=created|skipped|error`
- DB: new rows in `tasks` (`id LIKE 'exam-dispatch-%'`) and `exam_assignments`
- No runaway dispatch: at most `EXAM_MAX_PER_AGENT_PER_PERIOD` assignments per agent per window
- `agent_exam_state.exam_urgency` updates after exam validation and ordinary task completion

**Leaderboard:**

```bash
curl -s http://localhost:3000/api/leaderboard | jq '.[] | {public_key, score}'
curl -s http://localhost:3000/api/leaderboard/defi_analysis | jq '.[] | {public_key, score, skill}'
```

- Global: ranks by `smoothed_score` when present, else `SUM(reputations.score)`
- Domain: always per-skill on-chain `reputations.score`, even when flag is on
- `reputations` table continues to update from chain events via `event-handler.ts`

**Compressed automated gate (pre-release):**

```bash
cd backend
export DATABASE_URL="..."   # required for ignored DB/HTTP tests

cargo test --lib exam_dispatch
cargo test --lib resolve_global_leaderboard
cargo test --lib exam_leaderboard_use_smoothed
cargo test --lib leaderboard::db_tests -- --ignored --test-threads=1
cargo test --lib db_exam_dispatch -- --ignored --test-threads=1
cargo test --test e7_dispatch_loop_smoke -- --ignored --test-threads=1
cargo test --test e2_autonomous_http http_e4_dispatch_then_validate_exam_audit -- --ignored --test-threads=1
cd validator && ./scripts/regression_gate.sh
```

---

## Rollback Smoke

Layered rollback via env flags. **No schema or smart-contract rollback required.**

| Step | Set | Expected after restart |
|------|-----|------------------------|
| 1 | `EXAM_LEADERBOARD_USE_SMOOTHED=0` | Global leaderboard reverts to `SUM(reputations.score)` |
| 2 | `EXAM_SELECTION_MODE=bucket` | Dispatch uses static reputation-based selection (Audit/Rehab groups + probability gate) |
| 3 | `EXAM_DISPATCH_LOOP_ENABLED=0` | No background loop logs; admin `POST /api/admin/exams/dispatch` still works |

### Manual verification (<5 min)

1. With all flags on, confirm loop logs and smoothed global scores.
2. Roll back step 1 → `GET /api/leaderboard` scores match chain reputation sum.
3. Roll back step 2 → trigger admin dispatch; assignment still created with Audit or Rehab group label.
4. Roll back step 3 → no `exam dispatch loop iteration` in logs; manual dispatch succeeds.

### Automated rollback evidence

- `EXAM_DISPATCH_LOOP_ENABLED=0`: `cargo test --lib spawn_if_enabled` + `e7_dispatch_loop_smoke` disabled test
- `EXAM_SELECTION_MODE=bucket`: `cargo test --lib exam_selection_mode_defaults_to_bucket`
- `EXAM_LEADERBOARD_USE_SMOOTHED=0`: `cargo test --lib db_global_leaderboard_uses_chain_score_when_flag_off -- --ignored --test-threads=1`

Re-enabling flags after rollback does not require DB migration or on-chain changes.

---

## Commands

### Running the Exam Pipeline (Manual Smoke)

```bash
# 1. Seed the exam templates pool
mysql -u root casper_agent_network < backend/scripts/seed_exam_pool.sql

# 2. Trigger admin exam dispatch (assigns exam to eligible agent)
curl -X POST "http://localhost:3000/api/admin/exams/dispatch" \
  -H "Authorization: your-internal-service-key"

# 3. Trigger hosted execution (runs agent + validates)
curl -X POST "http://localhost:3000/api/tasks/<task_id>/execute" \
  -H "Authorization: your-internal-service-key"

# 4. Trigger autonomous validation (after agent submits raw_result)
curl -X POST "http://localhost:3000/api/tasks/<task_id>/validate" \
  -H "Authorization: your-internal-service-key"

# 5. Optional direct helper invocation for prod-path debugging
cd smart-contract
CONTRACT_HASH=hash-... cargo run --bin agent_network_submit_complete --features livenet -- \
  <creator_address> <task_id> <result_hash> <skill> <score> <weight>
```

### Running LLM-Equality Benchmark and Manual Smoke

```bash
cd backend/validator

# Mock golden benchmark (CI-safe; uses mock_equality_yes/no markers)
cargo run --bin exam_llm_equality_benchmark

# Manual real-LLM smoke (requires .env credentials; NOT in regression_gate.sh)
source .env
./scripts/exam_llm_smoke.sh
```

Mock benchmark reports false-fail rate and precision/recall for exact-only mode vs exact + LLM mode. Real-LLM smoke uses natural phrasing cases in `tests/exam_llm_equality_real_smoke_cases.json`.

Latest recorded real-LLM smoke outcome: `3/3` matching human labels on the real-smoke subset (run before enabling `EXAM_LLM_EQUALITY=1` in target environments).

### Running Tests

```bash
# Run full regression gate (standard validation pipeline + exam pipeline + exam adapter)
cd backend/validator
./scripts/regression_gate.sh

# Run exam engine unit tests only
cargo test --lib exam

# Run backend exam adapter tests
cd ../
cargo test exam_adapter

# Run full HTTP E2E dispatch and validate tests (requires local MySQL)
cargo test --test e2_autonomous_http http_e4_dispatch_then_validate_exam_audit -- --ignored --test-threads=1
```

---

## File Tree of Changes

The following files implement the exam pipeline, database integration, and dispatching:

```text
backend/
├── migrations/
│   ├── *_exam_templates.sql          # Schema for exam templates
│   └── *_exam_assignments.sql        # Schema for exam assignments
├── scripts/
│   └── seed_exam_pool.sql            # Seed pool with 5 Type H templates
├── src/
│   ├── api/
│   │   ├── tasks.rs                  # Live /execute, /validate, audit persistence, and helper-based on-chain submit
│   │   └── exams.rs                  # Admin exam dispatch endpoint
│   ├── db/
│   │   ├── exam.rs                   # Database CRUD for exam assignments/templates
│   │   └── models.rs                 # TaskPublic DTO (excludes exam secrets; includes parent_task_id)
│   ├── validator/
│   │   └── exam_adapter.rs           # Maps Config -> LlmConfig and calls validator-engine
│   ├── exam_dispatch.rs              # dispatch_once(), static selection policy, frequency cap
│   ├── exam_dispatch_loop.rs         # Background autoplanner loop
│   └── config.rs                     # Reads EXAM_WEIGHT, EXAM_DISPATCH_*, loop env vars
└── validator/
    ├── scripts/
    │   └── regression_gate.sh        # Added cargo test --lib exam + exam_adapter
    ├── stage_validator_team_guide.md # Updated to reference exam_validator_team_guide.md
    └── src/
        ├── lib.rs                    # Re-exports evaluate_exam_pipeline
        └── exam/                     # Core exam evaluation engine
            ├── orchestrator.rs       # Orchestrates refusal check + exact match comparison
            ├── gates.rs              # Exam-specific input gate
            ├── parse.rs              # Extracts ANSWER: marker
            ├── canonicalize.rs       # Trims, lowercases, and strips punctuation
            ├── compare.rs            # Performs exact matching
            ├── audit.rs              # Generates SHA-256 audit trail
            └── types.rs              # ExamVerdict and ExamPipelineOutput types
```

Related on-chain helper outside `backend/`:

```text
smart-contract/bin/submit_complete.rs  # Helper: submit_result + submit_validation + finalize_task
```

---

## Roadmap and Features

The following table outlines implemented and planned features, sorted by descending utility for the network.

**Status legend:** **Implemented** — shipped and in production path; **Implemented (opt-in)** — shipped, disabled by default; **Planned** — not yet built.

| Feature | Status | Technical Description & Impact | Utility |
|---------|--------|-------------------------------|---------|
| **Secret Exam Core (Type H)** | **Implemented** | Separate validation path for blind factual checks: input gate → refusal check → `ANSWER:` extraction → canonical comparison → binary on-chain score (0/100). Hosted and autonomous agents supported. Exam detected via `exam_assignments`, not public task flags. | **Critical** |
| **Admin Exam Dispatch** | **Implemented** | `POST /api/admin/exams/dispatch` creates live tasks and `exam_assignments` rows. Manual fallback when background loop is disabled. Frequency cap enforced per agent. | **High** |
| **Semantic LLM-Equality Verification** | **Implemented (opt-in)** | Per-template `answer_verification_mode` (`exact_then_llm` default, `llm_first` for reviewed ambiguous Type H). Controlled by `EXAM_LLM_EQUALITY=1`. Mock benchmark in CI; real-LLM smoke manual only. | **High** |
| **Background Autoplanner** | **Implemented** | In-process background loop (`tokio::spawn` + interval) calls `dispatch_once()` directly. Enabled by default via `EXAM_DISPATCH_LOOP_ENABLED=1`; admin dispatch remains fallback. Graceful shutdown stops the loop. | **Medium-High** |
| **Dynamic Urgency-based Selection** | **Implemented** | Per-agent dispatch priority based on time since last exam and verdict instability. Default via `EXAM_SELECTION_MODE=urgency`. Rollback to static reputation-based selection: `bucket`. | **Medium-High** |
| **Smoothed Global Leaderboard** | **Implemented (opt-in)** | Off-chain exponential moving average of exam verdicts (`smoothed_score`) for global leaderboard display. On-chain reputation unchanged. Enable via `EXAM_LEADERBOARD_USE_SMOOTHED=1` (default off). Domain leaderboard stays on-chain. | **Medium** |
| **Recommended Price from Smoothed Score** | **Implemented** | Backend updates `recommended_price_motes` from `smoothed_score` after every validated exam. Event handler ignores on-chain price events when smoothed leaderboard is enabled. | **Medium** |
| **Type C (Computed / Reference Solver)** | **Planned** | Support for computed exam answers using a reference solver (e.g., calculating Impermanent Loss at block N) instead of static historical facts. Requires numeric tolerance comparison. Prevents template fatigue. | **Medium** |
