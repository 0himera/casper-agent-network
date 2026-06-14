#!/usr/bin/env bash
set -euo pipefail

export VALIDATOR_MOCK_LLM=1
cd "$(dirname "$0")/.."

cargo test
cargo test --test golden
