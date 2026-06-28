#!/usr/bin/env bash
# SPEC-019 — Option 1 install E2E proof (v0.12.7 control)
# Simulates README Quick Start Option 1 using published GitHub raw assets.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PROOF_DIR="$(cd "$(dirname "$0")/.." && pwd)"
E2E_DIR="$(dirname "$0")"
SCREENSHOTS="$E2E_DIR/screenshots"
LOG="$E2E_DIR/001-install-run.log"
WORKDIR="${WORKDIR:-/tmp/edgequake-spec019-option1}"

EDGEQUAKE_VERSION="${EDGEQUAKE_VERSION:-0.12.7}"
EDGEQUAKE_PORT="${EDGEQUAKE_PORT:-18080}"
FRONTEND_PORT="${FRONTEND_PORT:-13000}"
RAW_BASE="https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main"

mkdir -p "$SCREENSHOTS" "$WORKDIR"
: >"$LOG"

log() { echo "[$(date -u +%H:%M:%S)] $*" | tee -a "$LOG"; }

log "SPEC-019 Option 1 install proof — v${EDGEQUAKE_VERSION}"
log "Workdir: $WORKDIR"
log "Ports: API=${EDGEQUAKE_PORT} UI=${FRONTEND_PORT}"

# ── Step 0: stop conflicting quickstart containers ───────────────────────────
for c in edgequake-api edgequake-frontend edgequake-postgres; do
  if docker ps -a --format '{{.Names}}' | grep -qx "$c"; then
    log "Stopping existing container: $c"
    docker stop "$c" >>"$LOG" 2>&1 || true
    docker rm "$c" >>"$LOG" 2>&1 || true
  fi
done

# ── Step 1: download compose file (Option 1 asset) ─────────────────────────────
log "Downloading docker-compose.quickstart.yml from GitHub raw"
curl -fsSL "${RAW_BASE}/docker-compose.quickstart.yml" -o "$WORKDIR/docker-compose.quickstart.yml"

# ── Step 2: headless stack start (pinned v0.12.7, Ollama-only) ───────────────
# WHY: host OPENAI_API_KEY leaks into compose and breaks Ollama-default installs.
log "Pulling GHCR images and starting stack (Ollama provider, keys cleared)"
export EDGEQUAKE_VERSION EDGEQUAKE_PORT FRONTEND_PORT
export EDGEQUAKE_LLM_PROVIDER="${EDGEQUAKE_LLM_PROVIDER:-ollama}"
export EDGEQUAKE_EMBEDDING_PROVIDER="${EDGEQUAKE_EMBEDDING_PROVIDER:-ollama}"
export OPENAI_API_KEY="" ANTHROPIC_API_KEY="" MISTRAL_API_KEY="" GEMINI_API_KEY=""
cd "$WORKDIR"
docker compose -f docker-compose.quickstart.yml pull >>"$LOG" 2>&1
docker compose -f docker-compose.quickstart.yml up -d >>"$LOG" 2>&1

# ── Step 3: health poll (90s max, mirrors quickstart.sh) ─────────────────────
API_URL="http://127.0.0.1:${EDGEQUAKE_PORT}"
UI_URL="http://127.0.0.1:${FRONTEND_PORT}"
HEALTH_OK=0
for i in $(seq 1 45); do
  if curl -sf --max-time 5 "${API_URL}/health" | grep -q '"status"[[:space:]]*:[[:space:]]*"healthy"'; then
    HEALTH_OK=1
    break
  fi
  sleep 2
done

if [[ "$HEALTH_OK" -ne 1 ]]; then
  log "FAIL: API health not ready after 90s"
  docker compose -f docker-compose.quickstart.yml ps >>"$LOG" 2>&1 || true
  docker compose -f docker-compose.quickstart.yml logs --tail=40 api >>"$LOG" 2>&1 || true
  exit 1
fi

curl -sf "${API_URL}/health" | tee "$E2E_DIR/002-health-response.json" >>"$LOG"
log "API health OK"

# ── Step 4: verify image tags ─────────────────────────────────────────────────
docker inspect edgequake-api --format '{{.Config.Image}}' | tee "$E2E_DIR/003-api-image.txt" >>"$LOG"
docker inspect edgequake-frontend --format '{{.Config.Image}}' | tee "$E2E_DIR/004-frontend-image.txt" >>"$LOG"
docker inspect edgequake-postgres --format '{{.Config.Image}}' | tee "$E2E_DIR/005-postgres-image.txt" >>"$LOG"

grep -q ":${EDGEQUAKE_VERSION}" "$E2E_DIR/003-api-image.txt" || {
  log "FAIL: API image not pinned to ${EDGEQUAKE_VERSION}"
  exit 1
}

# ── Step 5: frontend reachability ─────────────────────────────────────────────
UI_OK=0
for i in $(seq 1 30); do
  if curl -sf --max-time 5 -o /dev/null -w "%{http_code}" "$UI_URL" | grep -qE '200|307'; then
    UI_OK=1
    break
  fi
  sleep 2
done
[[ "$UI_OK" -eq 1 ]] || { log "FAIL: frontend not reachable at $UI_URL"; exit 1; }
log "Frontend reachable at $UI_URL"

# ── Step 6: swagger + ready probes ───────────────────────────────────────────
curl -sf --max-time 5 -o /dev/null -w "%{http_code}" "${API_URL}/swagger-ui" | tee "$E2E_DIR/006-swagger-status.txt" >>"$LOG"
curl -sf --max-time 5 "${API_URL}/ready" | tee "$E2E_DIR/007-ready-response.json" >>"$LOG" || true

# ── Step 7: compose ps snapshot ───────────────────────────────────────────────
docker compose -f docker-compose.quickstart.yml ps | tee "$E2E_DIR/008-compose-ps.txt" >>"$LOG"

# ── Step 8: UI screenshots (Playwright CLI) ───────────────────────────────────
WEBUI="$ROOT/edgequake_webui"
if [[ -d "$WEBUI/node_modules/@playwright" ]] || command -v bunx >/dev/null 2>&1; then
  log "Capturing UI screenshots"
  (
    cd "$WEBUI"
    bunx playwright screenshot --wait-for-timeout 3000 "${UI_URL}/" "$SCREENSHOTS/01-home-dashboard.png"
    bunx playwright screenshot --wait-for-timeout 3000 "${UI_URL}/documents" "$SCREENSHOTS/02-documents-page.png"
    bunx playwright screenshot --wait-for-timeout 3000 "${UI_URL}/query" "$SCREENSHOTS/03-query-page.png"
    bunx playwright screenshot --wait-for-timeout 3000 "${API_URL}/swagger-ui" "$SCREENSHOTS/04-swagger-ui.png"
  ) >>"$LOG" 2>&1 || log "WARN: screenshot capture failed (stack still healthy)"
fi

# ── Step 9: upload + query proof (API + UI) ───────────────────────────────────
if [[ "${SKIP_UPLOAD_QUERY:-0}" != "1" ]]; then
  log "Running upload + query E2E"
  API_URL="$API_URL" UI_URL="$UI_URL" \
    bash "$E2E_DIR/run_upload_query_proof.sh" >>"$LOG" 2>&1 || {
      log "FAIL: upload/query proof failed"
      exit 1
    }
  if command -v bunx >/dev/null 2>&1; then
    (
      cd "$WEBUI"
      EQ_BACKEND_URL="$API_URL" PLAYWRIGHT_BASE_URL="$UI_URL" \
        bunx playwright test e2e/spec019-option1-upload-query.spec.ts --project=chromium --workers=1
    ) >>"$LOG" 2>&1 || log "WARN: Playwright UI query proof failed"
  fi
fi

log "PASS: Option 1 headless install proof complete"
