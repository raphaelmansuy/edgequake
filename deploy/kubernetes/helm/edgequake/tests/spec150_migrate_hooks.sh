#!/usr/bin/env bash
set -euo pipefail
CHART="$(cd "$(dirname "$0")/.." && pwd)"
fail=0
out=$(helm template eq "$CHART" --set postgres.enabled=false)
echo "$out" | grep -q 'pre-install,pre-upgrade' || { echo "FAIL: external DB must use pre-install hook"; fail=1; }
echo "$out" | grep -q 'EDGEQUAKE_SCHEMA_GATE' || { echo "FAIL: SCHEMA_GATE missing"; fail=1; }
echo "$out" | grep -q 'startupProbe' || { echo "FAIL: startupProbe missing"; fail=1; }
out2=$(helm template eq "$CHART" --set postgres.enabled=true)
echo "$out2" | grep -q 'migrate-r1' || { echo "FAIL: bundled DB must use revision Job"; fail=1; }
echo "$out2" | grep -q 'pre-install' && { echo "FAIL: bundled DB must NOT use pre-install hook"; fail=1; } || true
[[ $fail -eq 0 ]] && echo "PASS: SPEC-150 helm migrate wiring"
exit $fail
