# Canonical EdgeQuake HTTP API (from `routes.rs`)

Source: `edgequake/crates/edgequake-api/src/routes.rs` (commit at `feat/sdk`).  
Prefix: business routes use `/api/v1/…` unless noted. Protected routes use `protected_api_auth` middleware.

## Unversioned

| Method | Path | Handler module |
|--------|------|----------------|
| GET | `/health` | `handlers::health_check` |
| GET | `/ready` | `handlers::readiness_check` |
| GET | `/live` | `handlers::liveness_check` |
| GET | `/metrics` | `handlers::get_metrics` |
| GET | `/ws/pipeline/progress` | WebSocket |
| GET | `/ws/progress/{track_id}` | WebSocket |

## Ollama emulation (`/api/…`)

| Method | Path |
|--------|------|
| GET | `/api/version` |
| GET | `/api/tags` |
| GET | `/api/ps` |
| POST | `/api/generate` |
| POST | `/api/chat` |

## `/api/v1/auth`

| POST | `/api/v1/auth/login` |
| POST | `/api/v1/auth/refresh` |
| POST | `/api/v1/auth/logout` |
| GET | `/api/v1/auth/me` |

## `/api/v1/users`

| POST | `/api/v1/users` |
| GET | `/api/v1/users` |
| GET | `/api/v1/users/{user_id}` |
| DELETE | `/api/v1/users/{user_id}` |

## `/api/v1/api-keys`

| POST | `/api/v1/api-keys` |
| GET | `/api/v1/api-keys` |
| DELETE | `/api/v1/api-keys/{key_id}` |

## `/api/v1/tenants` & workspaces under tenant

| POST | `/api/v1/tenants` |
| GET | `/api/v1/tenants` |
| GET | `/api/v1/tenants/{tenant_id}` |
| PUT | `/api/v1/tenants/{tenant_id}` |
| DELETE | `/api/v1/tenants/{tenant_id}` |
| POST | `/api/v1/tenants/{tenant_id}/workspaces` |
| GET | `/api/v1/tenants/{tenant_id}/workspaces` |
| GET | `/api/v1/tenants/{tenant_id}/workspaces/by-slug/{slug}` |

## `/api/v1/workspaces/{workspace_id}`

| GET | base |
| PUT | base |
| DELETE | base |
| GET | `…/stats` |
| GET | `…/metrics-history` |
| POST | `…/metrics-snapshot` |
| POST | `…/rebuild-embeddings` |
| POST | `…/rebuild-knowledge-graph` |
| POST | `…/reprocess-documents` |
| PUT | `…/injection` |
| PUT | `…/injection/file` |
| GET | `…/injections` |
| GET | `…/injections/{injection_id}` |
| PATCH | `…/injections/{injection_id}` |
| DELETE | `…/injections/{injection_id}` |

## `/api/v1/admin`

| PATCH | `/api/v1/admin/tenants/{tenant_id}/quota` |
| GET | `/api/v1/admin/config/defaults` |
| PATCH | `/api/v1/admin/config/defaults` |

## `/api/v1/documents` (order-sensitive in router)

| POST | `/api/v1/documents` (text/JSON upload) |
| GET | `/api/v1/documents` |
| DELETE | `/api/v1/documents` (bulk) |
| GET | `/api/v1/documents/track/{track_id}` |
| POST | `/api/v1/documents/upload` |
| POST | `/api/v1/documents/upload/batch` |
| POST | `/api/v1/documents/pdf` |
| GET | `/api/v1/documents/pdf` |
| GET | `/api/v1/documents/pdf/progress/{track_id}` |
| GET | `/api/v1/documents/pdf/progress/stream/{track_id}` (SSE) |
| POST | `/api/v1/documents/pdf/{pdf_id}/retry` |
| DELETE | `/api/v1/documents/pdf/{pdf_id}/cancel` |
| GET | `/api/v1/documents/pdf/{pdf_id}/download` |
| GET | `/api/v1/documents/pdf/{pdf_id}/content` |
| GET | `/api/v1/documents/pdf/{pdf_id}` |
| DELETE | `/api/v1/documents/pdf/{pdf_id}` |
| POST | `/api/v1/documents/scan` |
| POST | `/api/v1/documents/reprocess` |
| POST | `/api/v1/documents/recover-stuck` |
| GET | `/api/v1/documents/{document_id}/deletion-impact` |
| POST | `/api/v1/documents/{document_id}/retry-chunks` |
| GET | `/api/v1/documents/{document_id}/failed-chunks` |
| GET | `/api/v1/documents/{document_id}/lineage` |
| GET | `/api/v1/documents/{document_id}/metadata` |
| GET | `/api/v1/documents/{document_id}/lineage/export` |
| GET | `/api/v1/documents/{document_id}` |
| DELETE | `/api/v1/documents/{document_id}` |

## Query & chat

| POST | `/api/v1/query` |
| POST | `/api/v1/query/stream` |
| POST | `/api/v1/chat/completions` |
| POST | `/api/v1/chat/completions/stream` |

## Conversations, messages, folders, shared

| GET/POST | `/api/v1/conversations` |
| POST | `/api/v1/conversations/import` |
| POST | `/api/v1/conversations/bulk/delete` |
| POST | `/api/v1/conversations/bulk/archive` |
| POST | `/api/v1/conversations/bulk/move` |
| GET/PATCH/DELETE | `/api/v1/conversations/{id}` |
| GET/POST | `/api/v1/conversations/{id}/messages` |
| POST/DELETE | `/api/v1/conversations/{id}/share` |
| PATCH/DELETE | `/api/v1/messages/{message_id}` |
| GET/POST | `/api/v1/folders` |
| PATCH/DELETE | `/api/v1/folders/{folder_id}` |
| GET | `/api/v1/shared/{share_id}` |

## Graph

| GET | `/api/v1/graph` |
| GET | `/api/v1/graph/stream` |
| GET | `/api/v1/graph/nodes/{node_id}` |
| GET | `/api/v1/graph/nodes/search` |
| GET | `/api/v1/graph/labels/search` |
| GET | `/api/v1/graph/labels/popular` |
| POST | `/api/v1/graph/degrees/batch` |
| GET+POST | `/api/v1/graph/entities` |
| GET | `/api/v1/graph/entities/exists` |
| POST | `/api/v1/graph/entities/merge` |
| GET/PUT/DELETE | `/api/v1/graph/entities/{entity_name}` |
| GET | `/api/v1/graph/entities/{entity_name}/neighborhood` |
| GET+POST | `/api/v1/graph/relationships` |
| GET/PUT/DELETE | `/api/v1/graph/relationships/{relationship_id}` |

## Tasks & pipeline & costs

| GET | `/api/v1/tasks` |
| GET | `/api/v1/tasks/{track_id}` |
| POST | `/api/v1/tasks/{track_id}/cancel` |
| POST | `/api/v1/tasks/{track_id}/retry` |
| GET | `/api/v1/pipeline/status` |
| POST | `/api/v1/pipeline/cancel` |
| GET | `/api/v1/pipeline/queue-metrics` |
| GET | `/api/v1/pipeline/costs/pricing` |
| POST | `/api/v1/pipeline/costs/estimate` |
| GET | `/api/v1/costs/summary` |
| GET | `/api/v1/costs/history` |
| GET/PATCH | `/api/v1/costs/budget` |

## Lineage, chunks, provenance

| GET | `/api/v1/lineage/entities/{entity_name}` |
| GET | `/api/v1/lineage/documents/{document_id}` |
| GET | `/api/v1/chunks/{chunk_id}` |
| GET | `/api/v1/chunks/{chunk_id}/lineage` |
| GET | `/api/v1/entities/{entity_id}/provenance` |

## Settings, config, models

| GET | `/api/v1/settings/provider/status` |
| GET | `/api/v1/settings/providers` |
| GET | `/api/v1/config/effective` |
| GET | `/api/v1/models` |
| GET | `/api/v1/models/llm` |
| GET | `/api/v1/models/embedding` |
| GET | `/api/v1/models/health` |
| GET | `/api/v1/models/{provider}` |
| GET | `/api/v1/models/{provider}/{model}` |

### Not routed (SDK must not invent)

- `PUT /api/v1/settings/provider` — **not** in `routes.rs`.
- `POST /api/v1/workspaces/{id}/rebuild` — **not** in `routes.rs` (use rebuild-embeddings / rebuild-knowledge-graph / reprocess-documents).
- `GET /api/v1/documents/{id}/status` — **not** in `routes.rs` (use `GET /documents/{id}` or track endpoints).
- `GET /api/v1/documents/{id}/chunks` — **not** in `routes.rs`.
- `PATCH /api/v1/documents/{id}/metadata` — **not** in `routes.rs` (metadata is **GET** only).
- `POST /api/v1/documents/upload/text` — **not** in `routes.rs` (use `POST /documents`).
- `GET /api/v1/graph/stats`, `POST /api/v1/graph/clear`, `DELETE /api/v1/graph` (bulk clear) — **not** in `routes.rs`.
- `GET /api/v1/graph/entities/types`, `GET /api/v1/graph/relationships/types` — **not** in `routes.rs`.
- `POST /api/v1/api-keys/{id}/revoke` — **not** in `routes.rs` (use `DELETE /api-keys/{key_id}`).
- `PUT /api/v1/users/{id}` — **not** in `routes.rs`.

## Cross-references

- **SDK coverage (by endpoint / DTO):** [SDK-API-COVERAGE.md](./SDK-API-COVERAGE.md)  
- Rust gap: [RUST-SDK-GAP.md](./RUST-SDK-GAP.md)  
- Python gap: [python-skd/GAP.md](./python-skd/GAP.md)  
- Node/TS gap: [node-js/GAP.md](./node-js/GAP.md)
