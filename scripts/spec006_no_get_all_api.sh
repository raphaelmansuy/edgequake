#!/usr/bin/env bash
# SPEC-006: G-006-05 — fail on unallowlisted get_all_* in API handlers.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ALLOWLIST="$ROOT/specifications/006-ensure-perf/support/get_all_allowlist.txt"
API_SRC="$ROOT/edgequake/crates/edgequake-api/src"

if [[ ! -f "$ALLOWLIST" ]]; then
  echo "Missing allowlist: $ALLOWLIST"
  exit 1
fi

if ! command -v rg >/dev/null 2>&1; then
  echo "SPEC-006 violation: ripgrep (rg) required for static gates"
  exit 1
fi

MATCHES=$(rg 'get_all_nodes\(\)|get_all_edges\(\)' "$API_SRC" 2>/dev/null || true)

if [[ -z "$MATCHES" ]]; then
  echo "✓ SPEC-006: no get_all_* in edgequake-api/src"
  exit 0
fi

VIOLATIONS=""
while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  file="${line%%:*}"
  allowed=false
  while IFS= read -r pattern; do
    [[ -z "$pattern" || "$pattern" =~ ^# ]] && continue
    if [[ "$file" == *"$pattern"* ]]; then
      allowed=true
      break
    fi
  done < "$ALLOWLIST"
  if [[ "$allowed" == false ]]; then
    VIOLATIONS+="$line"$'\n'
  fi
done <<< "$MATCHES"

if [[ -n "$VIOLATIONS" ]]; then
  echo "SPEC-006 violation: unallowlisted get_all_* in API:"
  echo "$VIOLATIONS"
  exit 1
fi

echo "✓ SPEC-006: get_all_* only on allowlisted paths ($(wc -l < "$ALLOWLIST" | tr -d ' ') patterns)"
