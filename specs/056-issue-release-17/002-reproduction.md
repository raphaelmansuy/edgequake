# Reproduction — Published Docker v0.17.0 + Mistral

## Environment

- Host: macOS (Apple Silicon), Docker via OrbStack
- Isolated from local `make dev` (ports **18080** / **13000**, project `issue300`)
- Images (exact):
  - `ghcr.io/raphaelmansuy/edgequake:0.17.0`
  - `ghcr.io/raphaelmansuy/edgequake-frontend:0.17.0`
  - `ghcr.io/raphaelmansuy/edgequake-postgres:0.17.0`
- LLM: Mistral La Plateforme
  - Chat / vision: `mistral-small-latest` ([Mistral Small 4 multimodal](https://mistral.ai/news/mistral-small-4/))
  - Embeddings: `mistral-embed` (1024-d)

## Bring-up

```bash
# From a copy of docker-compose.quickstart.yml with unique container_name / volume
# (avoid clashing with local edgequake-postgres on :5432 / :8080)

export EDGEQUAKE_VERSION=0.17.0
export EDGEQUAKE_POSTGRES_TAG=0.17.0
export EDGEQUAKE_PORT=18080
export FRONTEND_PORT=13000
export EDGEQUAKE_API_URL=http://localhost:18080
export EDGEQUAKE_LLM_PROVIDER=mistral
export EDGEQUAKE_LLM_MODEL=mistral-small-latest
export EDGEQUAKE_EMBEDDING_PROVIDER=mistral
export EDGEQUAKE_EMBEDDING_MODEL=mistral-embed
export EDGEQUAKE_VISION_PROVIDER=mistral
export EDGEQUAKE_VISION_MODEL=mistral-small-latest
export EDGEQUAKE_AUTH_ENABLED=false
export EDGEQUAKE_DEV_MODE=true
# MISTRAL_API_KEY must be set

COMPOSE_PROJECT_NAME=issue300 docker compose up -d
curl -s http://localhost:18080/health | jq '.version, .providers'
# → "0.17.0", llm/embedding = mistral
```

## Steps

### A. Create workspace (vision default)

```bash
TENANT=00000000-0000-0000-0000-000000000002
curl -s -X POST "http://localhost:18080/api/v1/tenants/$TENANT/workspaces" \
  -H 'Content-Type: application/json' \
  -d '{
    "name":"issue300-repro",
    "llm_provider":"mistral","llm_model":"mistral-small-latest",
    "embedding_provider":"mistral","embedding_model":"mistral-embed",
    "vision_llm_provider":"mistral","vision_llm_model":"mistral-small-latest",
    "pdf_parser_backend":"vision"
  }'
```

### B. Wrong endpoint (immediate failure — not infinite load)

```bash
curl -s -X POST http://localhost:18080/api/v1/documents/upload \
  -H "X-Tenant-ID: $TENANT" -H "X-Workspace-ID: $WS" \
  -F file=@sample.pdf
# 400 Unsupported file type: .pdf
```

### C. Correct PDF endpoint without client track_id

```bash
curl -s -X POST http://localhost:18080/api/v1/documents/pdf \
  -H "X-Tenant-ID: $TENANT" -H "X-Workspace-ID: $WS" \
  -F file=@embedded_figure_sample.pdf \
  -F pdf_parser_backend=vision \
  -F vision_model=mistral-small-latest \
  -F vision_provider=mistral
# 200 status=queued, track_id=null, task_id=pdf-…
# Document later completes under task_id
```

### D. UI-like upload (client track_id) — reproduces “stuck progress”

```bash
TRACK=ui_track_$(date +%s)
curl -s -X POST http://localhost:18080/api/v1/documents/pdf \
  -H "X-Tenant-ID: $TENANT" -H "X-Workspace-ID: $WS" \
  -F file=@national-capitals.pdf \
  -F pdf_parser_backend=vision \
  -F enable_vision=true \
  -F track_id=$TRACK \
  -F vision_model=mistral-small-latest \
  -F vision_provider=mistral

curl -s http://localhost:18080/api/v1/documents/pdf/progress/$TRACK | jq .overall_percentage
# → 0 forever while conversion runs

curl -s http://localhost:18080/api/v1/documents/pdf/progress/<task_id from response> | jq .overall_percentage
# → increases (page N of M)
```

### E. Text baseline

`POST /documents/upload` with `.txt` returned `202` with a usable `track_id` and completed normally.

### F. WebUI smoke

Opened `http://localhost:13000/documents?workspace=issue300-repro`:

- Runtime config: `apiUrl=http://localhost:18080`
- Document list showed live stage text (`Extracting Entities · N/410 chunks`) from **document metadata** (server track)
- This confirms backend work proceeds; the broken surface is **progress keyed by client track_id**

## Observed outcomes

| Path | Result |
|------|--------|
| Text upload | OK |
| PDF vision upload (Mistral) | Backend OK → `completed` |
| Client `track_id` progress API | Stuck at 0% / Waiting for Upload |
| Server `task_id` progress API | Advances correctly |
| WebUI document table | Shows processing via metadata (not client progress key) |

## Artifacts

See [`artifacts/`](./artifacts/) for health JSON, upload bodies, and dual progress snapshots captured during the run.
