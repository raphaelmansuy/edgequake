#!/usr/bin/env bash
# SPEC-090 F-090-29 — falsifiable local scaling smoke (release-gate friendly).
# Requires DATABASE_URL and a running Postgres with pgvector.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT/edgequake"
: "${DATABASE_URL:=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake}"
export DATABASE_URL
export EDGEQUAKE_REQUIRE_POSTGRES_TESTS="${EDGEQUAKE_REQUIRE_POSTGRES_TESTS:-0}"

echo "== SPEC-090 Wave 1 =="
cargo test -p edgequake-storage --features postgres --test e2e_spec090_wave1 -- --nocapture
echo "== SPEC-090 Wave 2 =="
cargo test -p edgequake-storage --features postgres --test e2e_spec090_wave2 -- --nocapture
echo "== SPEC-090 verify (full closeout) =="
cargo test -p edgequake-storage --features postgres --test e2e_spec090_verify -- --nocapture
echo "== SPEC-090 multi-pool isolation =="
cargo test -p edgequake-storage --features postgres --test e2e_spec090_multi_pool -- --nocapture
echo "== SPEC-090 claim bounded + task verify =="
cargo test -p edgequake-tasks --features postgres --test e2e_spec090_claim_bounded -- --nocapture
echo "== migration checksums =="
"$ROOT/scripts/check_migration_checksums.sh"
echo "PASS: SPEC-090 scaling smoke"
