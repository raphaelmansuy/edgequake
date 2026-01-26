# OODA-18 Act: Reprocessing Edge Case Tests

## Implementation

Added 2 new tests to `e2e_document_deletion.rs`:

### Test 1: `test_reprocess_excludes_processing_documents`
- Creates document with `status: "processing"` and recent timestamp
- Creates entity belonging to that document
- Calls `/api/v1/documents/reprocess`
- Verifies:
  - Document NOT included in reprocess batch (requeued_count = 0)
  - Entity NOT cleaned up (document still processing)

### Test 2: `test_reprocess_cleans_all_entities_and_relationships`
- Creates FAILED document with 3 entities and 2 relationships
- Calls `/api/v1/documents/reprocess`
- Verifies:
  - ALL 3 entities cleaned up
  - ALL 2 relationships cleaned up
  - Graph is empty after cleanup

## Test Results

```
running 4 tests
test test_reprocess_cleans_partial_graph_data ... ok
test test_reprocess_preserves_shared_entities ... ok
test test_reprocess_excludes_processing_documents ... ok
test test_reprocess_cleans_all_entities_and_relationships ... ok

test result: ok. 4 passed; 0 failed; 0 ignored
```

Full suite: 27 tests pass.

## Safety Verification

The tests verify the current implementation correctly:
1. Only FAILED documents are included in reprocess batch
2. PROCESSING documents are excluded (protected from accidental cleanup)
3. All entities AND relationships are cleaned for failed documents
4. Cleanup happens before requeueing

## Next Iteration

OODA-19: Consider adding:
- Concurrent reprocess request test
- Idempotent reprocess test
- Reprocess with changed content test
