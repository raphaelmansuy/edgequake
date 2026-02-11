# Iteration 03 — Act

## What Was Done

### Package Polish

- Updated `prepublishOnly` script: `lint → test → build` (was just `build`)
- Changed license from MIT to Apache-2.0 (per mission spec)
- Created `LICENSE` file (Apache License 2.0)
- Created `CHANGELOG.md` (Keep a Changelog format)
- Added `.gitignore` with `coverage/` exclusion (iter_02)

### Examples (8 files)

| File                             | Description                                         |
| -------------------------------- | --------------------------------------------------- |
| `examples/basic_usage.ts`        | Setup, health check, upload, query                  |
| `examples/document_upload.ts`    | Text + PDF upload, tracking, pagination, delete     |
| `examples/query_demo.ts`         | Simple, hybrid, chat completion                     |
| `examples/graph_exploration.ts`  | Entity search, neighborhood, relationships, labels  |
| `examples/streaming_query.ts`    | SSE query stream, chat stream, abort                |
| `examples/websocket_progress.ts` | WebSocket pipeline progress, task tracking          |
| `examples/multi_tenant.ts`       | Tenant/workspace CRUD, scoped client                |
| `examples/batch_operations.ts`   | Bulk upload, pagination, bulk delete, cost estimate |

### CI/CD Pipelines (2 workflows)

- `.github/workflows/test.yml` — PR/push testing (Node 18/20/22 matrix), build verification
- `.github/workflows/publish.yml` — npm publish on `sdk-ts-v*` tag with provenance

### Documentation (3 docs)

- `docs/API.md` — Complete API reference (all 21 resources, all methods, error types, pagination, streaming)
- `docs/AUTHENTICATION.md` — API key, JWT, multi-tenant, middleware, best practices
- `docs/STREAMING.md` — SSE + WebSocket guide, parsing, error handling, abort patterns

## Test Verification

- 243 tests pass (unchanged from iteration_02)
- 98.52% line coverage maintained

## Outcome

Phase 1 deliverables nearly complete:

- ✅ Project structure
- ✅ Core client + all resources
- ✅ Unit tests (>90% coverage)
- ✅ Examples (8 files)
- ✅ CI/CD pipelines
- ✅ Documentation (API, Auth, Streaming)
- ⏳ Integration tests (require backend — iteration_04)
- ⏳ npm publish (requires NPM_TOKEN — deferred)
