#!/usr/bin/env bash
# SPEC-019 — Option 1 upload + query E2E (API + UI screenshots)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
E2E_DIR="$(dirname "$0")"
SCREENSHOTS="$E2E_DIR/screenshots"
LOG="$E2E_DIR/011-upload-query-run.log"

API_URL="${API_URL:-http://127.0.0.1:${EDGEQUAKE_PORT:-18080}}"
UI_URL="${UI_URL:-http://127.0.0.1:${FRONTEND_PORT:-13000}}"
TENANT_ID="${TENANT_ID:-00000000-0000-0000-0000-000000000002}"
WORKSPACE_ID="${WORKSPACE_ID:-00000000-0000-0000-0000-000000000003}"

mkdir -p "$SCREENSHOTS"
: >"$LOG"
log() { echo "[$(date -u +%H:%M:%S)] $*" | tee -a "$LOG"; }

api_headers() {
  printf '%s\n' \
    "Content-Type: application/json" \
    "X-Tenant-ID: ${TENANT_ID}" \
    "X-Workspace-ID: ${WORKSPACE_ID}"
}

log "Upload + query proof — API=$API_URL UI=$UI_URL"

curl -sf "${API_URL}/health" | grep -q '"status"[[:space:]]*:[[:space:]]*"healthy"' \
  || { log "FAIL: stack not healthy at $API_URL"; exit 1; }

DOC='SPEC-019 Option 1 E2E upload and query proof.
Sarah Chen is a senior engineer at EDGEQUAKE building GraphRAG features.
Michael Torres leads LLM integration for entity extraction pipelines.
Release v0.12.7 control test validates Docker quickstart end-to-end.'
TITLE="spec019-option1-upload-$(date +%s).md"

log "Uploading document (sync, Ollama pipeline)"
UPLOAD_JSON="$E2E_DIR/009-upload-response.json"
PAYLOAD="$E2E_DIR/.upload-payload.json"
python3 - "$TITLE" "$DOC" >"$PAYLOAD" <<'PY'
import json, sys
print(json.dumps({
    "title": sys.argv[1],
    "content": sys.argv[2],
    "async_processing": False,
}))
PY
curl -s -X POST "${API_URL}/api/v1/documents" \
  -H "Content-Type: application/json" \
  -H "X-Tenant-ID: ${TENANT_ID}" \
  -H "X-Workspace-ID: ${WORKSPACE_ID}" \
  -d @"$PAYLOAD" \
  --max-time 600 | tee "$UPLOAD_JSON" >>"$LOG"
rm -f "$PAYLOAD"

python3 - <<'PY' "$UPLOAD_JSON"
import json, sys
d = json.load(open(sys.argv[1]))
assert d.get("chunk_count", 0) > 0, d
assert d.get("entity_count", 0) > 0, d
assert d.get("status") in ("processed", "completed"), d
print("upload OK:", d.get("document_id"), "entities=", d.get("entity_count"))
PY

log "Running hybrid query"
QUERY_JSON="$E2E_DIR/010-query-response.json"
curl -s -X POST "${API_URL}/api/v1/query" \
  -H "Content-Type: application/json" \
  -H "X-Tenant-ID: ${TENANT_ID}" \
  -H "X-Workspace-ID: ${WORKSPACE_ID}" \
  -d '{"query":"Who is Sarah Chen at EDGEQUAKE and what is her role?","mode":"hybrid"}' \
  --max-time 300 | tee "$QUERY_JSON" >>"$LOG"

python3 - <<'PY' "$QUERY_JSON"
import json, sys
d = json.load(open(sys.argv[1]))
sources = d.get("sources") or []
answer = (d.get("answer") or "").lower()
assert len(sources) > 0, d
assert "sarah" in answer or "chen" in answer, d
print("query OK: sources=", len(sources), "answer_len=", len(d.get("answer","")))
PY

WEBUI="$ROOT/edgequake_webui"
if command -v bunx >/dev/null 2>&1; then
  log "Capturing post-ingestion UI screenshots"
  (
    cd "$WEBUI"
    bunx playwright screenshot --wait-for-timeout 4000 "${UI_URL}/documents" \
      "$SCREENSHOTS/05-documents-after-upload.png"
    bunx playwright screenshot --wait-for-timeout 4000 "${UI_URL}/query" \
      "$SCREENSHOTS/06-query-page-ready.png"
  ) >>"$LOG" 2>&1 || log "WARN: static UI screenshots failed"
fi

log "PASS: upload + query proof complete"
