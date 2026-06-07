#!/usr/bin/env bash
# SPEC-017 Playwright visual proof for edgequake-api routes (query + documents + health)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../../.." && pwd)"
WEBUI="$ROOT/edgequake_webui"
SCREENSHOTS="$(dirname "$0")/screenshots"
BACKEND_PORT="${BACKEND_PORT:-8081}"
FRONTEND_PORT="${FRONTEND_PORT:-3001}"

backend_healthy() {
  local body
  body="$(curl -sf --max-time 3 "$1/health" 2>/dev/null || curl -sf --max-time 3 "$1/api/v1/health" 2>/dev/null || true)"
  echo "$body" | grep -q '"status"[[:space:]]*:[[:space:]]*"healthy"' \
    && echo "$body" | grep -q '"storage_mode"'
}

resolve_backend_url() {
  local port="${BACKEND_PORT}"
  if [[ -f /tmp/edgequake-start.sh ]]; then
    local from_script
    from_script="$(grep '^export PORT=' /tmp/edgequake-start.sh | sed -E 's/^export PORT="?([^"]+)"?/\1/')"
    if [[ -n "${from_script:-}" ]]; then
      port="$from_script"
    fi
  fi
  local url="http://127.0.0.1:${port}"
  if backend_healthy "$url"; then
    echo "$url"
    return 0
  fi
  local fallback="http://127.0.0.1:${BACKEND_PORT}"
  if [[ "$fallback" != "$url" ]] && backend_healthy "$fallback"; then
    echo "$fallback"
    return 0
  fi
  echo "$url"
}

API_URL="${EQ_BACKEND_URL:-$(resolve_backend_url)}"

if ! backend_healthy "$API_URL"; then
  echo "→ Starting backend (nohup for agent-shell reliability)"
  (cd "$ROOT" && make backend-bg BACKEND_PORT="${API_URL##*:}" FRONTEND_PORT="$FRONTEND_PORT" --no-print-directory)
  if [[ -f /tmp/edgequake-start.sh ]]; then
    API_URL="$(resolve_backend_url)"
    nohup bash /tmp/edgequake-start.sh >> /tmp/edgequake-backend-agent.log 2>&1 &
  fi
fi

mkdir -p "$SCREENSHOTS"

edgequake_ui_port() {
  for p in "$FRONTEND_PORT" 3001 3000; do
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
UI_URL="http://localhost:${FP}"

for _ in $(seq 1 60); do
  backend_healthy "$API_URL" && break
  sleep 2
done

HEALTH="$(curl -sf --max-time 5 "${API_URL}/health")"
echo "$HEALTH" | tee "${SCREENSHOTS}/../002-health-response.json"
backend_healthy "$API_URL" || {
  echo "Backend health check failed (expected healthy + storage_mode): $HEALTH"
  exit 1
}

(cd "$WEBUI" && E2E_LIVE_STACK=1 EQ_BACKEND_URL="$API_URL" PLAYWRIGHT_BASE_URL="$UI_URL" \
  bunx playwright test e2e/spec017-api-query-documents.spec.ts --project=audit --workers=1)

ls -la "$SCREENSHOTS"/*.png 2>/dev/null || true
echo "✓ Playwright API proof complete"
