#!/usr/bin/env bash
set -euo pipefail

export VALIDATOR_MOCK_LLM=1
cd "$(dirname "$0")/.."

echo "== validator-engine stage regression (mock) =="
cargo test --lib stage_regression
cargo test --lib stage_calibration
cargo test --lib stage_factuality

echo "== validator-engine exam regression (mock) =="
cargo test --lib exam

echo "== backend stage integration (mock) =="
cd ../
VALIDATOR_MOCK_LLM=1 cargo test benchmark_adapter
VALIDATOR_MOCK_LLM=1 cargo test benchmark
VALIDATOR_MOCK_LLM=1 cargo test stage_adapter

echo "== backend exam adapter (mock) =="
VALIDATOR_MOCK_LLM=1 cargo test exam_adapter

echo "All stage and exam regression checks passed."
