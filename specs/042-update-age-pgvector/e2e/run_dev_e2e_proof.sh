#!/usr/bin/env bash
# SPEC-042 — Dev stack E2E proof + screenshot capture (pg16 | pg17 | pg18).
#
# Prerequisites: make dev-bg OR make dev running (backend + frontend + postgres)
#
# Usage:
#   ./specs/042-update-age-pgvector/e2e/run_dev_e2e_proof.sh
#   EQ_POSTGRES_PROFILE=pg16 ./specs/042-update-age-pgvector/e2e/run_dev_e2e_proof.sh
#   ./specs/042-update-age-pgvector/e2e/run_dev_e2e_proof_all_profiles.sh
#
# Outputs (per profile, e.g. pg17):
#   specs/042-update-age-pgvector/e2e/screenshots/health-pg17.json
#   specs/042-update-age-pgvector/e2e/screenshots/health-proof-pg17.html
#   specs/042-update-age-pgvector/e2e/screenshots/01-dashboard-pg17.png (when CAPTURE_UI=1)
#
# Optional env:
#   SKIP_BATTLE_TESTS=1  — do not run docker battle suite at end
#   CAPTURE_UI=0         — skip Playwright screenshots
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SHOT_DIR="$ROOT/specs/042-update-age-pgvector/e2e/screenshots"
WEBUI="$ROOT/edgequake_webui"
BACKEND_URL="${BACKEND_URL:-http://localhost:8081}"
FRONTEND_URL="${FRONTEND_URL:-http://localhost:3000}"

if [ -z "${EQ_POSTGRES_PROFILE:-}" ] && [ -f /tmp/edgequake-postgres-profile ]; then
  EQ_POSTGRES_PROFILE="$(cat /tmp/edgequake-postgres-profile)"
fi
EQ_POSTGRES_PROFILE="${EQ_POSTGRES_PROFILE:-pg18}"
PG_MAJOR="${EQ_POSTGRES_PROFILE#pg}"

HEALTH_JSON="$SHOT_DIR/health-${EQ_POSTGRES_PROFILE}.json"
HEALTH_HTML="$SHOT_DIR/health-proof-${EQ_POSTGRES_PROFILE}.html"
DASH_SHOT="$SHOT_DIR/01-dashboard-${EQ_POSTGRES_PROFILE}.png"
DOCS_SHOT="$SHOT_DIR/02-documents-${EQ_POSTGRES_PROFILE}.png"
PHASE_E_SHOT="$SHOT_DIR/03-health-phase-e-${EQ_POSTGRES_PROFILE}.png"

mkdir -p "$SHOT_DIR"

echo "== SPEC-042 dev E2E proof ($EQ_POSTGRES_PROFILE / PG$PG_MAJOR) =="
echo "Backend:  $BACKEND_URL"
echo "Frontend: $FRONTEND_URL"

# 1. Database URL from make db-start
if [ -f /tmp/edgequake-db-url ]; then
  echo "DATABASE_URL: $(cat /tmp/edgequake-db-url)"
else
  echo "WARN: /tmp/edgequake-db-url missing — run make db-start first"
fi

# 2. Health JSON (Phase E fields under operational.storage)
echo "→ Fetching /health ..."
curl -fsS "$BACKEND_URL/health" | python3 -m json.tool > "$HEALTH_JSON"

python3 - <<'PY' "$HEALTH_JSON" "$HEALTH_HTML" "$EQ_POSTGRES_PROFILE" "$PG_MAJOR"
import json, sys, html
from datetime import datetime, timezone
from pathlib import Path

health = json.loads(Path(sys.argv[1]).read_text())
storage = health.get("operational", {}).get("storage", {})
schema = health.get("schema", {})
migration = health.get("operational", {}).get("migration", {})
profile = sys.argv[3]
pg_major = sys.argv[4]

rows = [
    ("profile", profile),
    ("status", health.get("status")),
    ("storage_mode", health.get("storage_mode")),
    ("schema.latest_version", schema.get("latest_version")),
    ("vector_storage_mode", storage.get("vector_storage_mode")),
    ("document_id_generator", storage.get("document_id_generator")),
    ("age_rls_enabled", storage.get("age_rls_enabled")),
    ("age_copy_loader_enabled", storage.get("age_copy_loader_enabled")),
    ("pgvector_extversion", migration.get("pgvector_extversion")),
    ("age_extversion", migration.get("age_extversion")),
    ("postgres_major", pg_major),
]

body = "".join(
    f"<tr><th>{html.escape(str(k))}</th><td><code>{html.escape(str(v))}</code></td></tr>"
    for k, v in rows
)
ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

Path(sys.argv[2]).write_text(f"""<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>EdgeQuake Health — SPEC-042 {profile}</title>
<style>
body {{ font-family: system-ui, sans-serif; margin: 2rem; background: #0f172a; color: #e2e8f0; }}
h1 {{ color: #38bdf8; }}
table {{ border-collapse: collapse; width: 100%; max-width: 720px; }}
th, td {{ border: 1px solid #334155; padding: 0.6rem 1rem; text-align: left; }}
th {{ background: #1e293b; width: 40%; }}
code {{ color: #a5f3fc; }}
</style></head><body>
<h1>EdgeQuake /health — Phase E ({profile})</h1>
<table>{body}</table>
<p>Captured: {ts}</p>
</body></html>""")
PY

# 3. Wait for services
for i in $(seq 1 30); do
  curl -fsS "$BACKEND_URL/health" >/dev/null 2>&1 && break
  sleep 2
done
curl -fsS "$BACKEND_URL/health" >/dev/null || { echo "ERROR: backend not healthy at $BACKEND_URL"; exit 1; }

if [ "${CAPTURE_UI:-1}" = "1" ]; then
  for i in $(seq 1 30); do
    curl -fsS "$FRONTEND_URL" >/dev/null 2>&1 && break
    sleep 2
  done
  curl -fsS "$FRONTEND_URL" >/dev/null || { echo "ERROR: frontend not reachable at $FRONTEND_URL"; exit 1; }
fi

# 4. Playwright screenshots (headless Chromium)
if [ "${CAPTURE_UI:-1}" = "1" ]; then
  if [ -d "$WEBUI/node_modules/@playwright/test" ] || command -v pnpm >/dev/null 2>&1; then
    echo "→ Capturing UI screenshots (Playwright) ..."
    cd "$WEBUI"
    pnpm exec playwright screenshot "$FRONTEND_URL" "$DASH_SHOT" --full-page 2>/dev/null || true
    pnpm exec playwright screenshot "$FRONTEND_URL/documents" "$DOCS_SHOT" --full-page 2>/dev/null || true
    pnpm exec playwright screenshot "file://$HEALTH_HTML" "$PHASE_E_SHOT" --full-page 2>/dev/null || true
  fi
fi

if [ "${SKIP_BATTLE_TESTS:-0}" != "1" ]; then
  echo "→ SPEC-042 full battle test suite..."
  chmod +x "$ROOT/specs/042-update-age-pgvector/e2e/run_all_battle_tests.sh"
  SKIP_IMAGE_BUILD="${SKIP_IMAGE_BUILD:-1}" "$ROOT/specs/042-update-age-pgvector/e2e/run_all_battle_tests.sh"
fi

echo ""
echo "✓ E2E proof complete ($EQ_POSTGRES_PROFILE) — artifacts:"
ls -la "$HEALTH_JSON" "$HEALTH_HTML" 2>/dev/null || true
ls -la "$DASH_SHOT" "$DOCS_SHOT" "$PHASE_E_SHOT" 2>/dev/null || true
