#!/usr/bin/env bash
# SPEC-006 P9: orchestrator deletion must not use get_all_* (bounded GraphScanOps only).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$ROOT/edgequake/crates/edgequake-core/src/orchestrator/deletion.rs"

MATCHES=$(rg 'get_all_nodes\(\)|get_all_edges\(\)' "$TARGET" 2>/dev/null || true)

if [[ -n "$MATCHES" ]]; then
  echo "SPEC-006 violation: orchestrator/deletion.rs must use GraphScanOps, not get_all_*:"
  echo "$MATCHES"
  exit 1
fi

if ! rg -q 'find_nodes_by_source_prefixes' "$TARGET"; then
  echo "SPEC-006 violation: deletion.rs must call find_nodes_by_source_prefixes"
  exit 1
fi

if ! rg -q 'find_edges_by_source_prefixes' "$TARGET"; then
  echo "SPEC-006 violation: deletion.rs must call find_edges_by_source_prefixes"
  exit 1
fi

echo "✓ SPEC-006: orchestrator deletion bounded (no get_all_*)"
