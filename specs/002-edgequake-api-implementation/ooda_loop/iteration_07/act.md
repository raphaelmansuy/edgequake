# Iteration 07: Act

## Date: 2025-07-25

## Changes Implemented

### Files Modified

| File                           | Change                                                                                                                  |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| `src/config.ts`                | Added `userId` to EdgeQuakeConfig, ResolvedConfig, resolveConfig                                                        |
| `src/transport/middleware.ts`  | Added `userId` to TenantConfig, sends X-User-ID header                                                                  |
| `src/transport/index.ts`       | Passes userId to createTenantMiddleware                                                                                 |
| `src/types/workspaces.ts`      | Complete rewrite — TenantInfo, WorkspaceInfo, Create/UpdateRequest with model config                                    |
| `src/types/tasks.ts`           | Complete rewrite — TaskStatus, TaskErrorDetail, TaskListResponse, PipelineStatus, PipelineMessage                       |
| `src/types/chat.ts`            | Complete rewrite — ChatCompletionResponse (content), ChatStreamEvent (discriminated union), SourceReference, QueryStats |
| `src/resources/tasks.ts`       | list() accepts ListTasksQuery, returns TaskListResponse                                                                 |
| `src/resources/tenants.ts`     | list() and listWorkspaces() extract items from paginated response                                                       |
| `tests/unit/config.test.ts`    | Added userId tests                                                                                                      |
| `tests/unit/transport.test.ts` | Added X-User-ID header test                                                                                             |
| `tests/unit/resources.test.ts` | Updated ChatResource mock to match new response format                                                                  |
| `tests/e2e/helpers.ts`         | Added E2E_TENANT_ID, E2E_USER_ID, passes to client                                                                      |
| `tests/e2e/query.test.ts`      | Fixed chat response field (content), stream event type (token)                                                          |

### Files Created

| File                                   | Purpose                                         |
| -------------------------------------- | ----------------------------------------------- |
| `tests/e2e/tenants-workspaces.test.ts` | 6 E2E tests: tenant CRUD, workspace operations  |
| `tests/e2e/tasks-pipeline.test.ts`     | 11 E2E tests: tasks, pipeline, settings, models |

### Verification Results

| Check            | Result                                     |
| ---------------- | ------------------------------------------ |
| `tsc --noEmit`   | ✅ Clean                                   |
| Unit tests       | ✅ 243 passed                              |
| E2E tests        | ✅ 46 passed (6 files)                     |
| Build (ESM)      | ✅ 46.03 KB                                |
| Build (CJS)      | ✅ 46.56 KB                                |
| Build (DTS)      | ✅ 67.86 KB                                |
| Chat completions | ✅ Working with tenant/user context        |
| Chat streaming   | ✅ Working with discriminated union events |

### Commit

```
IMPL-07: userId support, type accuracy vs Rust API, E2E expansion (46 tests)
```

### Next Iteration (08) Candidates

- Conversations E2E tests (CRUD, messages, with tenant context)
- Extended document operations (PDF upload, scan, batch, track status)
- Graph extended endpoints (stream, nodes search, labels, degrees batch)
- Costs resource E2E tests
- Lineage and provenance E2E tests
- Add `.env.e2e.example` with all required E2E env vars documented
