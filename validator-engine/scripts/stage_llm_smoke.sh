#!/usr/bin/env bash
# Real-LLM smoke for stage pipeline S0–S3 on a weaker/cheaper model.
#
# Usage (from backend/validator/):
#   source .env
#   ./scripts/stage_llm_smoke.sh
#
# Optional env overrides for weak-model runs:
#   VALIDATOR_LLM_MODEL=gpt-4o-mini     # model slug for custom/openai-compatible provider
#   VALIDATOR_PROVIDER=openai           # force provider (openai|custom|fireworks|ollama|claude|cloudflare)
#   OPENAI_BASE_URL=https://...         # ProxyAPI or other OpenAI-compatible endpoint
#   OPENAI_API_KEY=sk-...
#   VALIDATOR_MOCK_LLM=0                # required for real LLM (script sets this)
#
# Rate-limit tuning (see documentation/stage_prompt_tuning.md):
#   STAGE_LLM_REQUEST_DELAY_MS=1000
#   STAGE_LLM_RATE_LIMIT_BACKOFF_MS=500

set -euo pipefail

cd "$(dirname "$0")/.."

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

export VALIDATOR_MOCK_LLM=0

if [[ "${VALIDATOR_MOCK_LLM:-}" == "1" ]]; then
  echo "VALIDATOR_MOCK_LLM=1 — real LLM smoke requires mock off. Unset VALIDATOR_MOCK_LLM."
  exit 1
fi

if [[ -z "${OPENAI_API_KEY:-}" && -z "${VALIDATOR_LLM_API_KEY:-}" && -z "${CLAUDE_API_KEY:-}" && -z "${CLOUDFLARE_API_TOKEN:-}" && -z "${OLLAMA_URL:-}" ]]; then
  echo "No judge LLM credentials found. Set OPENAI_API_KEY (or VALIDATOR_LLM_API_KEY / CLAUDE / CLOUDFLARE / OLLAMA)."
  exit 1
fi

echo "Stage pipeline manual smoke (real LLM)"
echo "  VALIDATOR_PROVIDER=${VALIDATOR_PROVIDER:-<auto>}"
echo "  VALIDATOR_LLM_MODEL=${VALIDATOR_LLM_MODEL:-${OPENAI_MODEL:-<provider default>}}"
echo "  OPENAI_BASE_URL=${OPENAI_BASE_URL:-<default>}"

cargo run --bin stage_pipeline_manual_smoke
