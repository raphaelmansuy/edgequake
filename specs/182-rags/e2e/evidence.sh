#!/bin/bash
# SPEC-182 API evidence run: start the backend, seed the corpus, verify retrieval, stop.
#
# WHY bundled like capture.sh: this environment reaps detached processes between tool
# calls, so "start the server, then run the checks" leaves a dead port by step two.
set -uo pipefail

REPO=/Users/raphaelmansuy/Github/03-working/edgequake
SPEC=$REPO/specs/182-rags
LOGS=$SPEC/api/logs
mkdir -p "$LOGS"
BACKEND_LOG=$LOGS/backend-demo.log

cleanup() {
  echo "--- stopping backend"
  [ -n "${BE_PID:-}" ] && kill "$BE_PID" 2>/dev/null
  sleep 1
}
trap cleanup EXIT

wait_for() { # wait_for <url> <label> <seconds>
  local i=0
  while [ $i -lt "$3" ]; do
    if curl -sf -m 3 -o /dev/null "$1"; then echo "  $2 ready after ${i}s"; return 0; fi
    sleep 1; i=$((i+1))
  done
  echo "  $2 DID NOT START in ${3}s"; return 1
}

echo "=== 1. database"
docker start edgequake-postgres > /dev/null 2>&1 || true
# A restarted container needs a moment before it accepts connections.
for i in $(seq 1 30); do
  if PGPASSWORD=edgequake_secret psql -h localhost -U edgequake -d edgequake -tAc 'select 1' > /dev/null 2>&1; then
    echo "  postgres ready after ${i}s"; break
  fi
  sleep 1
done

echo "=== 2. backend"
lsof -ti:8090 | xargs kill -9 2>/dev/null
sleep 1
( cd "$REPO/edgequake" && exec "$SPEC/e2e/run-backend.sh" ) > "$BACKEND_LOG" 2>&1 &
BE_PID=$!
wait_for http://127.0.0.1:8090/live backend 90 || { tail -25 "$BACKEND_LOG"; exit 1; }
wait_for http://127.0.0.1:8090/health backend-deep 90 || { tail -25 "$BACKEND_LOG"; exit 1; }

echo "=== 3. probe check (needs no server; reads the cached vectors)"
cd "$SPEC/e2e" && python3 check_probes.py
PROBE_RC=$?

echo "=== 4. corpus + retrieval (24 checks, real embeddings, real HTTP)"
python3 run_demo.py --base http://127.0.0.1:8090 2>&1 | tail -12
DEMO_RC=${PIPESTATUS[0]}

echo "=== 5. summary"
echo "probes exit: $PROBE_RC  |  demo exit: $DEMO_RC"
[ "$PROBE_RC" -eq 0 ] && [ "$DEMO_RC" -eq 0 ]
