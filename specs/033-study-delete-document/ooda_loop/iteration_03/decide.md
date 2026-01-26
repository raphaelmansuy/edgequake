# ITERATION 03 - DECIDE

**Date**: 2026-01-26  
**Mission**: Study document add/delete process on EdgeQuake (specs/033-study-delete-document/003-study-document.md)  
**Focus**: Add partial data cleanup test + enhance reprocessing

---

## Mission Re-Read ✅

From mission file:

> "Ensure there is reprocessing mechanism for failed documents."  
> "Ensure deleting a failed document cleans up all partial data."

---

## Selected Changes for ITERATION 03

### ✅ CHANGE-IT03-01: Add Test for Partial Data Cleanup

**Priority**: P0 - CRITICAL (Mission requirement verification)

**Problem**:
Current test only verifies deletion returns 200 OK, not that partial data is actually cleaned up.

**Solution**:
Add test that:

1. Creates partial entities/edges manually
2. Links them to a document via source_ids
3. Creates document with "failed" status
4. Deletes document
5. Verifies entities with only this document as source are removed
6. Verifies entities with multiple sources are preserved (reference counting)

**Implementation Plan**:

1. Add new test `test_delete_failed_document_cleans_partial_data`
2. Create graph entities with source_ids pointing to test document
3. Delete document via HTTP API
4. Assert entities were cleaned up

**Files to Modify**:

- `edgequake/crates/edgequake-api/tests/e2e_document_deletion.rs`

**Acceptance Criteria**:

- ✅ Test creates partial entities linked to document
- ✅ Test verifies entities are removed after deletion
- ✅ Test passes, proving mission requirement

**Estimated Effort**: 1 hour
**Risk**: LOW

---

### ✅ CHANGE-IT03-02: Add Cleanup Logic to Reprocess Endpoint

**Priority**: P1 - HIGH (Data integrity)

**Problem**:
`reprocess_failed` requeues documents without cleaning up partial data from failed attempt.

**Solution**:

1. Extract cascade cleanup logic into reusable helper function
2. Call cleanup before requeueing in `reprocess_failed`
3. Same for `recover_stuck`

**Implementation Plan**:

1. Create `cleanup_document_graph_data()` helper function
   - Takes document_id as input
   - Removes document from entity source_ids
   - Deletes entities with empty source_ids
   - Cleans up orphaned edges
   - (Does NOT delete document metadata/content)

2. Call this helper at start of reprocess loop
3. Log cleanup metrics

**Files to Modify**:

- `edgequake/crates/edgequake-api/src/handlers/documents.rs`

**Acceptance Criteria**:

- ✅ Helper function extracts common logic
- ✅ reprocess_failed cleans up before requeueing
- ✅ recover_stuck cleans up before requeueing
- ✅ All tests pass

**Estimated Effort**: 2 hours
**Risk**: MEDIUM (refactoring existing code)

---

### ✅ CHANGE-IT03-03: Update Summary Documentation

**Priority**: P2 - DOCUMENTATION

**Solution**:
Update summary.md with:

1. ITERATION 03 findings
2. Reprocessing mechanism documentation
3. Provider behavior differences
4. Complete safety verification status

**Files to Modify**:

- `specs/033-study-delete-document/docs/summary.md`

**Estimated Effort**: 30 minutes
**Risk**: NONE

---

## Deferred to Future Iterations

### ⏸️ DEFERRED: PostgreSQL Integration Tests

**Reason**:

- Requires PostgreSQL with AGE extension in CI
- Current Memory provider tests are sufficient for logic verification
- PostgreSQL behavior verified via code review

**When**: When CI infrastructure supports PostgreSQL with AGE

### ⏸️ DEFERRED: Query-by-Property API

**Reason**:

- Performance optimization, not correctness fix
- Current O(N) scan works for moderate graph sizes
- Can address when performance becomes an issue

**When**: ITERATION 04+ when graph sizes exceed 100K nodes

---

## Implementation Order

1. **CHANGE-IT03-01**: Add partial data cleanup test
2. **CHANGE-IT03-02**: Add cleanup to reprocess endpoints
3. **CHANGE-IT03-03**: Update documentation

---

## Commit Plan

```
OODA-03-01: Add test for partial data cleanup on failed document deletion
OODA-03-02: Add cleanup logic to reprocess and recover endpoints
OODA-03-03: Update summary documentation
```

---

**Status**: DECIDE COMPLETE ✅  
**Next**: Implement changes in ACT phase
