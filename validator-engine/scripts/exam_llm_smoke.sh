#!/usr/bin/env bash
# Manual real-LLM smoke for exam E6 equality (exact_then_llm + llm_first).
#
# Usage (from backend/validator/):
#   source .env
#   ./scripts/exam_llm_smoke.sh
#
# Requires:
#   EXAM_LLM_EQUALITY=1          (script sets this)
#   VALIDATOR_MOCK_LLM=0         (script sets this)
#   OPENAI_API_KEY or VALIDATOR_LLM_API_KEY / CLAUDE / CLOUDFLARE / OLLAMA
#
# NOT part of regression_gate.sh or CI.

set -euo pipefail

cd "$(dirname "$0")/.."

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

export VALIDATOR_MOCK_LLM=0
export EXAM_LLM_EQUALITY=1

if [[ "${VALIDATOR_MOCK_LLM:-}" == "1" ]]; then
  echo "VALIDATOR_MOCK_LLM=1 — real LLM smoke requires mock off."
  exit 1
fi

if [[ -z "${OPENAI_API_KEY:-}" && -z "${VALIDATOR_LLM_API_KEY:-}" && -z "${CLAUDE_API_KEY:-}" && -z "${CLOUDFLARE_API_TOKEN:-}" && -z "${OLLAMA_URL:-}" ]]; then
  echo "No judge LLM credentials found. Set OPENAI_API_KEY (or VALIDATOR_LLM_API_KEY / CLAUDE / CLOUDFLARE / OLLAMA)."
  exit 1
fi

echo "Exam E6 manual smoke (real LLM)"
echo "  EXAM_LLM_EQUALITY=${EXAM_LLM_EQUALITY}"
echo "  VALIDATOR_PROVIDER=${VALIDATOR_PROVIDER:-<auto>}"
echo "  VALIDATOR_LLM_MODEL=${VALIDATOR_LLM_MODEL:-${OPENAI_MODEL:-<provider default>}}"
echo "  OPENAI_BASE_URL=${OPENAI_BASE_URL:-<default>}"

cargo run --bin exam_llm_equality_manual_smoke
