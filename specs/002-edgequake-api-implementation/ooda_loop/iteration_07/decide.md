# Iteration 07: Decide

## Date: 2025-07-25

## Decisions Made

### 1. Add userId to SDK Config Pipeline

- Add `userId?: string` to `EdgeQuakeConfig`
- Add `userId: string` to `ResolvedConfig` (defaults to empty string)
- Read `EDGEQUAKE_USER_ID` env var as fallback
- Send `X-User-ID` header via tenant middleware when set

### 2. Rewrite Types to Match Rust API Exactly

- **workspaces.ts**: `CreateTenantRequest`, `TenantInfo`, `WorkspaceInfo`, `CreateWorkspaceRequest`, `UpdateWorkspaceRequest`, `UpdateTenantRequest` — all updated with model config fields (LLM provider/model, embedding provider/model/dimension)
- **tasks.ts**: `TaskStatus` with full TaskResponse fields, `TaskErrorDetail`, `TaskListResponse` with pagination+statistics, `ListTasksQuery`, `PipelineStatus` with `PipelineMessage[]`
- **chat.ts**: `ChatCompletionResponse` with `content`/`mode`/`sources`/`stats`, `ChatStreamEvent` as discriminated union (6 event types), `SourceReference`, `QueryStats`

### 3. Fix Resource Methods

- `tasks.list()`: Accept `ListTasksQuery` params, build URL search params, return `TaskListResponse`
- `tenants.list()`: Extract `items` from paginated response
- `tenants.listWorkspaces()`: Same items extraction

### 4. Update Unit Test Mocks

- ChatResource unit tests: Update mock response from OpenAI-style to EdgeQuake format
- Update test assertions to match new type fields

### 5. Add 2 New E2E Test Files

- `tenants-workspaces.test.ts`: 6 tests covering tenant CRUD and workspace operations
- `tasks-pipeline.test.ts`: 11 tests covering tasks, pipeline, settings, models

### 6. Not Changed (Deferred)

- `workspace_metrics_history` table creation (database schema change needed)
- Retry mechanism for flaky document delete (existing 409 handling is sufficient)
- Conversation E2E tests (deferred to iteration 08)
