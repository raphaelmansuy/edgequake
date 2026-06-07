#!/usr/bin/env bash
# SPEC-017 Playwright proof for edgequake-webui DRY/SOLID remediation
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../../.." && pwd)"
WEBUI="$ROOT/edgequake_webui"
SCREENSHOTS="$(dirname "$0")/screenshots"
BACKEND_PORT="${BACKEND_PORT:-8081}"
FRONTEND_PORT="${FRONTEND_PORT:-3001}"

backend_healthy() {
  local body
  body="$(curl -sf --max-time 3 "$1/health" 2>/dev/null || true)"
  echo "$body" | grep -q '"status"[[:space:]]*:[[:space:]]*"healthy"' \
    && echo "$body" | grep -q '"storage_mode"'
}

resolve_backend_url() {
  for port in "$BACKEND_PORT" 8081 8080; do
    local url="http://127.0.0.1:${port}"
    if backend_healthy "$url"; then
      echo "$url"
      return 0
    fi
  done
  echo "http://127.0.0.1:${BACKEND_PORT}"
}

API_URL="${EQ_BACKEND_URL:-$(resolve_backend_url)}"

if ! backend_healthy "$API_URL"; then
  echo "→ Starting backend on port ${BACKEND_PORT}"
  (cd "$ROOT" && make backend-bg BACKEND_PORT="$BACKEND_PORT" FRONTEND_PORT="$FRONTEND_PORT" --no-print-directory)
  if [[ -f /tmp/edgequake-start.sh ]]; then
    nohup bash /tmp/edgequake-start.sh >> /tmp/edgequake-backend-agent.log 2>&1 &
  fi
  API_URL="$(resolve_backend_url)"
fi

for _ in $(seq 1 60); do
  API_URL="$(resolve_backend_url)"
  backend_healthy "$API_URL" && break
  sleep 2
done
backend_healthy "$API_URL" || { echo "Backend failed to start (tried 8081/8080)"; exit 1; }
echo "Using backend: $API_URL"

mkdir -p "$SCREENSHOTS"

edgequake_ui_port() {
  for p in "$FRONTEND_PORT" 3000 3001; do
    if curl -sf --max-time 3 "http://localhost:${p}/" 2>/dev/null | grep -qi EdgeQuake; then
      echo "$p"
      return 0
    fi
  done
  return 1
}

if ! edgequake_ui_port >/dev/null 2>&1; then
  echo "→ Starting frontend"
  (cd "$ROOT" && make frontend-bg BACKEND_PORT="${API_URL##*:}" FRONTEND_PORT="$FRONTEND_PORT" --no-print-directory)
  if [[ -f /tmp/edgequake-frontend-start.sh ]]; then
    nohup bash /tmp/edgequake-frontend-start.sh >> /tmp/edgequake-frontend-agent.log 2>&1 &
  fi
fi

FP=""
for _ in $(seq 1 30); do
  if FP="$(edgequake_ui_port)"; then break; fi
  sleep 2
done
FP="${FP:-$FRONTEND_PORT}"
UI_URL="http://127.0.0.1:${FP}"

for _ in $(seq 1 60); do
  backend_healthy "$API_URL" && break
  sleep 2
done

HEALTH="$(curl -sf --max-time 5 "${API_URL}/health")"
echo "$HEALTH" | tee "$(dirname "$0")/002-health-response.json"
backend_healthy "$API_URL" || {
  echo "Backend health check failed: $HEALTH"
  exit 1
}

echo "→ Route smoke + live stack (Playwright starts Next on 3001 with EDGEQUAKE_API_URL)"
# Keep backend alive for full run (restart if it died during prior attempts)
if ! backend_healthy "$API_URL" && [[ -f /tmp/edgequake-start.sh ]]; then
  nohup bash /tmp/edgequake-start.sh >> /tmp/edgequake-backend-agent.log 2>&1 &
  sleep 15
fi
(cd "$WEBUI" && PLAYWRIGHT_SKIP_STACK_CHECK=1 E2E_LIVE_STACK=1 EQ_BACKEND_URL="$API_URL" \
  bunx playwright test e2e/spec017-webui-dry-solid.spec.ts e2e/spec017-barrel-smoke.spec.ts \
  --project=chromium --workers=1)

ls -la "$SCREENSHOTS"/*.png 2>/dev/null || true
echo "✓ Playwright webui proof complete"
