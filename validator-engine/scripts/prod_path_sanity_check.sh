#!/usr/bin/env bash
# Gap 3 substitute: verify backend reaches submit-path attempt without EXAM_SKIP_ONCHAIN.
# On-chain success is NOT required — CLI failure still proves branch behavior.
set -euo pipefail

BACKEND_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
LOG_FILE="${TMPDIR:-/tmp}/prod_path_sanity.log"

if [[ "${EXAM_SKIP_ONCHAIN:-}" == "1" || "${EXAM_SKIP_ONCHAIN:-}" == "true" ]]; then
  echo "ERROR: EXAM_SKIP_ONCHAIN must be unset (only 1/true enable skip)" >&2
  exit 1
fi

unset EXAM_SKIP_ONCHAIN
export VALIDATOR_MOCK_LLM=1

echo "== prod-path branch sanity: unit (skip flag off) =="
cd "$BACKEND_ROOT"
cargo test needs_submit_retry_when_audit_present_and_not_completed_without_skip

if [[ -z "${DATABASE_URL:-}" && -f "$BACKEND_ROOT/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "$BACKEND_ROOT/.env"
  set +a
fi

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "SKIP: DATABASE_URL not set — unit check passed; set DATABASE_URL for full submit-attempt proof"
  exit 0
fi

echo "== prod-path branch sanity: DB + submit attempt (ignored test) =="
export RUST_LOG="${RUST_LOG:-info}"
cargo test prod_path_branch_sanity_reaches_submit_attempt -- --ignored --test-threads=1 --nocapture 2>&1 | tee "$LOG_FILE"

if grep -q "Skipping on-chain submit" "$LOG_FILE"; then
  echo "FAIL: skip branch was taken (EXAM_SKIP_ONCHAIN=1 path)" >&2
  exit 1
fi

if ! grep -q "submitting to chain" "$LOG_FILE"; then
  echo "FAIL: missing pre-submit log (submitting to chain)" >&2
  exit 1
fi

if ! grep -Eq "Successfully completed task|On-chain transaction failed|Failed to execute on-chain CLI tool" "$LOG_FILE"; then
  echo "FAIL: missing post-submit signal (success, tx failed, or CLI error)" >&2
  exit 1
fi

echo "PASS: prod-path branch sanity check (gap 3 substitute)"
echo "Log saved: $LOG_FILE"
