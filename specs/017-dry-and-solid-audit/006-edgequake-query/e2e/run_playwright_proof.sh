#!/usr/bin/env bash
# SPEC-017 Playwright visual proof for query UI
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
WEBUI="$ROOT/edgequake_webui"
SCREENSHOTS="$(dirname "$0")/screenshots"
BACKEND_PORT="${BACKEND_PORT:-8081}"
FRONTEND_PORT="${FRONTEND_PORT:-3001}"
API_URL="${EQ_BACKEND_URL:-http://127.0.0.1:${BACKEND_PORT}}"

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

if ! curl -sf --max-time 3 "${API_URL}/health" >/dev/null 2>&1; then
  echo "→ Starting backend on :${BACKEND_PORT}"
  (cd "$ROOT" && make backend-bg BACKEND_PORT="$BACKEND_PORT" FRONTEND_PORT="$FRONTEND_PORT" --no-print-directory)
  if [[ -f /tmp/edgequake-start.sh ]]; then
    _PORT="$(grep '^export PORT=' /tmp/edgequake-start.sh | sed -E 's/^export PORT="?([^"]+)"?/\1/')"
    API_URL="http://127.0.0.1:${_PORT}"
  fi
fi

if ! edgequake_ui_port >/dev/null 2>&1; then
  echo "→ Starting frontend"
  (cd "$ROOT" && make frontend-bg BACKEND_PORT="$BACKEND_PORT" FRONTEND_PORT="$FRONTEND_PORT" --no-print-directory)
fi

FP=""
for _ in $(seq 1 30); do
  if FP="$(edgequake_ui_port)"; then break; fi
  sleep 2
done
FP="${FP:-$FRONTEND_PORT}"
UI_URL="http://localhost:${FP}"

for _ in $(seq 1 60); do
  curl -sf --max-time 3 "${API_URL}/health" >/dev/null 2>&1 && break
  sleep 2
done
curl -sf --max-time 3 "${API_URL}/health" >/dev/null

(cd "$WEBUI" && E2E_LIVE_STACK=1 EQ_BACKEND_URL="$API_URL" PLAYWRIGHT_BASE_URL="$UI_URL" \
  bunx playwright test e2e/spec017-query-pipeline.spec.ts --project=audit --workers=1)

ls -la "$SCREENSHOTS"/*.png 2>/dev/null || true
echo "✓ Playwright query proof complete"
