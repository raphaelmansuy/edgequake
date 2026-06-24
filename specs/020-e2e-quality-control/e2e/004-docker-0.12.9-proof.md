# SPEC-020 — Docker GHCR v0.12.9 E2E Proof

**Date:** 2026-06-08  
**Images:** `ghcr.io/raphaelmansuy/edgequake:0.12.9` (+ frontend, postgres)  
**CD:** [Release workflow run](https://github.com/raphaelmansuy/edgequake/actions/runs/27136040396) — success  
**Stack:** `docker-compose.quickstart.yml` with `EDGEQUAKE_LLM_PROVIDER=mock`

## Stack verification

| Check | Result |
|-------|--------|
| API health | `status: healthy`, `version: 0.12.9` |
| `/ready` | 200 |
| migration-038 | ready (via health schema) |
| Frontend runtime | `apiUrl: http://localhost:18080` |
| Docker Ollama rewrite | `OLLAMA_HOST` `localhost:11434` → `host.docker.internal:11434` at startup ✅ |
| Audit enum | `Wrote audit event` … `event_type=WorkspaceAccess` (PascalCase) ✅ |

```bash
EDGEQUAKE_VERSION=0.12.9 EDGEQUAKE_PORT=18080 FRONTEND_PORT=13000 \
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
| Failed | 3 (Ollama sync timeout: tests 10, 19, 22) |
| Skipped | 0 |

**Mock-path coverage (release-grade):** health, routes, ingest, query, PDF, isolation, citations, streaming, 404, delete cascade, graph search edge case, UI uploads — all green on Docker v0.12.9.

## v0.12.8 → v0.12.9 delta (honest)

| Issue | v0.12.8 Docker | v0.12.9 Docker |
|-------|----------------|----------------|
| Ollama networking | 500 `PIPELINE_ERROR` — `localhost:11434` inside container | ✅ Ollama reached via `host.docker.internal` |
| Audit inserts | Silent enum failures (`documentupload`) | ✅ PascalCase writes (`WorkspaceAccess`) |
| Ollama E2E tests 10/19/22 | Failed (connection) | Failed (408 sync timeout @ 120s) |

## Failures (remaining)

| Test | Root cause |
|------|------------|
| 10, 19, 22 | Sync markdown upload uses `SYNC_PROCESSING_TIMEOUT_SECS=120`. Ollama `gemma4:latest` extraction (~84s) + `embeddinggemma:latest` (~35s) exceeds 120s → HTTP **408**. Not a networking bug. |

Evidence (API log):

```text
Ollama chat request: 1 messages to model gemma4:latest   # ~12:42:56
Ollama embedding request: 1 texts ... embeddinggemma    # ~12:44:19
status=408 duration_ms=120257
```

Re-test with `SPEC020_OLLAMA_MODEL=qwen3.5:latest` — same 408 (local Ollama still >120s end-to-end in Docker).

## Grade: **A** for Docker mock release path; **B+** for full live-Ollama Docker E2E

- **Ship-ready:** mock/default Docker quickstart on GHCR `0.12.9`
- **Follow-up (v0.12.10+):** raise sync timeout for local/Ollama providers or use `async_processing: true` in Ollama SPEC-020 helpers
