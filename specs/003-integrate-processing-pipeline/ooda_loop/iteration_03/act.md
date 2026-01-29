# OODA Iteration 03: Act - Chunk Retry Queue Scaffolding

## Summary

Implemented chunk-level retry queue scaffolding with placeholder endpoints to enable frontend integration while full backend implementation is developed.

## Actions Taken

### Backend (Rust)

1. **DTOs Added** (`documents_types.rs`):
   - `RetryChunksRequest` - Request body with `chunk_indices`, `force`, `max_retries`
   - `RetryChunksResponse` - Response with `document_id`, `chunks_queued`, `implemented` flag
   - `FailedChunkInfo` - Detailed chunk failure information
   - `ListFailedChunksResponse` - List of failed chunks for a document

2. **Endpoints Added** (`documents.rs`):
   - `POST /documents/{document_id}/retry-chunks` - Accepts retry requests (placeholder)
   - `GET /documents/{document_id}/failed-chunks` - Lists failed chunks (placeholder)

3. **Routes Added** (`routes.rs`):
   - Registered both endpoints before the generic `{document_id}` catch-all

4. **Database Migration** (`021_add_failed_chunks_table.sql`):
   - Created `failed_chunks` table with:
     - `id`, `document_id`, `workspace_id`, `tenant_id`
     - `chunk_index`, `chunk_id`, `error_message`, `was_timeout`
     - `retry_attempts`, `status`, `created_at`, `updated_at`, `last_retry_at`
   - Indexes for efficient queries:
     - `idx_failed_chunks_document_id`
     - `idx_failed_chunks_workspace_pending`
     - `idx_failed_chunks_retry_schedule`

### Frontend (TypeScript)

1. **API Functions** (`edgequake.ts`):
   - `retryFailedChunks(documentId, chunkIndices, force)` → `RetryChunksResponse`
   - `listFailedChunks(documentId)` → `ListFailedChunksResponse`

2. **IngestionProgressPanel Integration**:
   - Added `isRetrying` state
   - Added `handleRetryChunks` callback using `retryFailedChunks` API
   - Wired `onRetry` and `isRetrying` props to `FailedChunksCard`

## Current Status

| Component | Status | Notes |
|-----------|--------|-------|
| Retry DTOs | ✅ Complete | Full type definitions |
| Retry Endpoints | ✅ Scaffolded | Return `implemented: false` |
| Database Migration | ✅ Complete | Table schema with indexes |
| Frontend API | ✅ Complete | Functions and types |
| Frontend UI Wiring | ✅ Complete | Retry button calls API |
| Full Retry Logic | ⏳ Pending | Requires chunk content storage |

## Commit

```
55d8d4c2 feat(api): add chunk retry queue scaffolding with placeholder endpoints
```

## Next Steps (for full implementation)

1. **Store chunk content** - Modify pipeline to store individual chunk text in KV storage or failed_chunks table
2. **Implement retry logic** - Query failed_chunks, retrieve content, re-run extraction
3. **Merge results** - Upsert extracted entities/relationships into graph
4. **Update status** - Mark chunks as succeeded or increment retry_attempts
5. **Wire to FailedChunksCard** - Show live status updates during retry

## Why Placeholder Approach

Full chunk-level retry requires significant infrastructure:
- Chunk content must be stored persistently (not just in memory)
- Extraction must be callable on individual chunks
- Results must merge with existing graph data

By implementing the scaffolding first:
1. Frontend integration can proceed immediately
2. API contract is defined and stable
3. Database schema is ready for data population
4. Backend implementation can follow without frontend changes
