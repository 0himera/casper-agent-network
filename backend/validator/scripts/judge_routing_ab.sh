#!/usr/bin/env bash
set -euo pipefail

# Manual A/B for judge routing (real LLM). Requires API keys or Ollama.
# Usage: ./scripts/judge_routing_ab.sh

cd "$(dirname "$0")/.."

echo "Running soft-label calibration with default routing..."
cargo test --test soft_uplift compare_few_shot_uplift_real_llm -- --ignored --nocapture

echo "Set VALIDATOR_JUDGE_CASCADE=local_first to compare local-first cascade."
