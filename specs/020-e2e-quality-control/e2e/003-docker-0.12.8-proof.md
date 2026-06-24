# SPEC-020 — Docker GHCR v0.12.8 E2E Proof

**Date:** 2026-06-08  
**Images:** `ghcr.io/raphaelmansuy/edgequake:0.12.8` (+ frontend, postgres)  
**Stack:** `docker-compose.quickstart.yml` with `EDGEQUAKE_LLM_PROVIDER=mock`

## Stack verification

| Check | Result |
|-------|--------|
| API health | `status: healthy`, `version: 0.12.8` |
| `/ready` | 200 |
| `/live` | OK |
| migration-038 | `source_ids_indexes.ready: true` |
| Frontend runtime | `apiUrl: http://localhost:18080` (EDGEQUAKE_API_URL) |
| Containers | `edgequake-api`, `edgequake-frontend`, `edgequake-postgres` all healthy |

```bash
EDGEQUAKE_VERSION=0.12.8 EDGEQUAKE_PORT=18080 FRONTEND_PORT=13000 \
  EDGEQUAKE_LLM_PROVIDER=mock EDGEQUAKE_API_URL=http://localhost:18080 \
  docker compose -f docker-compose.quickstart.yml up -d
```

## Playwright SPEC-020 (against published images)

```bash
E2E_LIVE_STACK=1 PLAYWRIGHT_BASE_URL=http://localhost:13000 \
  EQ_BACKEND_URL=http://localhost:18080 SPEC020_STRICT_MIGRATION=1 \
  bunx playwright test e2e/spec020-quality-control.spec.ts --project=audit --workers=1
```

| Result | Count |
|--------|-------|
| **Passed** | **21** |
| Failed | 3 (Ollama-only: tests 10, 19, 22) |
| Skipped | 0 |

**Mock-path coverage (release-grade):** health, routes, ingest, query, PDF, isolation, citations, streaming, 404, delete cascade, graph search edge case, UI uploads — all green on Docker.

## Failures (honest)

| Test | Root cause |
|------|------------|
| 10, 19, 22 | Ollama workspace uploads hit `http://localhost:11434` **inside** the API container instead of `host.docker.internal:11434` → 500 `PIPELINE_ERROR` |

Repro:

```text
Network error: error sending request for url (http://localhost:11434/api/chat)
```

`OLLAMA_HOST` env is set for server default, but **per-workspace Ollama clients** still use container localhost.

## Secondary issue (non-blocking API)

Audit logger writes lowercase enum (`documentupload`) but Postgres `audit_event_type` expects PascalCase (`DocumentUpload`). API requests succeed; audit rows fail silently in logs.

## Grade: **A-** for Docker mock release path

Ship-ready for mock/default Docker quickstart. Ollama workspace path in Docker needs fix before claiming full live-LLM Docker E2E.
