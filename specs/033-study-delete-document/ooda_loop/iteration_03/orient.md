# ITERATION 03 - ORIENT

**Date**: 2026-01-26  
**Mission**: Study document add/delete process on EdgeQuake (specs/033-study-delete-document/003-study-document.md)  
**Focus**: PostgreSQL provider verification + reprocessing mechanism enhancement

---

## Mission Re-Read ✅

From mission file:
> "Ensure it working with postgres provider and memory provider for all storage layers (KV, Vector, Graph)."
> "Ensure there is reprocessing mechanism for failed documents."
> "Ensure deleting a failed document cleans up all partial data."

---

## Gap Analysis

### GAP-A: Reprocessing Doesn't Clean Partial Data

**Problem**:
When `reprocess_failed` is called, it requeues the document for processing WITHOUT cleaning up partial data from the failed attempt.

**Risk**:
```
Failed Processing → Creates entities A, B (partial)
Reprocess → Creates entities A, B, C (new attempt)
Result → Duplicate entities OR inconsistent source_ids
```

**First Principles Analysis**:
- Reprocessing SHOULD be idempotent
- Each processing attempt should start with a clean slate
- Entity deduplication is LLM-dependent (names may vary between attempts)

**Root Cause**:
- `reprocess_failed` doesn't invoke any cleanup logic
- Just updates status and creates new task
- Assumes processing will handle duplicates (it might not)

**Solution Options**:

1. **Clean Before Reprocess (Recommended)**
   - Before updating status to "pending", call cleanup logic
   - Remove all entities/edges that reference this document
   - Remove embeddings for those entities
   - Then requeue for fresh processing

2. **Mark as "reprocessing" and Merge**
   - Set special status during reprocess
   - During processing, merge with existing entities
   - More complex, harder to verify correctness

3. **Delete and Re-upload**
   - Require user to delete failed document first
   - Then upload fresh
   - Simple but poor UX

**Decision**: Option 1 - Clean Before Reprocess

---

### GAP-B: No Tests for Partial Data Cleanup

**Problem**:
Current test `test_delete_failed_document_allowed` only verifies deletion returns 200 OK. It doesn't verify partial entities/edges are actually cleaned up.

**Risk**:
- Deletion might succeed but leave orphaned data
- Can't prove mission requirement: "deleting a failed document cleans up all partial data"

**Solution**:
Add comprehensive test that:
1. Manually creates partial entities/edges (simulating failed processing)
2. Creates document with "failed" status
3. Links entities to document via source_ids
4. Deletes document
5. Verifies all linked entities are removed
6. Verifies no orphaned edges remain

---

### GAP-C: PostgreSQL Provider Verification

**Current State**:
- All tests use Memory provider (via `AppState::test_state()`)
- PostgreSQL implementation looks correct (DETACH DELETE)
- But not verified with actual PostgreSQL

**Risk**:
- Memory and PostgreSQL may have subtle behavioral differences
- Status check works with Memory, might fail with PostgreSQL

**Solution Options**:

1. **Integration Test with PostgreSQL**
   - Requires running PostgreSQL with AGE extension
   - More realistic but harder to set up in CI

2. **Feature Flag for Provider Selection**
   - Tests can specify which provider to use
   - Run same tests against both providers

3. **Document Expected Behavior**
   - Trust PostgreSQL implementation (code review)
   - Document differences between providers
   - Manual verification in staging environment

**Decision**: Option 3 for now (Document + Manual Verify), Option 1 for future

---

## Solution Design

### Solution 1: Add Cleanup to Reprocessing

**Implementation**:

```rust
// In reprocess_failed function (documents.rs:2842)
async fn reprocess_failed(/* ... */) -> ApiResult<Json<ReprocessFailedResponse>> {
    // ... find failed documents ...

    for (doc_id, _doc_key) in &failed_docs {
        // NEW: Clean up partial data before requeueing
        cleanup_partial_processing_data(&state, &doc_id, &workspace_id_for_storage).await?;
        
        // ... existing requeue logic ...
    }
}

// New helper function
async fn cleanup_partial_processing_data(
    state: &AppState,
    document_id: &str,
    workspace_id: &str,
) -> Result<(), ApiError> {
    // 1. Find all entities that reference this document
    // 2. For each entity: remove document from source_ids
    // 3. If source_ids becomes empty: delete entity
    // 4. Clean up orphaned edges
    // 5. Delete entity embeddings if entity was deleted
    
    // This is essentially the cascade logic from delete_document,
    // but without deleting the document metadata/content
}
```

**Complexity**: MEDIUM
**Risk**: LOW (reuses existing cascade logic)
**Value**: HIGH (ensures clean reprocessing)

---

### Solution 2: Add Comprehensive Partial Data Test

**Implementation**:

```rust
#[tokio::test]
async fn test_delete_failed_document_cleans_partial_data() {
    let state = AppState::test_state();
    let server = Server::new(create_test_config(), state.clone());
    let app = server.build_router();
    
    let doc_id = "test-partial-failed-doc";
    
    // 1. Manually create partial entities that reference this document
    let entity_a = GraphNode {
        id: "ENTITY_A".to_string(),
        properties: hashmap!{
            "source_ids" => json!([format!("{}-chunk-0", doc_id)]),
            "entity_type" => json!("PERSON"),
        },
    };
    state.graph_storage.upsert_node("ENTITY_A", entity_a.properties).await?;
    
    // 2. Create document with "failed" status
    let metadata = json!({
        "id": doc_id,
        "status": "failed",
        "workspace_id": "default"
    });
    state.kv_storage.upsert(&[(format!("{}-metadata", doc_id), metadata)]).await?;
    
    // 3. Verify entity exists before deletion
    let nodes_before = state.graph_storage.get_all_nodes().await?;
    assert!(nodes_before.iter().any(|n| n.id == "ENTITY_A"));
    
    // 4. Delete the failed document
    let (status, _) = delete_document_http(&app, doc_id).await;
    assert_eq!(status, StatusCode::OK);
    
    // 5. Verify entity was cleaned up
    let nodes_after = state.graph_storage.get_all_nodes().await?;
    assert!(!nodes_after.iter().any(|n| n.id == "ENTITY_A"),
        "Partial entity should be cleaned up when failed document is deleted");
}
```

**Complexity**: LOW
**Risk**: NONE (test only)
**Value**: HIGH (proves mission requirement)

---

## Priority Matrix

| Solution | Impact | Effort | Risk | Priority |
|----------|--------|--------|------|----------|
| Partial data test | HIGH | LOW | NONE | P0 |
| Cleanup in reprocess | HIGH | MEDIUM | LOW | P1 |
| PostgreSQL test setup | MEDIUM | HIGH | LOW | P2 (future) |
| Provider documentation | MEDIUM | LOW | NONE | P1 |

---

## Risk Assessment

### Risk: Cleanup Logic Duplication

**Issue**: If we add cleanup to reprocess_failed, we'll have similar code in two places (delete_document and reprocess_failed).

**Mitigation**: Extract cleanup logic into reusable helper function.

### Risk: Cleanup Fails Mid-Way

**Issue**: If cleanup fails, document might be left in inconsistent state.

**Mitigation**: 
- Log cleanup failures but continue to requeue
- Document status helps identify issues
- Can always manually delete and re-upload

---

## Success Criteria

1. ✅ Test proves failed document deletion cleans partial data
2. ✅ Reprocess endpoint cleans partial data before requeueing
3. ✅ Documentation explains provider differences
4. ✅ All existing tests still pass

---

**Status**: ORIENT COMPLETE ✅  
**Next**: Create DECIDE with specific implementation plan
