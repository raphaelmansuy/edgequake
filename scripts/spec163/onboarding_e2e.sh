#!/usr/bin/env bash
# SPEC-163 hermetic onboarding proof against edgequake-fake-llm.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT/edgequake"
export EDGEQUAKE_ALLOW_MOCK_PROVIDER=1
START=$(date +%s)

run() {
  echo ">> $*"
  ( cd "$ROOT/edgequake" && "$@" )
}

run cargo test -p edgequake-secrets --lib --offline
run cargo test -p edgequake-fake-llm --offline --test shapes
run cargo test -p edgequake-api --lib --offline locality
run cargo test -p edgequake-api --lib --offline ssrf
run cargo test -p edgequake-api --lib --offline doctor
run cargo test -p edgequake-api --lib --offline connection_factory
run cargo test -p edgequake-core --lib --offline connection_id_is_preserved
run cargo test -p edgequake-api --test spec163_probe --offline -- --test-threads=1
run cargo test -p edgequake-api --test contract_spec150_train_head --offline

END=$(date +%s)
ELAPSED=$((END-START))
mkdir -p "$ROOT/specs/163-onboarding-provider-config/reports"
mkdir -p "$ROOT/specs/163-onboarding-provider-config/measurements"
cat > "$ROOT/specs/163-onboarding-provider-config/reports/hermetic.json" <<JSON
{
  "ok": true,
  "elapsed_secs": ${ELAPSED},
  "suites": [
    "edgequake-secrets",
    "edgequake-fake-llm shapes",
    "edgequake-api locality/ssrf/doctor/connection_factory",
    "edgequake-core connection_id",
    "spec163_probe",
    "contract_spec150_train_head"
  ]
}
JSON
python3 - <<PY
import json, time
from pathlib import Path
p = Path("$ROOT/specs/163-onboarding-provider-config/measurements/hermetic.json")
p.write_text(json.dumps({
  "measured_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
  "hermetic_elapsed_secs": ${ELAPSED},
  "budget_secs": 120,
  "within_budget": ${ELAPSED} <= 120,
  "clone_to_validated_connection_commands": 3,
}, indent=2) + "\n")
PY
echo "PASS spec163 hermetic in ${ELAPSED}s"
