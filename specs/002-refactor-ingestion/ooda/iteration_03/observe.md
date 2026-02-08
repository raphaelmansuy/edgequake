# OODA Iteration 03 - OBSERVE

## Issue

**Critical Issue #3**: Partial Extraction Failures Hidden (`pipeline.rs:800-850`)

From mission spec:

> 8/10 chunks succeed → "Completed" status, but 2 chunks failed silently
> Add `partial_success` status with chunk failure visibility

## Data Gathered

### Current Architecture

```text
┌──────────────────────┐     ┌────────────────────┐     ┌──────────────────┐
│ Pipeline::process()  │────▶│ extract_parallel() │────▶│ ProcessingStats  │
│ (pipeline.rs:878)    │     │ (pipeline.rs:393)  │     │   failed_chunks  │
└──────────────────────┘     └────────────────────┘     │   chunk_errors   │
         │                                              └──────────────────┘
         │                                                       │
         ▼                                                       │
┌──────────────────────┐     ┌────────────────────┐             │
│ upload_document()    │────▶│ Set status:        │◀────────────┘
│ (documents.rs:1196)  │     │ "completed"        │  ← ALWAYS "completed"
└──────────────────────┘     └────────────────────┘    even with failed_chunks!
```

### File Analysis

| File                                                                                          | Lines | Purpose                           |
| --------------------------------------------------------------------------------------------- | ----- | --------------------------------- |
| [pipeline.rs](../../../../edgequake/crates/edgequake-pipeline/src/pipeline.rs#L150-250)       | 2138  | ProcessingStats definition        |
| [error.rs](../../../../edgequake/crates/edgequake-pipeline/src/error.rs#L206-280)             | 392   | ResilientExtractionResult         |
| [documents.rs](../../../../edgequake/crates/edgequake-api/src/handlers/documents.rs#L937-960) | 4717  | Logs partial success but...       |
| [documents.rs](../../../../edgequake/crates/edgequake-api/src/handlers/documents.rs#L1196)    | 4717  | ...sets status "completed" anyway |

### The Bug: documents.rs:1196

```rust
// Line 937-959: Logs partial success if chunks failed
if result.stats.failed_chunks > 0 {
    warn!(
        document_id = %document_id,
        successful_chunks = result.stats.successful_chunks,
        failed_chunks = result.stats.failed_chunks,
        "Document processed with partial success"
    );
    // Broadcasts chunk failures via WebSocket
}

// ... later at line 1196 ...

// Status is ALWAYS "completed" - ignores failed_chunks!
let doc_metadata = serde_json::json!({
    "id": document_id,
    "status": "completed",  // ← BUG: should be "partial_success" when failed_chunks > 0
    ...
});
```

### Existing Infrastructure

**ProcessingStats** already tracks:

- `chunk_count: usize` - total chunks
- `successful_chunks: usize` - chunks that worked
- `failed_chunks: usize` - chunks that failed (the key field)
- `chunk_errors: Option<Vec<ChunkErrorInfo>>` - detailed error per chunk

**ResilientExtractionResult** has:

- `is_complete_success()` - all chunks succeeded
- `has_any_success()` - at least one succeeded
- `is_complete_failure()` - all chunks failed
- `success_rate()` - percentage 0.0-1.0

**WebSocket already broadcasts** chunk failures:

```rust
state.progress_broadcaster.broadcast_chunk_failure(...)
```

### Frontend Status Handling

**EnhancedStatusBadge** (document-manager.tsx):

```typescript
const statusConfig = {
  completed: { variant: "success", label: "Completed" },
  failed: { variant: "destructive", label: "Failed" },
  // No "partial_success" defined!
};
```

### User Impact

| Scenario             | Current Behavior      | Expected Behavior         |
| -------------------- | --------------------- | ------------------------- |
| 10/10 chunks succeed | Status: "completed" ✓ | Status: "completed" ✓     |
| 8/10 chunks succeed  | Status: "completed" ✗ | Status: "partial_success" |
| 0/10 chunks succeed  | Status: "failed" ✓    | Status: "failed" ✓        |

Users see "Completed" thinking document is fully processed, but 20% of content may be missing from the knowledge graph.

### API Contract Consideration

Adding `partial_success` status is **backwards compatible**:

- Existing code checks for `status == "completed"` will still work
- New clients can handle `partial_success` for better UX
- No breaking changes to existing workflows

## Root Cause

Line 1196 in `documents.rs` unconditionally sets `status: "completed"` without checking if `result.stats.failed_chunks > 0`.

## Key Observations

1. **Stats already tracked** - `failed_chunks` is already computed and logged
2. **WebSocket already broadcasts** chunk failures - but document status lies
3. **Frontend needs update** - EnhancedStatusBadge needs `partial_success` variant
4. **Document metadata needs** `failed_chunks` and `successful_chunks` fields for UI display
