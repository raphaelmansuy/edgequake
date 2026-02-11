# Iteration 07: Observe

## Date: 2025-07-25

## What We Observed

### 1. Missing userId Support

After iteration_06 established working E2E tests, we observed that the Chat endpoint returns 401 Unauthorized when `X-User-ID` header is missing. The SDK's `EdgeQuakeConfig` had no `userId` field, and the tenant middleware didn't send this header.

### 2. Type Mismatches Between SDK and Rust API

Systematic comparison of SDK TypeScript types against Rust handler response types revealed significant mismatches:

| Type                     | SDK (Before)                                        | Rust API (Actual)                                                                                                   |
| ------------------------ | --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `TenantInfo`             | `{tenant_id, name, created_at}`                     | `{id, name, slug, plan, is_active, max_workspaces, default_llm_*, default_embedding_*, created_at, updated_at}`     |
| `WorkspaceInfo`          | `{workspace_id, tenant_id, name, slug, created_at}` | `{id, tenant_id, name, slug, description?, is_active, max_documents?, llm_*, embedding_*, created_at, updated_at}`  |
| `TaskStatus`             | `{id, status, ...}`                                 | `{track_id, tenant_id, workspace_id, task_type, status, error?: TaskErrorDetail, retry_count, max_retries, ...}`    |
| `PipelineStatus`         | `{status, active_tasks, ...}`                       | `{is_busy, total_documents, processed_documents, current_batch, history_messages: PipelineMessage[], ...}`          |
| `ChatCompletionResponse` | `{message, conversation_id}`                        | `{content, conversation_id, user_message_id, assistant_message_id, mode, sources, stats, tokens_used, duration_ms}` |
| `ChatStreamEvent`        | `{content, delta}` style                            | Discriminated union: `conversation\|context\|token\|thinking\|done\|error`                                          |

### 3. Paginated Response Extraction

Tenant and workspace list endpoints return `{items: [...], total, offset, limit}` but the SDK was returning the raw wrapper object instead of extracting `items`.

### 4. E2E Coverage Gaps

No E2E tests existed for: tenants CRUD, workspaces CRUD, tasks listing, pipeline status, settings/providers, models listing.

### 5. Chat Working with Proper Context

With correct `X-Tenant-ID`, `X-User-ID`, and `X-Workspace-ID` headers, chat completions and streaming work correctly against the live API.

## Metrics

| Metric           | Before (IMPL-06) | After (IMPL-07) |
| ---------------- | ---------------- | --------------- |
| Unit Tests       | 243              | 243             |
| E2E Tests        | 24               | 46              |
| E2E Test Files   | 4                | 6               |
| Build Size (ESM) | 45.03 KB         | 46.03 KB        |
| Build Size (CJS) | 45.56 KB         | 46.56 KB        |
| Build Size (DTS) | 63.36 KB         | 67.86 KB        |
| tsc --noEmit     | Clean            | Clean           |
