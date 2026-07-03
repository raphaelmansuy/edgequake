#!/usr/bin/env bash
# SPEC-042 — Dev E2E proof across PG16, PG17, PG18 dev profiles.
#
# For each profile: switch DB → restart backend → capture /health JSON + proof HTML.
# UI screenshots captured once on the final profile (pg18) unless CAPTURE_UI_EACH=1.
#
# Usage:
#   ./specs/042-update-age-pgvector/e2e/run_dev_e2e_proof_all_profiles.sh
#   PROFILES="pg16 pg17" ./specs/042-update-age-pgvector/e2e/run_dev_e2e_proof_all_profiles.sh
#
# Optional env:
#   SKIP_BATTLE_TESTS=1  — skip docker battle suite at end (default: run once)
#   SKIP_IMAGE_BUILD=1   — passed through to battle tests
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
E2E="$ROOT/specs/042-update-age-pgvector/e2e"
PROFILES="${PROFILES:-pg16 pg17 pg18}"
BACKEND_URL="${BACKEND_URL:-http://localhost:8081}"
FRONTEND_URL="${FRONTEND_URL:-http://localhost:3000}"

chmod +x "$E2E/run_dev_e2e_proof.sh"

echo "== SPEC-042 multi-profile dev E2E proof =="
echo "Profiles: $PROFILES"
echo ""

for profile in $PROFILES; do
  major="${profile#pg}"
  echo "========== Dev E2E: $profile (PostgreSQL $major) =========="

  make -C "$ROOT" kill-app --no-print-directory 2>/dev/null || true

  make -C "$ROOT" "db-start-${profile}" --no-print-directory

  # Avoid orphaned vision PDF jobs destabilizing backend during proof
  if [ -f /tmp/edgequake-db-url ]; then
    psql "$(cat /tmp/edgequake-db-url)" -c \
      "UPDATE eq_tasks SET status='cancelled' WHERE task_type='pdf_processing' AND status IN ('pending','processing','running');" \
      2>/dev/null || true
  fi

  WORKER_THREADS=4 MAX_TASKS_PER_TENANT=1 EDGEQUAKE_PDF_VISION_JOBS=0 \
    make -C "$ROOT" backend-bg BACKEND_PORT=8081 EQ_POSTGRES_PROFILE="$profile" --no-print-directory

  for i in $(seq 1 30); do
    curl -fsS "$BACKEND_URL/health" >/dev/null 2>&1 && break
    sleep 2
  done
  curl -fsS "$BACKEND_URL/health" >/dev/null || {
    echo "ERROR: backend not healthy for $profile at $BACKEND_URL"
    tail -30 /tmp/edgequake-backend.log
    exit 1
  }

  capture_ui=0
  if [ "${CAPTURE_UI_EACH:-0}" = "1" ] || [ "$profile" = "pg18" ]; then
    capture_ui=1
    if ! curl -fsS "$FRONTEND_URL" >/dev/null 2>&1; then
      make -C "$ROOT" frontend-bg --no-print-directory
      for i in $(seq 1 20); do
        curl -fsS "$FRONTEND_URL" >/dev/null 2>&1 && break
        sleep 2
      done
    fi
  fi

  EQ_POSTGRES_PROFILE="$profile" \
    BACKEND_URL="$BACKEND_URL" \
    FRONTEND_URL="$FRONTEND_URL" \
    SKIP_BATTLE_TESTS=1 \
    CAPTURE_UI="$capture_ui" \
    "$E2E/run_dev_e2e_proof.sh"

  # Verify postgres major in health matches profile
  python3 - <<PY "$ROOT/specs/042-update-age-pgvector/e2e/screenshots/health-${profile}.json" "$major"
import json, sys
from pathlib import Path
health = json.loads(Path(sys.argv[1]).read_text())
age = health.get("operational", {}).get("migration", {}).get("age_extversion", "")
pv = health.get("operational", {}).get("migration", {}).get("pgvector_extversion", "")
status = health.get("status")
mig = health.get("schema", {}).get("latest_version")
print(f"  ✓ $profile: status={status} migrations={mig} pgvector={pv} age={age}")
expected_major = int(sys.argv[2])
if expected_major == 16 and not age.startswith("1.6"):
    raise SystemExit(f"AGE version {age} unexpected for PG16 (expected 1.6.x)")
if expected_major in (17, 18) and not age.startswith("1.7"):
    raise SystemExit(f"AGE version {age} unexpected for PG{expected_major} (expected 1.7.x)")
if not pv.startswith("0.8"):
    raise SystemExit(f"pgvector version {pv} unexpected (expected 0.8.x)")
PY
  echo ""
done

if [ "${SKIP_BATTLE_TESTS:-0}" != "1" ]; then
  echo "→ SPEC-042 docker battle test suite (pg16 + pg17 + pg18)..."
  chmod +x "$E2E/run_all_battle_tests.sh"
  SKIP_IMAGE_BUILD="${SKIP_IMAGE_BUILD:-1}" "$E2E/run_all_battle_tests.sh"
fi

echo ""
echo "✓ Multi-profile dev E2E proof complete"
ls -la "$ROOT/specs/042-update-age-pgvector/e2e/screenshots/"
