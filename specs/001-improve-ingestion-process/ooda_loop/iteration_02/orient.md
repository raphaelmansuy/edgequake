# Iteration 02: Orient

**Mission Re-read**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-improve-ingestion-process.md`

---

## Gap Analysis

### Current State vs Desired State

| Aspect | Current | Desired | Gap |
|--------|---------|---------|-----|
| Status granularity | 2 states (processing, completed/failed) | 6 states (chunking, extracting, embedding, indexing, completed, failed) | 4 missing |
| Status storage | Final status only | Stage-by-stage | Need incremental updates |
| Frontend display | Shows "Processing" | Shows current stage | Already prepared ✅ |
| API response | Returns status | Returns status | ✅ Ready |

### Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Performance degradation from frequent KV updates | Medium | Low | Batch updates, async fire-and-forget |
| Status race conditions | Low | Medium | Document-level locking not needed (single processor per doc) |
| Backward compatibility | Low | Low | New states gracefully fallback in old clients |

---

## First Principles Analysis

### Why update status incrementally?

1. **User psychology**: Visible progress reduces anxiety
2. **Debugging**: Knowing where processing stopped helps identify issues
3. **Resource planning**: Understanding bottlenecks helps optimization

### What's the simplest change?

Modify `update_document_status()` to be called at each stage transition:
- Before chunking → status: "chunking"
- After chunking, before extraction → status: "extracting"
- After extraction, before embedding → status: "embedding"
- After embedding, before indexing → status: "indexing"
- After complete → status: "completed"
- On error → status: "failed"

---

## Solution Design

### Option A: Minimal Change (RECOMMENDED)

Add `update_document_status()` calls at each stage in `process_text_insert()`:

```rust
// Stage 1: Chunking
self.update_document_status(&document_id, "chunking", None).await?;
// ... chunking logic ...

// Stage 2: Extracting
self.update_document_status(&document_id, "extracting", None).await?;
// ... extraction logic ...

// Stage 3: Embedding
self.update_document_status(&document_id, "embedding", None).await?;
// ... embedding logic ...

// Stage 4: Indexing
self.update_document_status(&document_id, "indexing", None).await?;
// ... indexing logic ...

// Final: Completed
self.update_document_status_with_stats(&document_id, "completed", &stats).await?;
```

**Pros**: Simple, uses existing infrastructure
**Cons**: Adds 4 KV writes per document

### Option B: Progress Field Approach

Add separate `processing_stage` field instead of overloading `status`:

```rust
updated.insert("status".to_string(), json!("processing"));
updated.insert("processing_stage".to_string(), json!("chunking"));
```

**Pros**: Cleaner separation of concerns
**Cons**: Requires frontend changes, more complex

---

## Decision

**Selected: Option A (Minimal Change)**

Rationale:
1. Frontend already supports the new status values
2. Minimal code changes in backend
3. Backward compatible (old clients see "chunking" as unknown, fallback to "processing")
4. Performance impact is acceptable (4 extra KV writes ~40ms total)

---

## Implementation Plan

1. In `process_text_insert()`, add status updates:
   - Line ~598: Before chunking → "chunking"
   - Line ~621: After chunk storage → "extracting" 
   - Line ~625: After extraction → "embedding"
   - Line ~948: Before graph storage → "indexing"

2. Keep existing progress tracking for task-level monitoring

3. Test with Ollama model to verify stages are visible

---

## Next Step

Proceed to **Decide** phase to finalize implementation approach.
