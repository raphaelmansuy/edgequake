#!/usr/bin/env bash
# scripts/check_audit_ignores.sh — every `cargo audit --ignore` must be justified.
#
# First principles: `--ignore` is the only way cargo-audit 0.22 lets a gate pass
# on a finding, so the ignore list IS the security policy. An unreviewed id
# added to a workflow turns a vulnerability into a green check with no trace of
# why. This guard fails when an id has no recorded reason.
#
# The reason lives in docs/security/dependency-remediation-2026-11.md, in a
# section that names the advisory. One line per id is enough.
#
# Usage: ./scripts/check_audit_ignores.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

POLICY="docs/security/dependency-remediation-2026-11.md"
fail=0

if [[ ! -f "$POLICY" ]]; then
  echo "FAIL: policy document missing: $POLICY" >&2
  exit 1
fi

# Collect every ignored id from every workflow that runs an audit.
# Plain grep, not ripgrep: this runs on a stock runner before any tool install.
ids="$(
  grep -hoE 'RUSTSEC-[0-9]{4}-[0-9]{4}' .github/workflows/*.yml 2>/dev/null | sort -u
)"

if [[ -z "$ids" ]]; then
  echo "PASS: no audit ignores to justify"
  exit 0
fi

count=0
for id in $ids; do
  count=$((count + 1))
  if ! grep -qF -- "$id" "$POLICY"; then
    echo "FAIL: $id is ignored in CI but has no entry in $POLICY" >&2
    echo "      Add why it cannot be fixed or why it is unreachable, then retry." >&2
    fail=1
  fi
done

if [[ "$fail" -eq 0 ]]; then
  echo "PASS: all $count audit ignore(s) are justified in $POLICY"
fi
exit "$fail"
