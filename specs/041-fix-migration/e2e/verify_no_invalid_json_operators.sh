#!/usr/bin/env bash
# SPEC-041 G1 — Static gate: no invalid ->>> JSON operator in migrations.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
MIGRATIONS="${ROOT}/edgequake/migrations"

echo "== SPEC-041 G1: scan for invalid ->>> operator =="

HITS=$(grep -rFe '->>>' "${MIGRATIONS}" --include='*.sql' 2>/dev/null | grep -v '^[^:]*:--' || true)
# Exclude SQL comment lines that mention the typo in documentation
HITS=$(echo "${HITS}" | grep -vE ':--.*->>>' || true)

if [[ -n "${HITS}" ]]; then
  echo "FAIL: found ->>> in migrations:"
  echo "${HITS}"
  exit 1
fi

# Positive control: ->> must exist in M078 (proves scan path works)
if ! grep -Fe "->>''workspace_id''" "${MIGRATIONS}/078_age_child_workspace_stats.sql" >/dev/null; then
  echo "FAIL: M078 missing expected ->>''workspace_id'' pattern"
  exit 1
fi

echo "PASS: zero ->>> in ${MIGRATIONS}"
echo "PASS: M078 contains correct ->> pattern"
