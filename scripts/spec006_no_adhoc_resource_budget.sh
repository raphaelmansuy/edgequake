#!/usr/bin/env bash
# SPEC-006 P8: handlers must use AppState::resource_budget(), not ad-hoc budget constructors.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HANDLERS="$ROOT/edgequake/crates/edgequake-api/src/handlers"

VIOLATIONS=$(rg 'ResourceBudgetConfig::(default|from_env)\(' "$HANDLERS" 2>/dev/null || true)

if [[ -n "$VIOLATIONS" ]]; then
  echo "SPEC-006 violation: handlers must use state.resource_budget() SSOT:"
  echo "$VIOLATIONS"
  exit 1
fi

if ! rg -q 'resource_budget\(\)\.max_upload_bytes' "$ROOT/edgequake/crates/edgequake-api/src/server.rs"; then
  echo "SPEC-006 violation: server.rs must use state.resource_budget().max_upload_bytes"
  exit 1
fi

echo "✓ SPEC-006: resource budget SSOT (handlers + server upload limit)"
