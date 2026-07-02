#!/usr/bin/env bash
# SPEC-006 resource safety proof runner.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT/edgequake"

echo "== SPEC-006: edgequake-core resource budget =="
cargo test -p edgequake-core resource_budget --quiet
cargo test -p edgequake-core default_token_budget_matches_resource_ssot --features pipeline --quiet

echo "== SPEC-006: edgequake-storage graph_scan_ops =="
cargo test -p edgequake-storage graph_scan_ops --quiet

echo "== SPEC-006: edgequake-api resource_safety proof =="
cargo test -p edgequake-api resource_safety --features postgres --quiet
cargo test -p edgequake-api graph_materialization --lib --features postgres --quiet

echo "== SPEC-006: migration readiness battle test =="
cargo test -p edgequake-api --test migration_readiness_proof --features postgres --quiet

echo "== SPEC-006: e2e_document_deletion shared-entity smoke =="
cargo test -p edgequake-api test_delete_preserves_shared_entities --features postgres --quiet

postgres_bootstrap_ready() {
  if [[ -z "${DATABASE_URL:-}" && -z "${POSTGRES_PASSWORD:-}" ]]; then
    return 1
  fi
  local host="${POSTGRES_HOST:-localhost}"
  local port="${POSTGRES_PORT:-5432}"
  if command -v pg_isready >/dev/null 2>&1; then
    pg_isready -h "$host" -p "$port" -q 2>/dev/null
  else
    timeout 2 bash -c "echo >/dev/tcp/$host/$port" 2>/dev/null
  fi
}

if postgres_bootstrap_ready; then
  echo "== SPEC-006: migration bootstrap postgres e2e =="
  cargo test -p edgequake-api --test migration_bootstrap_proof --features postgres --quiet
else
  echo "== SPEC-006: migration bootstrap postgres e2e (skipped — no reachable Postgres) =="
fi

echo "== SPEC-006: static gates =="
"$ROOT/scripts/spec006_no_get_all_api.sh"
"$ROOT/scripts/spec006_budget_catalog_sync.sh"
"$ROOT/scripts/spec006_source_ids_migration.sh"
"$ROOT/scripts/spec006_no_unguarded_community_api.sh"
"$ROOT/scripts/spec006_no_adhoc_resource_budget.sh"
"$ROOT/scripts/spec006_no_get_all_orchestrator.sh"
"$ROOT/scripts/spec006_runbook_env_sync.sh"

echo "✓ SPEC-006 resource-proof passed (P0–P9)"
