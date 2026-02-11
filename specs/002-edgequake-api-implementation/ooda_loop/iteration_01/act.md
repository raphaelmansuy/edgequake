# Iteration 01 — ACT: TypeScript SDK Foundation

## Summary

Implemented the complete TypeScript SDK foundation for the EdgeQuake API, covering all 131+ endpoints across 27 resource categories.

## Changes Made

### Project Scaffolding

- `sdks/typescript/package.json` — Package config with ESM+CJS dual exports
- `sdks/typescript/tsconfig.json` — TypeScript 5.7; strict mode, ES2022 target
- `sdks/typescript/tsup.config.ts` — tsup v8.5 bundler config
- `sdks/typescript/vitest.config.ts` — vitest v3.0 test config with 80% coverage thresholds

### Error Handling (12 classes)

- `src/errors.ts` — Full error hierarchy: EdgeQuakeError → BadRequestError(400), UnauthorizedError(401), ForbiddenError(403), NotFoundError(404), ConflictError(409), PayloadTooLargeError(413), ValidationError(422), RateLimitError(429), InternalError(500), ServiceUnavailableError(503), TimeoutError(408), NetworkError(0)
- `parseErrorResponse()` — Maps HTTP status codes to typed error instances

### Type Definitions (12 files)

- `src/types/common.ts` — Page<T>, ListQuery, PageQuery, TaskStatusValue, BulkOperationResponse
- `src/types/auth.ts` — Login, JWT refresh, user management, API key types
- `src/types/documents.ts` — Upload, list, track, scan, reprocess, PDF types
- `src/types/query.ts` — RAG query request/response, stream events
- `src/types/chat.ts` — Chat completion request/response, stream events
- `src/types/graph.ts` — Graph, entity, relationship types with CRUD and neighborhood
- `src/types/conversations.ts` — Conversations, messages, folders, sharing, bulk ops
- `src/types/workspaces.ts` — Tenants, workspaces, stats, metrics history
- `src/types/tasks.ts` — Task tracking, pipeline status, queue metrics, cost estimation
- `src/types/costs.ts` — Cost summary, history, budget management
- `src/types/health.ts` — Health checks, providers, models, lineage, chunks, provenance, Ollama, WebSocket events
- `src/types/index.ts` — Barrel re-export

### Transport Layer (5 files)

- `src/transport/types.ts` — HttpTransport interface, RequestOptions, Middleware type
- `src/transport/fetch.ts` — FetchTransport: JSON, SSE streaming, multipart upload, blob download, timeout, error parsing
- `src/transport/middleware.ts` — Auth (X-API-Key / Bearer) + Tenant (X-Tenant-ID / X-Workspace-ID) middleware
- `src/transport/retry.ts` — Exponential backoff with jitter, configurable status codes
- `src/transport/index.ts` — createTransport factory accepting ResolvedConfig

### Resources (21 files)

- `src/resources/base.ts` — Abstract Resource class with \_get, \_post, \_put, \_patch, \_del, \_streamSSE
- `src/resources/auth.ts` — Login, refresh, logout, me
- `src/resources/users.ts` — CRUD user management
- `src/resources/api-keys.ts` — Create, list, revoke API keys
- `src/resources/documents.ts` — Upload, list, get, delete, scan, reprocess + PdfResource sub-namespace
- `src/resources/query.ts` — Execute + stream RAG queries
- `src/resources/chat.ts` — Completions + stream chat
- `src/resources/conversations.ts` — CRUD + share + bulk ops + MessagesResource sub-namespace
- `src/resources/folders.ts` — Conversation folder CRUD
- `src/resources/shared.ts` — Public shared conversation access
- `src/resources/graph.ts` — Graph query + stream + EntitiesResource + RelationshipsResource sub-namespaces
- `src/resources/tenants.ts` — Multi-tenant CRUD + workspace management
- `src/resources/workspaces.ts` — Workspace CRUD + stats + rebuild + reprocess
- `src/resources/tasks.ts` — Task tracking + cancel + retry
- `src/resources/pipeline.ts` — Pipeline status + queue metrics + cost estimation
- `src/resources/costs.ts` — Cost summary + history + budget management
- `src/resources/lineage.ts` — Entity + document lineage
- `src/resources/chunks.ts` — Chunk detail
- `src/resources/provenance.ts` — Entity provenance
- `src/resources/settings.ts` — Provider status + list providers
- `src/resources/models.ts` — Model CRUD + provider health
- `src/resources/ollama.ts` — Ollama-compatible API

### Client & Config

- `src/config.ts` — EdgeQuakeConfig + resolveConfig (env var + explicit)
- `src/client.ts` — EdgeQuake class with 21 resource namespaces + health/ready/live

### Streaming & Pagination

- `src/pagination.ts` — Paginator<T> implementing AsyncIterable
- `src/streaming/sse.ts` — parseSSEStream<T> for SSE data parsing
- `src/streaming/websocket.ts` — EdgeQuakeWebSocket implementing AsyncIterable<WebSocketEvent>

### Public API

- `src/index.ts` — Re-exports: EdgeQuake, config, errors, pagination, streaming, transport, types

### Tests (59 passing)

- `tests/unit/errors.test.ts` — 25 tests: error classes, status codes, parseErrorResponse
- `tests/unit/client.test.ts` — 6 tests: construction, config, resource namespaces
- `tests/unit/config.test.ts` — 6 tests: defaults, env vars, explicit config, overrides
- `tests/unit/pagination.test.ts` — 6 tests: iteration, single page, empty, getPage, toArray, firstPage
- `tests/unit/transport.test.ts` — 16 tests: GET, POST, 204, errors (404/400/500), WebSocket URL, query params, auth/tenant/retry middleware

### Build Output

- ESM: `dist/index.js` (43.89 KB)
- CJS: `dist/index.cjs` (44.42 KB)
- Types: `dist/index.d.ts` (62.53 KB)
- Types (CJS): `dist/index.d.cts` (62.53 KB)

## Metrics

- **Files created**: 42
- **Type definitions**: 12 files covering all 131 API endpoints
- **Resource classes**: 21 (with 4 sub-resource namespaces)
- **Error classes**: 12 + parser
- **Unit tests**: 59 (all passing)
- **Build size**: ~44 KB ESM, ~44 KB CJS, ~62 KB types
- **Dependencies**: 0 runtime, 4 dev
- **TypeScript errors**: 0
- **tsc --noEmit**: Clean

## Decisions

- Used tsup over tsdown (tsdown still pre-1.0; tsup is battle-tested)
- Zero runtime dependencies (native fetch only)
- Dual ESM+CJS output for maximum compatibility
- Resource namespace pattern (Stripe/OpenAI style)
- Middleware-based transport for composable request processing
- Paginator<T> as AsyncIterable for ergonomic pagination
- Environment-based config resolution for deployment flexibility
