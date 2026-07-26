#!/usr/bin/env bash
# Wave 4 evil-test runner (no LLM, no testnet). Requires Docker MySQL on :3307.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export DATABASE_URL="${DATABASE_URL:-mysql://deagentnet:passw0rd@127.0.0.1:3307/deagentnet}"
export DB_URI="${DB_URI:-$DATABASE_URL}"
export VALIDATOR_MOCK_LLM="${VALIDATOR_MOCK_LLM:-1}"
export EXAM_SKIP_ONCHAIN="${EXAM_SKIP_ONCHAIN:-1}"
export VALIDATOR_FACTUALITY="${VALIDATOR_FACTUALITY:-0}"

echo "=== Wave 4 evil tests ==="
echo "DATABASE_URL=$DATABASE_URL"
echo "DB_URI=$DB_URI"

echo ""
echo "--- validator-engine gates (scenario 5) ---"
(cd "$ROOT" && cargo test -p validator-engine fixture_free_boundary_ -- --nocapture)

echo ""
echo "--- validator-node unit (scenario 1) ---"
(cd "$ROOT" && cargo test -p validator-node test_validator_loop_missing_pubkey -- --nocapture)

echo ""
echo "--- validator-node ignored DB (scenarios 2–4) ---"
(cd "$ROOT" && cargo test -p validator-node -- --ignored --test-threads=1 --nocapture)

echo ""
echo "--- backend audit / x402 / decay ---"
(cd "$ROOT" && cargo test -p backend api::audit -- --ignored --test-threads=1 --nocapture)
(cd "$ROOT" && cargo test -p backend api::x402 -- --ignored --test-threads=1 --nocapture)
(cd "$ROOT" && cargo test -p backend reputation_decay -- --ignored --test-threads=1 --nocapture)

echo ""
echo "--- backend wave4_evil_http (scenarios 12–13) ---"
(cd "$ROOT" && cargo test -p backend --test wave4_evil_http -- --ignored --test-threads=1 --nocapture)

echo ""
echo "--- backend wave4 reputation snapshot (G23–G24) ---"
(cd "$ROOT" && cargo test -p backend --test wave4_reputation_snapshot -- --ignored --test-threads=1 --nocapture)

echo ""
echo "--- backend wave4 parent_task_id round-trip (G25) ---"
(cd "$ROOT" && cargo test -p backend --test wave4_parent_task_roundtrip -- --ignored --test-threads=1 --nocapture)

echo ""
echo "--- server TS runners (B6–B10, E17–E19, G25–G26) ---"
(cd "$ROOT/server" && npm run test:mcp)
(cd "$ROOT/server" && npm run test:mcp-http)
(cd "$ROOT/server" && npm run test:mcp-db)
(cd "$ROOT/server" && npm run test:mcp-write-parity)
(cd "$ROOT/server" && npm run test:event-handler)

echo ""
echo "=== Wave 4 evil tests finished ==="
