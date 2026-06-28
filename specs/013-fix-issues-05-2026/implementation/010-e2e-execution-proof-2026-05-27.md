# SPEC-013 E2E execution proof (2026-05-27)

Executed on developer machine with PostgreSQL (`edgequake-postgres`), backend `:8090`, frontend `:3001`.

## Results summary

| Suite | Command | Result | Duration |
|-------|---------|--------|----------|
| GitHub issues API | `e2e_spec013_github_issues` | **8/8 pass** | ~5s |
| Mistral PDF + query | `e2e_spec013_mistral_pdf_query` | **6/6 pass** | ~61s |
| Vector stats DDL | `postgres_workspace_vector_stats` | **1/1 pass** | ~0.14s |
| Playwright UI | `playwright.spec013-ui.config.ts` | **6/6 pass** | ~4.6s |

**Total automated proof: 21/21 tests passed** (after harness fixes below).

## Harness fixes applied this session

1. **Worker pool in test AppState** — in-process tests now start `WorkerPool` (same as `main.rs`). Without this, PDF tasks stayed `pending` → false-negative failures.
2. **`wait_for_pdf_completed`** — tolerate `status=completed` before `document_id` is linked (markdown phase vs entity pipeline).
3. **Cancel E2E** — mock: cancel while processing; live (`SPEC013_LIVE_API_URL`): cancel completed → 409.
4. **Vector stats test** — accepts `DATABASE_URL` (not only `POSTGRES_PASSWORD`).
5. **UI proof** — requires correct frontend port (3001 here); Makefile checks page contains `edgequake`.

## Mistral metrics (golden path)

```
SPEC013_INGEST_ELAPSED_SECS=6
SPEC013_QUERY_ELAPSED_SECS=3
```

SLO gates used: `SPEC013_INGEST_SLO_SECS=900`, `SPEC013_QUERY_SLO_SECS=180`.

## Environment

- `MISTRAL_API_KEY`: set (from `.env`)
- `DATABASE_URL`: `postgresql://edgequake:...@localhost:5432/edgequake`
- Backend health: `http://localhost:8090/health` → `postgresql`
- Frontend: `http://localhost:3001` (not 3000 — other app occupied default port)

## Anti-flake repeat (2026-05-27 later)

| Run | Mistral 6/6 | Wall time |
|-----|-------------|-----------|
| 1 | pass | 58.97s |
| 2 | pass | 63.74s |
| 3 | pass | 56.76s |

Harness: per-test `AppState` + worker shutdown between `#[serial]` tests; `--test-threads=1`.

## Reproduce

```bash
set -a && source .env && set +a
# Do NOT set SPEC013_LIVE_API_URL while make dev-bg is running (dual worker pools).
make spec013-proof
BACKEND_PORT=8090 FRONTEND_PORT=3001 make spec013-proof-full
```

## Iteration 7 (2026-05-27 afternoon)

| Command | Result |
|---------|--------|
| `make spec013-proof-pr` | 8 github + 1 vector stats — pass |
| `make spec013-proof` | 8 + **7** Mistral + 1 — pass (~92s) |
| `make spec013-proof-ui` | 6 Playwright — pass (`http://localhost:8083` from start script) |
| `make spec013-entity-type-audit-all` | 0 violations across scanned workspaces |

New harness: `wait_until_app_ready` polls `/health`; `spec013_effective_backend_url` reads runtime `PORT` from `/tmp/edgequake-start.sh`.
