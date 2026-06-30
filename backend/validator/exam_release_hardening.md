# Exam Release Hardening Notes

This note keeps E5 release-gate details and E6 rollout hardening guidance separate from
`exam_validator_team_guide.md` so the main team guide can stay aligned with upstream.

---

## Seed Review Procedure (E5.5)

Before first prod dispatch, the 3 **required MVP** exam templates must have human
reviewer notes on file. Optional templates
(`exam-curve-3pool-tvl-block-19000000`,
`exam-rwa-tokenized-tbill-nav-2024-q3`) do not block this gate.

**Required templates:**

1. `exam-casper-total-stake-block-5000000`
2. `exam-uniswap-v3-eth-usdc-tvl-block-19000000`
3. `exam-aave-v3-usdc-total-supply-block-19000000`

**Reviewer note format** (one row per template):

| Field | Description |
|-------|-------------|
| `template_id` | Stable slug from `seed_exam_pool.sql` |
| `independent_source` | External source or curated `source_metadata` used for recheck |
| `recalculated_value` | Value obtained from that source |
| `matches_canonical` | `yes` or `no` vs `expected_answer_canonical` in seed |
| `reviewer` | Initials or role |
| `date` | Review date (`YYYY-MM-DD`) |
| `notes` | Short free-text (for example leak check or E0 format) |

**Permanent record:** `backend/validator/documentation/exam_idea_implementation.md`
§ E5.5 `MVP seed review notes`.

**Second reviewer:** For the current MVP release, the E5.5 second-reviewer sign-off is
**waived by policy** (see `exam_idea_implementation.md` § E5.5 `Second reviewer waiver`).
A second reviewer is not required before prod dispatch.

**Do not dispatch to prod** until all 3 required rows are recorded and
`matches_canonical = yes` for each, or canonical values are corrected in
`seed_exam_pool.sql` first. This blocking condition does **not** require a second
reviewer sign-off.

---

## E6 Rollout Checklist

Current recorded smoke outcome is `3/3` matching human labels on the real-smoke subset
for the dev endpoint run on `2026-06-26`. Before target-environment enablement,
re-run smoke on the target provider/model and record the result in the decision doc.

1. Run mock regression:

```bash
cd backend/validator
cargo test --test exam_e6_fail_closed
cd ../
cargo test --test exam_adapter_fail_closed
```

2. Configure `backend/validator/.env` for the target provider/model.
3. Run manual smoke: `./scripts/exam_llm_smoke.sh`
4. Record the result in `documentation/exam_e6_recommendation.md`.
5. Only then set `EXAM_LLM_EQUALITY=1` in the target backend `.env`.

**Failure checks:** missing credentials should fail fast; timeout can be checked with
`VALIDATOR_JUDGE_TIMEOUT_MS=1`; automated fail-closed coverage now lives in the
dedicated tests above instead of the hot E6 source files.
