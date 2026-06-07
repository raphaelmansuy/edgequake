#!/usr/bin/env bash
# SPEC-006 P6: community detection must be guarded at API boundary.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
API_SRC="$ROOT/edgequake/crates/edgequake-api/src"
STORAGE_LIB="$ROOT/edgequake/crates/edgequake-storage/src/lib.rs"

VIOLATIONS=$(rg 'detect_communities(_unchecked)?\(' "$API_SRC" \
  --glob '!**/graph_community.rs' \
  --glob '!**/mod.rs' 2>/dev/null || true)

if [[ -n "$VIOLATIONS" ]]; then
  echo "SPEC-006 violation: unguarded community detection in API layer:"
  echo "$VIOLATIONS"
  exit 1
fi

if rg -q 'detect_communities,' "$STORAGE_LIB" 2>/dev/null; then
  echo "SPEC-006 violation: detect_communities must not be re-exported from edgequake_storage lib.rs"
  exit 1
fi

if ! rg -q 'detect_communities_unchecked' "$ROOT/edgequake/crates/edgequake-storage/src/community.rs"; then
  echo "SPEC-006 violation: community.rs must define detect_communities_unchecked"
  exit 1
fi

WORKSPACE_VIOLATIONS=$(rg 'detect_communities_unchecked' "$ROOT/edgequake/crates" \
  --glob '!**/community.rs' \
  --glob '!**/graph_community.rs' 2>/dev/null || true)

if [[ -n "$WORKSPACE_VIOLATIONS" ]]; then
  echo "SPEC-006 violation: detect_communities_unchecked only allowed in community.rs + graph_community.rs:"
  echo "$WORKSPACE_VIOLATIONS"
  exit 1
fi

echo "✓ SPEC-006: community detection sealed (API lint + storage non-export + workspace)"
