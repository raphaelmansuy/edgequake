#!/usr/bin/env bash
# SPEC-006 P9: operator runbook env vars must match code anchors (DRY ops/docs).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUNBOOK="$ROOT/specifications/006-ensure-perf/009_operator_runbook.md"

if [[ ! -f "$RUNBOOK" ]]; then
  echo "Missing runbook: $RUNBOOK"
  exit 1
fi

# var_name:code_file (relative to ROOT)
PAIRS=(
  "WORKER_THREADS:edgequake/src/main.rs"
  "MAX_TASKS_PER_TENANT:edgequake/src/main.rs"
  "TASK_PROCESSING_TIMEOUT_SECS:edgequake/src/main.rs"
  "EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS:edgequake/crates/edgequake-pipeline/src/pipeline/config.rs"
  "EDGEQUAKE_GRAPH_SCAN_THRESHOLD:edgequake/crates/edgequake-core/src/resource/budget.rs"
  "EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT:edgequake/crates/edgequake-core/src/resource/budget.rs"
  "EDGEQUAKE_GRAPH_QUERY_TIMEOUT_SECS:edgequake/crates/edgequake-core/src/resource/budget.rs"
  "EDGEQUAKE_MAX_UPLOAD_BYTES:edgequake/crates/edgequake-core/src/resource/budget.rs"
  "EDGEQUAKE_MEM_LIMIT:edgequake/src/main.rs"
)

FAIL=0
for entry in "${PAIRS[@]}"; do
  var="${entry%%:*}"
  file="${entry#*:}"
  path="$ROOT/$file"

  if ! rg -q "$var" "$path" 2>/dev/null; then
    echo "SPEC-006 violation: $var not found in $file"
    FAIL=1
    continue
  fi

  if ! rg -q "$var" "$RUNBOOK" 2>/dev/null; then
    echo "SPEC-006 violation: $var not documented in 009_operator_runbook.md"
    FAIL=1
  fi
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi

echo "✓ SPEC-006: runbook env vars synced with code anchors (${#PAIRS[@]} vars)"
