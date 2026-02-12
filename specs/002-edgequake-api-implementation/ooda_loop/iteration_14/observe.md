# OODA Iteration 14 — Observe: TypeScript SDK E2E Audit

**Date**: 2026-02-11
**Mission**: `specs/002-edgequake-api-implementation.md` (re-read at start)
**SDK**: TypeScript (`sdks/typescript/`)
**Backend**: localhost:8080 (healthy, openai/gpt-4.1-nano, postgresql)

## Environment

- Test Tenant: `774e3598-ec47-48a5-ba78-c4bcbedf3774` (sdk-e2e-test)
- Test Workspace: `1b48dd1e-735a-4f57-b74a-ce20c5c7a5a9` (sdk-e2e-workspace)
- Test Document: `cbb771ab-8797-4d74-b25b-c77d03b871ad` (8 entities, 6 relationships)

## SDK Structure

```
sdks/typescript/
├── src/resources/ (22 resource modules)
│   auth, api-keys, chat, chunks, conversations, costs, documents,
│   folders, graph, lineage, models, ollama, pipeline, provenance,
│   query, settings, shared, tasks, tenants, users, workspaces
├── tests/e2e/ (8 test files, 62 total tests)
├── tests/unit/ (comprehensive unit tests)
├── examples/ (working examples)
└── docs/ (API documentation)
```

## E2E Test Run — `EDGEQUAKE_E2E_URL=http://localhost:8080`

| Test File                     | Passed | Failed | Skipped |
| ----------------------------- | ------ | ------ | ------- |
| health.test.ts                | 8      | 0      | 0       |
| documents.test.ts             | 4      | 0      | 0       |
| graph.test.ts                 | 7      | 0      | 0       |
| query.test.ts                 | 5      | 0      | 0       |
| tenants-workspaces.test.ts    | 6      | 0      | 0       |
| auth-costs.test.ts            | 7      | 0      | 0       |
| tasks-pipeline.test.ts        | 11     | 0      | 0       |
| conversations-folders.test.ts | 0      | 6      | 8       |
| **TOTAL**                     | **48** | **6**  | **8**   |

## Failure Analysis

All 6 failures are in `conversations-folders.test.ts`:

- `lists conversations with cursor-based pagination` → Missing X-Tenant-ID header
- `creates, gets, updates, and deletes a conversation` → Missing X-Tenant-ID header
- `filters conversations by archived status` → Missing X-Tenant-ID header
- `filters conversations by mode` → Missing X-Tenant-ID header
- `lists folders` → Missing X-Tenant-ID header
- `creates, updates, and deletes a folder` → Missing X-Tenant-ID header

**Root cause**: Conversation/folder endpoints require `X-Tenant-ID` and `X-User-ID` headers.
When `EDGEQUAKE_TENANT_ID` is not set, the SDK sends no tenant header → 400 error.

## Endpoint Coverage

| Category      | Routes in routes.rs                  | Covered by SDK | Gap                                     |
| ------------- | ------------------------------------ | -------------- | --------------------------------------- |
| Health/System | 4 (/health, /ready, /live, /metrics) | 3              | /metrics missing                        |
| Documents     | 13                                   | 8              | PDF, scan, reprocess, recover endpoints |
| Query         | 2                                    | 2              | ✅                                      |
| Chat          | 2                                    | 2              | ✅                                      |
| Conversations | 12                                   | 12             | ✅                                      |
| Folders       | 4                                    | 4              | ✅                                      |
| Messages      | 4                                    | 4              | ✅                                      |
| Graph         | 7                                    | 6              | degrees/batch missing                   |
| Entities      | 7                                    | 6              | merge missing                           |
| Relationships | 5                                    | 4              | update missing                          |
| Tasks         | 4                                    | 3              | retry missing                           |
| Pipeline      | 3                                    | 3              | ✅                                      |
| Costs         | 5                                    | 4              | estimate missing                        |
| Tenants       | 5                                    | 5              | ✅                                      |
| Workspaces    | 8                                    | 6              | rebuild, reprocess missing              |
| Users         | 4                                    | 4              | ✅                                      |
| API Keys      | 3                                    | 3              | ✅                                      |
| Auth          | 4                                    | 4              | ✅                                      |
| Models        | 6                                    | 5              | get specific model missing              |
| Settings      | 2                                    | 2              | ✅                                      |
| Lineage       | 2                                    | 2              | ✅                                      |
| Chunks        | 1                                    | 1              | ✅                                      |
| Provenance    | 1                                    | 1              | ✅                                      |
| Shared        | 1                                    | 1              | ✅                                      |
| Ollama        | 5                                    | 5              | ✅                                      |
