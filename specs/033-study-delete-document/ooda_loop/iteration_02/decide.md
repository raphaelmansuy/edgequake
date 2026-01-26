# ITERATION 02 - DECIDE

**Date**: 2026-01-26  
**Mission**: Study document add/delete process on EdgeQuake (specs/033-study-delete-document/003-study-document.md)  
**Focus**: Fix integration tests + ensure perfect safety for partially processed/failed document deletion

---

## Mission Re-Read ✅

From mission file:
> "You must ensure and prove perfect safety when deleting documents that are partially processed, in the middle of processing, or failed processing."

---

## Selected Changes for ITERATION 02

### ✅ CHANGE-IT02-01: Fix Test Environment to Use HTTP Router Pattern

**Priority**: P0 - CRITICAL (Unblocks verification)

**Problem**:
Tests call handlers directly → pipeline doesn't execute → entity_count = 0 → tests fail

**Solution**:
Rewrite `e2e_document_deletion.rs` to use the same pattern as `e2e_documents.rs`:
- Use `Server::new(config, state).build_router()` 
- Make HTTP requests via `app.oneshot()`
- Parse JSON responses

**Implementation Plan**:
1. Add helper functions matching e2e_documents.rs pattern
2. Rewrite test cases to use HTTP requests
3. Remove direct handler calls
4. Verify all 5 tests pass

**Files to Modify**:
- `edgequake/crates/edgequake-api/tests/e2e_document_deletion.rs`

**Acceptance Criteria**:
- ✅ All 5 deletion tests pass
- ✅ entity_count > 0 after upload
- ✅ Tests verify cascade deletion behavior
- ✅ No direct handler calls in integration tests

**Estimated Effort**: 2 hours
**Risk**: LOW

---

### ✅ CHANGE-IT02-02: Add Status Validation Before Deletion

**Priority**: P1 - HIGH (Safety requirement)

**Problem**:
`delete_document` doesn't check document status → can delete during processing → orphaned data

**Solution**:
Add status check at the beginning of `delete_document`:
```rust
// Check document status before deletion
let status = metadata.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
match status {
    "pending" => return Err(ApiError::Conflict(
        "Cannot delete document that is pending processing. Wait for processing to complete or cancel the task.".to_string()
    )),
    "processing" => return Err(ApiError::Conflict(
        "Cannot delete document that is currently processing. Wait for processing to complete or cancel the task.".to_string()
    )),
    "completed" | "processed" | "failed" | "deleting" | "unknown" => {
        // OK to delete
    }
    other => {
        tracing::warn!(status = %other, "Unknown document status, allowing deletion");
    }
}
```

**Implementation Plan**:
1. Read metadata at start of `delete_document`
2. Extract status field
3. Return 409 Conflict for "pending" or "processing"
4. Allow deletion for other statuses
5. Add WHY comment explaining safety requirement

**Files to Modify**:
- `edgequake/crates/edgequake-api/src/handlers/documents.rs`

**Acceptance Criteria**:
- ✅ 409 Conflict returned for "pending" documents
- ✅ 409 Conflict returned for "processing" documents
- ✅ Successful deletion for "completed", "failed" documents
- ✅ Clear error messages guide user
- ✅ Tests verify status validation

**Estimated Effort**: 2 hours
**Risk**: LOW

---

### ✅ CHANGE-IT02-03: Add Safety Test Cases

**Priority**: P1 - HIGH (Mission requirement)

**Problem**:
No tests verify deletion safety for different document states

**Solution**:
Add new test cases:
1. `test_delete_pending_document_rejected` - Cannot delete pending
2. `test_delete_processing_document_rejected` - Cannot delete processing
3. `test_delete_failed_document_allowed` - Can delete failed
4. `test_delete_completed_document_allowed` - Can delete completed

**Files to Modify**:
- `edgequake/crates/edgequake-api/tests/e2e_document_deletion.rs`

**Acceptance Criteria**:
- ✅ 4 new tests added
- ✅ All tests pass
- ✅ Tests cover mission safety requirements
- ✅ Clear test names indicate purpose

**Estimated Effort**: 2 hours
**Risk**: LOW

---

### ✅ CHANGE-IT02-04: Document Status Lifecycle

**Priority**: P2 - DOCUMENTATION

**Solution**:
Add ASCII diagram and documentation explaining status lifecycle

```ascii
┌─────────────────────────────────────────────────────────┐
│              Document Status Lifecycle                   │
└─────────────────────────────────────────────────────────┘

Upload (async=true)         Upload (async=false)
       │                           │
       ▼                           ▼
   "pending"                  "processing"
       │                           │
       ▼                           ▼
  "processing"                "processed"/"completed"
       │                           │
       ├───────────────────────────┤
       ▼                           ▼
   "failed"                  "completed"
       │                           │
       └───────────────────────────┘
                   │
                   ▼
            DELETE allowed
                   │
                   ▼
             "deleting" (future)
                   │
                   ▼
              [removed]

DELETION RULES:
- "pending"    → 409 Conflict (wait or cancel)
- "processing" → 409 Conflict (wait or cancel)  
- "completed"  → 200 OK (cascade delete)
- "processed"  → 200 OK (legacy, same as completed)
- "failed"     → 200 OK (cleanup partial data)
```

**Files to Create/Modify**:
- `edgequake/crates/edgequake-api/src/handlers/documents.rs` (WHY comments)
- `specs/033-study-delete-document/docs/status-lifecycle.md` (new)

**Estimated Effort**: 1 hour
**Risk**: NONE

---

## Deferred to Future Iterations

### ⏸️ DEFERRED: Cancellation Token for Background Tasks

**Reason**: More complex implementation, not needed for MVP safety
**When**: ITERATION 03+

### ⏸️ DEFERRED: Atomic "deleting" Status Transition

**Reason**: Current status check provides sufficient safety
**When**: If race conditions observed in production

### ⏸️ DEFERRED: Query-by-Property API for Performance

**Reason**: Focus on correctness first (from ITERATION 01 DECIDE)
**When**: ITERATION 03+

---

## Implementation Order

1. **CHANGE-IT02-01**: Fix tests (unblocks verification)
2. **CHANGE-IT02-02**: Add status validation (safety)
3. **CHANGE-IT02-03**: Add safety tests (verification)
4. **CHANGE-IT02-04**: Documentation (completeness)

---

## Commit Plan

```
OODA-02-01: Fix deletion tests to use HTTP router pattern
OODA-02-02: Add status validation before document deletion
OODA-02-03: Add safety tests for pending/processing/failed deletion
OODA-02-04: Document status lifecycle
```

---

**Status**: DECIDE COMPLETE ✅  
**Next**: Implement changes in ACT phase
