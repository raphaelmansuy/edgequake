# Iteration 01 — Decide

## Prioritized Tasks

### P0: Project Scaffolding

- [ ] Create `sdks/typescript/` directory structure
- [ ] Create `package.json` with @edgequake/sdk name
- [ ] Create `tsconfig.json` for strict TypeScript
- [ ] Create `tsup.config.ts` for ESM+CJS dual build
- [ ] Create `vitest.config.ts` for unit testing

### P1: Foundation Layer

- [ ] `src/errors.ts` — 12 error classes (EdgeQuakeError hierarchy)
- [ ] `src/types/common.ts` — Page<T>, PaginatedResponse
- [ ] `src/types/auth.ts` — Login, Token, User types
- [ ] `src/types/documents.ts` — Document CRUD types
- [ ] `src/types/query.ts` — Query request/response types
- [ ] `src/types/chat.ts` — Chat completion types
- [ ] `src/types/graph.ts` — Graph, Entity, Relationship types
- [ ] `src/types/conversations.ts` — Conversation, Message types
- [ ] `src/types/workspaces.ts` — Workspace, Tenant types
- [ ] `src/types/tasks.ts` — Task tracking types
- [ ] `src/types/costs.ts` — Cost tracking types
- [ ] `src/types/index.ts` — Re-export all types

### P2: Transport Layer

- [ ] `src/transport/types.ts` — TransportConfig, RequestOptions, HttpTransport
- [ ] `src/transport/fetch.ts` — FetchTransport implementing HttpTransport
- [ ] `src/transport/retry.ts` — Retry middleware with exponential backoff
- [ ] `src/transport/middleware.ts` — Auth and tenant middleware
- [ ] `src/transport/index.ts` — createTransport factory

### P3: Resource Infrastructure

- [ ] `src/resources/base.ts` — Resource base class
- [ ] `src/pagination.ts` — Paginator<T> with AsyncIterable

### P4: Client & All Resources

- [ ] `src/config.ts` — EdgeQuakeConfig + resolveConfig
- [ ] `src/client.ts` — EdgeQuake class with all 21 resource namespaces
- [ ] `src/resources/auth.ts` — AuthResource (4 methods)
- [ ] `src/resources/documents.ts` — DocumentsResource + PdfResource (23 methods)
- [ ] `src/resources/query.ts` — QueryResource (2 methods)
- [ ] `src/resources/chat.ts` — ChatResource (2 methods)
- [ ] `src/resources/graph.ts` — GraphResource + Entities + Relationships (21 methods)
- [ ] `src/resources/conversations.ts` — ConversationsResource + Messages (20 methods)
- [ ] `src/resources/workspaces.ts` — WorkspacesResource (12 methods)
- [ ] `src/resources/tenants.ts` — TenantsResource (5 methods)
- [ ] `src/resources/users.ts` — UsersResource (4 methods)
- [ ] `src/resources/api-keys.ts` — ApiKeysResource (3 methods)
- [ ] `src/resources/tasks.ts` — TasksResource (4 methods)
- [ ] `src/resources/pipeline.ts` — PipelineResource (5 methods)
- [ ] `src/resources/costs.ts` — CostsResource (4 methods)
- [ ] `src/resources/lineage.ts` — LineageResource (2 methods)
- [ ] `src/resources/chunks.ts` — ChunksResource (1 method)
- [ ] `src/resources/provenance.ts` — ProvenanceResource (1 method)
- [ ] `src/resources/settings.ts` — SettingsResource (2 methods)
- [ ] `src/resources/models.ts` — ModelsResource (6 methods)
- [ ] `src/resources/websocket.ts` — WebSocketResource (2 methods)
- [ ] `src/resources/folders.ts` — FoldersResource (4 methods)
- [ ] `src/resources/shared.ts` — SharedResource (1 method)
- [ ] `src/streaming/sse.ts` — SSE parser
- [ ] `src/streaming/websocket.ts` — WebSocket wrapper
- [ ] `src/index.ts` — Public API exports

### P5: Unit Tests

- [ ] `tests/unit/errors.test.ts` — Error class tests
- [ ] `tests/unit/transport.test.ts` — Transport layer tests
- [ ] `tests/unit/pagination.test.ts` — Paginator tests
- [ ] `tests/unit/client.test.ts` — Client creation tests

### P6: Documentation

- [ ] `README.md` — Getting started guide
- [ ] `CHANGELOG.md` — v0.1.0 entry
- [ ] `LICENSE` — MIT

## Commit Plan

Single commit: `IMPL-01: TypeScript SDK foundation — project scaffolding, 131-endpoint client, transport layer, types, error handling, unit tests`
