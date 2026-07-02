#!/usr/bin/env bash
# SPEC-018: Run all observability proof commands (non-interactive).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
EQ="$ROOT/edgequake"
WEBUI="$ROOT/edgequake_webui"

cd "$EQ"

echo "== edgequake-observability =="
cargo test -p edgequake-observability --lib

echo "== edgequake-tasks =="
cargo test -p edgequake-tasks --lib

echo "== edgequake-api observability =="
cargo test -p edgequake-api --test observability_proof --features postgres
cargo test -p edgequake-api --lib test_request_id_header_added --features postgres 2>/dev/null || \
  cargo test -p edgequake-api --test integration_tests test_request_id_header_added --features postgres

echo "== edgequake-audit =="
cargo test -p edgequake-audit --lib

echo "== edgequake-api lib (smoke) =="
cargo test -p edgequake-api --lib --features postgres -- handlers::query::tests::test_query_success

echo "== edgequake-webui =="
cd "$WEBUI"
bun test src/lib/api/__tests__/observability-client.test.ts

echo "SPEC-018: all proof commands passed."
