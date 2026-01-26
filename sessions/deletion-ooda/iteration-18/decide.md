# OODA-18 Decide: Reprocessing Edge Case Tests

## Decision

Add test to verify that PROCESSING documents are not included in reprocess batch.

## Implementation

### Test: Processing Documents Excluded from Reprocess

Location: `edgequake/crates/edgequake-api/tests/e2e_document_deletion.rs`

The test will verify:
1. Create a document with `status: "processing"`
2. Call `/api/v1/documents/reprocess`
3. Verify document is NOT requeued
4. Verify no cleanup happened (document still processing)

### Why This Matters

```
Scenario WITHOUT protection:
  T1: Document A is processing (50% complete, entities X, Y created)
  T2: User calls reprocess endpoint
  T3: reprocess_failed cleans up X, Y
  T4: Document A continues processing → ERROR or duplicate entities

Scenario WITH protection:
  T1: Document A is processing
  T2: User calls reprocess endpoint
  T3: Document A ignored (status != failed)
  T4: Document A completes normally
```

## Acceptance Criteria

- [ ] Test verifies PROCESSING documents are excluded from reprocess
- [ ] Test passes
- [ ] Existing tests still pass

## Files to Modify

1. `edgequake/crates/edgequake-api/tests/e2e_document_deletion.rs`
