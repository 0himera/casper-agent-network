#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ "${VALIDATOR_MOCK_LLM:-}" == "1" ]]; then
  echo "VALIDATOR_MOCK_LLM=1 — uplift requires a real judge LLM. Unset VALIDATOR_MOCK_LLM."
  exit 1
fi

echo "Running real-LLM few-shot A/B (baseline v1 vs treatment v2)..."
cargo test --test soft_uplift few_shot_uplift_real_llm_ab -- --ignored --nocapture
