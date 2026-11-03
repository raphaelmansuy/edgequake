#!/bin/bash
# SPEC-182 capture run: start the stack, capture the screenshots, stop the stack.
#
# WHY one script: this environment reaps detached processes between tool calls, so a
# "start in one step, test in the next" workflow finds dead ports. Everything the capture
# needs is therefore started, used, and torn down inside a single invocation.
#
# Requires: specs/182-rags/api/scope.env (written by e2e/run_demo.py) with the tenant and
# workspace that already hold the 20-picture corpus.
set -uo pipefail

REPO=/Users/raphaelmansuy/Github/03-working/edgequake
SPEC=$REPO/specs/182-rags
# WHY logs inside the repo: this environment clears /tmp between tool calls, which
# also took the script itself once. Anything the run needs to outlive must live here.
LOGS=$SPEC/api/logs
mkdir -p "$LOGS"
BACKEND_LOG=$LOGS/backend.log
UI_LOG=$LOGS/ui.log

cleanup() {
  echo "--- stopping stack"
  [ -n "${UI_PID:-}" ] && kill "$UI_PID" 2>/dev/null
  [ -n "${BE_PID:-}" ] && kill "$BE_PID" 2>/dev/null
  sleep 1
}
trap cleanup EXIT

wait_for() { # wait_for <url> <label> <seconds>
  local url=$1 label=$2 secs=$3 i=0
  while [ $i -lt "$secs" ]; do
    if curl -sf -m 3 -o /dev/null "$url"; then echo "  $label ready after ${i}s"; return 0; fi
    sleep 1; i=$((i+1))
  done
  echo "  $label DID NOT START in ${secs}s"; return 1
}

echo "=== 1. backend"
lsof -ti:8090 | xargs kill -9 2>/dev/null
sleep 1
( cd "$REPO/edgequake" && exec "$SPEC/e2e/run-backend.sh" ) > "$BACKEND_LOG" 2>&1 &
BE_PID=$!
wait_for http://127.0.0.1:8090/live backend 90 || { tail -25 "$BACKEND_LOG"; exit 1; }
# /live answers before the deep document is servable; wait for the real one too.
wait_for http://127.0.0.1:8090/health backend-deep 90 || { tail -25 "$BACKEND_LOG"; exit 1; }

echo "=== 2. frontend"
lsof -ti:3010 | xargs kill -9 2>/dev/null
rm -rf "$REPO/edgequake_webui/.next/dev/lock"
sleep 1
( cd "$REPO/edgequake_webui" && EDGEQUAKE_API_URL=http://127.0.0.1:8090 \
    NEXT_PUBLIC_API_URL=http://127.0.0.1:8090 NODE_ENV=development \
    exec ./node_modules/.bin/next dev -p 3010 ) > "$UI_LOG" 2>&1 &
UI_PID=$!
wait_for http://127.0.0.1:3010/ ui 120 || { tail -30 "$UI_LOG"; exit 1; }

# The proxy resolves its backend at boot from a health probe; prove it points at :8090.
TID=$(grep '^SPEC182_TENANT_ID=' "$SPEC/api/scope.env" | cut -d= -f2)
printf '  proxy -> '
curl -s -m 60 -o /dev/null -w '%{http_code}\n' \
  "http://127.0.0.1:3010/api/v1/tenants/$TID/workspaces" \
  -H "X-User-ID: 00000000-0000-0000-0000-000000000001"

echo "=== 3. warm the chat model (the health probe loads it; a cold load trips the"
echo "        'No LLM provider is reachable' banner and would appear in a screenshot)"
# WHY a cap: a 9 GB local model can take a minute to load. Warm-up is best-effort — the
# banner check in the spec fails the capture with an actionable message if it did not take.
curl -s -m 120 -X POST http://127.0.0.1:11434/api/chat \
  -d '{"model":"gemma4:latest","stream":false,"messages":[{"role":"user","content":"hi"}]}' \
  -o /dev/null -w '  warm-up: %{http_code} in %{time_total}s\n' || echo "  warm-up timed out (continuing)"
curl -s -m 30 http://127.0.0.1:8090/health \
  | python3 -c 'import sys,json;d=json.load(sys.stdin);print("  llm_provider:",d["components"]["llm_provider"])' \
  || echo "  health unreadable"

echo "=== 4. capture"
cd "$REPO/edgequake_webui" || exit 1
set -a; . "$SPEC/api/scope.env"; set +a
SPEC182_ROUND=${SPEC182_ROUND:-round-01} \
PLAYWRIGHT_BASE_URL=http://127.0.0.1:3010 \
  npx playwright test e2e/spec182/picture-search.spec.ts e2e/spec182/image-query.spec.ts \
  --project=chromium --reporter=line
CAPTURE_RC=$?

echo "=== 5. results"
ls -la "$SPEC/screenshots/${SPEC182_ROUND:-round-01}" 2>/dev/null
echo "capture exit: $CAPTURE_RC"
exit $CAPTURE_RC
