# Option 1 Install — E2E Proof (v0.12.7)

**Status:** ✅ PASS  
**Date:** 2026-06-07 02:01 UTC  
**Method:** Headless Option 1 (published `docker-compose.quickstart.yml` + GHCR `0.12.7`)  
**Runner:** `bash specs/019-0-12-7-control/e2e/run_option1_install_proof.sh`

## Command (reproduces README Option 1 assets)

```bash
EDGEQUAKE_VERSION=0.12.7 \
EDGEQUAKE_PORT=18080 \
FRONTEND_PORT=13000 \
bash specs/019-0-12-7-control/e2e/run_option1_install_proof.sh
```

Underlying install steps (no git clone):

1. `curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/docker-compose.quickstart.yml`
2. `docker compose pull && docker compose up -d` with `EDGEQUAKE_VERSION=0.12.7`
3. Poll `http://127.0.0.1:18080/health` until healthy (≤90s)

Interactive wizard (`curl …/quickstart.sh | sh`) uses the same compose file; validated separately that `quickstart.sh` downloads and passes `sh -n`.

## Results

| Step | Evidence | Result |
|------|----------|--------|
| Image pull | `001-install-run.log` | `edgequake`, `edgequake-frontend`, `edgequake-postgres` @ `0.12.7` |
| API health | `002-health-response.json` | `"version":"0.12.7"`, `"status":"healthy"`, `storage_mode: postgresql` |
| API image pin | `003-api-image.txt` | `ghcr.io/raphaelmansuy/edgequake:0.12.7` |
| Frontend image | `004-frontend-image.txt` | `ghcr.io/raphaelmansuy/edgequake-frontend:0.12.7` |
| Postgres image | `005-postgres-image.txt` | `ghcr.io/raphaelmansuy/edgequake-postgres:0.12.7` |
| Swagger | `006-swagger-status.txt` | HTTP 200 |
| Compose status | `008-compose-ps.txt` | api + postgres healthy; frontend up |
| UI smoke | screenshots `01`–`04` | Dashboard, Documents, Query, Swagger render |

### Health excerpt

```json
{
  "status": "healthy",
  "version": "0.12.7",
  "storage_mode": "postgresql",
  "llm_provider_name": "ollama",
  "schema": { "latest_version": 38, "migrations_applied": 37 }
}
```

## Screenshot analysis

### `01-home-dashboard.png`

- Header shows **v0.12.7** with green health dot.
- Default workspace bootstrapped; toast confirms workspace selection.
- Dashboard cards (Documents, Entities, Relationships, Entity Types) render at 0 — fresh install expected.

### `02-documents-page.png`

- Documents upload zone visible; empty state (0 documents) — consistent with new volume.

### `03-query-page.png`

- Query page with Hybrid mode selected; suggestion cards and history sidebar render.
- No connection-error overlay — runtime API URL wiring OK for custom ports (`18080`/`13000`).

### `04-swagger-ui.png`

- OpenAPI Swagger UI loads from API container on port `18080`.

## Notes

- Proof uses non-default host ports (`18080`/`13000`) to avoid collisions with local `make dev` stacks; compose file supports `EDGEQUAKE_PORT` / `FRONTEND_PORT`.
- Sidebar footer may show an older baked label (`v0.12.3`); **authoritative version** is API `/health` and header badge (`v0.12.7`).
- Default LLM provider is Ollama on host (`host.docker.internal:11434`); Ollama was running during proof.

## Upload + query (extended proof)

See [002-upload-query-proof.md](002-upload-query-proof.md) — sync ingest (12 entities) and hybrid query (27 sources) on the same Option 1 stack, with UI screenshots `05`–`07`.

## Verdict

**GO** — Option 1 install from [raphaelmansuy/edgequake](https://github.com/raphaelmansuy/edgequake) works E2E with published v0.12.7 Docker images, including document upload and query.
