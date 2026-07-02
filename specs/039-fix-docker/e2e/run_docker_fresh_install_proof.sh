#!/usr/bin/env bash
# SPEC-039: Fresh Docker install E2E proof (ingest + query).
#
# Usage:
#   MISTRAL_API_KEY=... ./run_docker_fresh_install_proof.sh mistral
#   OLLAMA_MODEL=gemma4:e4b ./run_docker_fresh_install_proof.sh ollama
#
# Requires: docker, curl, python3
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
MODE="${1:-mistral}"
PROJECT="edgequake-spec039-e2e"
API_PORT="${EDGEQUAKE_PORT:-18081}"
UI_PORT="${FRONTEND_PORT:-13001}"
COMPOSE_FILE="${COMPOSE_FILE:-$ROOT/docker-compose.quickstart.yml}"

export EDGEQUAKE_VERSION="${EDGEQUAKE_VERSION:-0.13.1}"
export EDGEQUAKE_PORT="$API_PORT"
export FRONTEND_PORT="$UI_PORT"
export EDGEQUAKE_API_URL="http://localhost:${API_PORT}"

case "$MODE" in
  mistral)
    [[ -n "${MISTRAL_API_KEY:-}" ]] || { echo "MISTRAL_API_KEY required"; exit 1; }
    export EDGEQUAKE_LLM_PROVIDER=mistral
    export EDGEQUAKE_LLM_MODEL=mistral-small-latest
    export EDGEQUAKE_EMBEDDING_PROVIDER=mistral
    export MISTRAL_EMBEDDING_MODEL=mistral-embed
    export EDGEQUAKE_VISION_PROVIDER=mistral
    export EDGEQUAKE_VISION_MODEL=pixtral-large-latest
    ;;
  ollama)
    export EDGEQUAKE_LLM_PROVIDER=ollama
    export EDGEQUAKE_LLM_MODEL="${OLLAMA_MODEL:-gemma4:e4b}"
    export EDGEQUAKE_EMBEDDING_PROVIDER=ollama
    export OLLAMA_EMBEDDING_MODEL="${OLLAMA_EMBEDDING_MODEL:-embeddinggemma}"
    export EDGEQUAKE_VISION_PROVIDER=ollama
    export EDGEQUAKE_VISION_MODEL="${OLLAMA_MODEL:-gemma4:e4b}"
    curl -sf "${OLLAMA_HOST:-http://localhost:11434}/api/tags" >/dev/null || {
      echo "Ollama not reachable at ${OLLAMA_HOST:-http://localhost:11434}"; exit 1;
    }
    ;;
  *)
    echo "Usage: $0 mistral|ollama"; exit 1;;
esac

stop_conflicting_containers() {
  for c in edgequake-api edgequake-frontend edgequake-postgres; do
    if docker ps -a --format '{{.Names}}' | grep -qx "$c"; then
      docker stop "$c" >/dev/null 2>&1 || true
      docker rm "$c" >/dev/null 2>&1 || true
    fi
  done
}

echo "== SPEC-039: tear down prior project ($PROJECT) =="
docker compose -p "$PROJECT" -f "$COMPOSE_FILE" down -v --remove-orphans 2>/dev/null || true
stop_conflicting_containers

echo "== SPEC-039: start stack ($MODE, v$EDGEQUAKE_VERSION) =="
docker compose -p "$PROJECT" -f "$COMPOSE_FILE" up -d

echo "== SPEC-039: wait for API health =="
for i in $(seq 1 60); do
  if curl -sf "http://localhost:${API_PORT}/health" | python3 -c "import json,sys; d=json.load(sys.stdin); exit(0 if d.get('status')=='healthy' else 1)" 2>/dev/null; then
    break
  fi
  sleep 3
done
curl -sf "http://localhost:${API_PORT}/health" | python3 -m json.tool | head -20

echo "== SPEC-039: register + login (auth enabled in Docker) =="
USER="spec039-$(date +%s)"
PASS='TestPass123!'
curl -sf -X POST "http://localhost:${API_PORT}/api/v1/users" \
  -H 'Content-Type: application/json' \
  -d "{\"username\":\"$USER\",\"email\":\"$USER@test.local\",\"password\":\"$PASS\"}" >/dev/null
LOGIN_JSON=$(curl -sf -X POST "http://localhost:${API_PORT}/api/v1/auth/login" \
  -H 'Content-Type: application/json' \
  -d "{\"username\":\"$USER\",\"password\":\"$PASS\"}")
TOKEN=$(echo "$LOGIN_JSON" | python3 -c "import json,sys; print(json.load(sys.stdin)['access_token'])")
TENANT_ID=$(echo "$LOGIN_JSON" | python3 -c "import base64,json,sys; t=json.load(sys.stdin)['access_token'].split('.')[1]; t+=('='*(-len(t)%4)); print(json.loads(base64.urlsafe_b64decode(t))['tenant_id'])")
WORKSPACE_ID=$(echo "$LOGIN_JSON" | python3 -c "import base64,json,sys; t=json.load(sys.stdin)['access_token'].split('.')[1]; t+=('='*(-len(t)%4)); print(json.loads(base64.urlsafe_b64decode(t))['workspace_id'])")

echo "== SPEC-039: upload document =="
DOC=$(curl -sf -X POST "http://localhost:${API_PORT}/api/v1/documents" \
  -H "Authorization: Bearer $TOKEN" \
  -H "X-Tenant-ID: $TENANT_ID" \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  -H 'Content-Type: application/json' \
  -d '{"content":"# SPEC-039\n\nDr. Sarah Chen leads EdgeQuake in Zurich. Founded 2024.\n","title":"SPEC-039 proof","async_processing":true}' \
  | python3 -c "import json,sys; print(json.load(sys.stdin)['document_id'])")

for i in $(seq 1 48); do
  STATUS=$(curl -s "http://localhost:${API_PORT}/api/v1/documents/$DOC" \
    -H "Authorization: Bearer $TOKEN" \
    -H "X-Tenant-ID: $TENANT_ID" \
    -H "X-Workspace-ID: $WORKSPACE_ID" \
    | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('status','?'))" 2>/dev/null || echo "?")
  echo "  poll $i: $STATUS"
  [[ "$STATUS" == "completed" ]] && break
  [[ "$STATUS" == "failed" ]] && {
    curl -sf "http://localhost:${API_PORT}/api/v1/documents/$DOC" \
      -H "Authorization: Bearer $TOKEN" \
      -H "X-Tenant-ID: $TENANT_ID" -H "X-Workspace-ID: $WORKSPACE_ID" | python3 -m json.tool
    exit 1
  }
  sleep 5
done
[[ "$STATUS" == "completed" ]] || { echo "Document did not complete in time"; exit 1; }

echo "== SPEC-039: query =="
ANSWER=$(curl -sf -X POST "http://localhost:${API_PORT}/api/v1/query" \
  -H "Authorization: Bearer $TOKEN" \
  -H "X-Tenant-ID: $TENANT_ID" \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  -H 'Content-Type: application/json' \
  -d '{"query":"Who leads EdgeQuake in Zurich?","mode":"mix"}' \
  | python3 -c "import json,sys; print(json.load(sys.stdin).get('answer',''))")

echo "$ANSWER" | head -5
echo "$ANSWER" | rg -qi "sarah|chen" || { echo "Query answer missing expected entity"; exit 1; }

echo "✓ SPEC-039 fresh Docker E2E passed ($MODE)"
