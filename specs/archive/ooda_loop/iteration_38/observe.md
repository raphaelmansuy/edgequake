# Iteration 38: Observe

## Mission Re-Read Confirmation

✅ Read mission file: `specs/001-improve-ingestion-process.md`

- 8 new requirements added (Issues 7-14)
- Focus: Timeout/retry handling (Issue 8)

## Codebase Exploration

### Issue 7: Document Cancel - ALREADY IMPLEMENTED ✅

Found existing implementation:

- **Backend**: `/edgequake/crates/edgequake-api/src/handlers/tasks.rs:155`
  - `cancel_task` endpoint at `/api/v1/tasks/{track_id}/cancel`
  - Checks task status before cancellation
  - Returns 409 if already indexed/cancelled
- **Frontend**: `/edgequake_webui/src/components/documents/document-manager.tsx`
  - `cancelTask` API client function (line 54)
  - `cancelMutation` hook (line 543)
  - Cancel button in dropdown menu (line 1394-1398)
  - Toast notifications for success/failure

No changes needed for Issue 7.

### Issue 8: Timeout and Retry Handling - Gaps Found

**Current State:**

- `WorkerPoolConfig.retry_delay_secs` - Fixed 5s delay (no backoff)
- `Task.retry_count` and `Task.max_retries` - Tracking exists
- No timeout on individual extraction operations
- No circuit breaker pattern

**Files to Modify:**

1. `edgequake-pipeline/src/pipeline.rs` - Add timeout config
2. `edgequake-pipeline/src/error.rs` - Add timeout/retry errors
3. `edgequake-tasks/src/worker.rs` - Add exponential backoff

## Key Metrics

| File          | Lines | Purpose                |
| ------------- | ----- | ---------------------- |
| `pipeline.rs` | 1226  | Pipeline orchestration |
| `worker.rs`   | 310   | Task worker pool       |
| `error.rs`    | 51    | Error types            |
| `chunker.rs`  | 751   | Chunking strategies    |

## Dependencies

```
edgequake-pipeline
├── edgequake-llm (LLM traits)
├── edgequake-storage (Storage traits)
└── thiserror (Error handling)

edgequake-tasks
├── tokio (Async runtime)
├── num_cpus (Worker count)
└── chrono (Timestamps)
```
