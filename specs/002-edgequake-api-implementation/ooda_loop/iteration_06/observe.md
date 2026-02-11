# Iteration 06 — Observe

## E2E Tests Against Live Backend: Reality Check

### What We Found

Ran 24 E2E tests against live EdgeQuake backend (http://localhost:8080) with freshly restarted services (PostgreSQL + Ollama + backend). Initial run revealed **19 failures** across multiple categories.

### SDK Bugs Discovered

| Bug                                        | Root Cause                                                                              | Impact                  |
| ------------------------------------------ | --------------------------------------------------------------------------------------- | ----------------------- |
| `/ready`, `/live` return text "OK"         | FetchTransport always called `.json()`                                                  | Health checks crash     |
| `client.health()` timeout                  | Backend slow after E2E load; no explicit test timeouts                                  | 4 tests timeout at 5s   |
| `documents.list()` wrong format            | API returns `{documents:[]}`, SDK expected `{items:[]}`                                 | Pagination broken       |
| `CreateEntityRequest` wrong fields         | SDK had `{name, label}`, API needs `{entity_name, entity_type, description, source_id}` | Entity creation fails   |
| `entities.exists()` wrong param            | SDK uses `?name=`, API expects `?entity_name=`                                          | 400 Bad Request         |
| `entities.list()` returns paginated object | SDK declared `Promise<EntityDetail[]>` but API returns `{items:[],total,...}`           | `Array.isArray()` fails |
| `relationships.list()` same issue          | Paginated response not extracted                                                        | Consistency issue       |
| Query stream returns raw text SSE          | `_streamSSE()` tries `JSON.parse()` on raw text, silently skips all                     | 0 chunks received       |
| Chat endpoint requires tenant context      | `X-Tenant-ID` + `X-User-ID` headers (UUIDs) mandatory                                   | 401 Unauthorized        |

### API Response Formats Confirmed (via curl)

```
GET /ready            → text/plain "OK"
GET /live             → text/plain "OK"
GET /health           → application/json {status, version, storage_mode, ...}
GET /api/v1/documents → {documents:[], total, page, page_size, ...}
GET /api/v1/graph/entities → {items:[], total, page, page_size, total_pages}
GET /api/v1/graph/relationships → {items:[], total, page, page_size, total_pages}
GET /api/v1/graph/entities/exists?entity_name=X → {exists:bool, ...}
POST /api/v1/query/stream → SSE: "data: <raw text>\n\n"
POST /api/v1/chat/completions → requires X-Tenant-ID, X-User-ID headers
```

### Test Infrastructure Issues

- Default vitest timeout 5000ms insufficient for backend endpoints that check DB/LLM
- Running vitest from workspace root (v4.0.18) vs SDK dir (v3.2.4) — different configs
- Backend health endpoint can hang under load (exhausted DB connections?)
