# ITERATION 02 - ORIENT

**Date**: 2026-01-26  
**Mission**: Study document add/delete process on EdgeQuake (specs/033-study-delete-document/003-study-document.md)  
**Focus**: Fix integration tests + ensure perfect safety for partially processed/failed document deletion

---

## Mission Re-Read ✅

Mission file: `specs/033-study-delete-document/003-study-document.md`

**Key Requirements**:

1. "perfect safety when deleting documents that are partially processed"
2. "in the middle of processing"
3. "failed processing"
4. "No dangling data must remain"
5. "No shared data must be deleted"
6. "Comprehensive test coverage"
7. "Comprehensive Edge cases must be implemented in tests"

---

## Gap Analysis from OBSERVE Phase

### GAP-A: Broken Test Environment (Critical for Verification)

**Root Cause**:

```rust
// BROKEN: Direct handler calls bypass pipeline
let result = edgequake_api::handlers::documents::upload_document(
    axum::extract::State(state.clone()),
    // ...
);
// Result: entity_count = 0 (pipeline never runs)
```

**Working Pattern (from e2e_documents.rs)**:

```rust
// WORKING: HTTP router initializes full stack
let app = Server::new(config, state).build_router();
let response = app.oneshot(Request::builder()...).await;
// Result: entity_count > 0 (pipeline executes)
```

**Analysis**:

- `Server::new().build_router()` creates layers that properly configure:
  1. Pipeline with mock LLM provider
  2. Entity extraction middleware
  3. Full async runtime context
- Direct handler calls skip these layers → entities never extracted

**First Principles Solution**:

- Fix tests to use HTTP router pattern (like e2e_documents.rs)
- This is the correct approach because tests should exercise the full stack
- Direct handler calls are unit tests, not integration tests

---

### GAP-B: Race Condition Between Processing and Deletion

**Problem from OBSERVE**:

```
T0: Document uploaded (async)
T1: Background processing ACTIVELY running
T2: User calls delete_document
    → Reads entity list (gets 5 entities)
    → Starts deleting entities
T3: Background processor continues
    → Creates entity #6
    → Entity #6 is orphaned!
```

**Current State Analysis**:

- `delete_document` does NOT check document status before deletion
- No locking mechanism between processing and deletion
- Background tasks continue even after deletion starts

**First Principles Solution Options**:

1. **Status-Based Blocking (Simple)**:
   - Add status check before deletion: reject if "pending" or "processing"
   - Force user to cancel/wait before deleting
   - ✅ Simple, ❌ Poor UX (user must wait)

2. **Cancellation Token Pattern (Better)**:
   - Cancel background task before deletion
   - Wait for cancellation to complete
   - ✅ Clean, ❌ More complex implementation

3. **Status Transition Lock (Best)**:
   - Atomic status update to "deleting"
   - Background processor checks status before each step
   - If "deleting", abort processing and clean up
   - ✅ Safe, ✅ Good UX, ⚠️ Most complex

**Recommended**: Option 1 (Status-Based Blocking) for initial implementation:

- Simple to implement and verify
- Can enhance to Option 3 later if needed
- Meets mission requirement: "perfect safety"

---

### GAP-C: Missing Tests for Safety Scenarios

**Required Test Scenarios (from mission)**:

| Scenario                                  | Current Coverage      | Priority      |
| ----------------------------------------- | --------------------- | ------------- |
| Delete document with status="pending"     | ❌ Not tested         | HIGH          |
| Delete document with status="processing"  | ❌ Not tested         | CRITICAL      |
| Delete document with status="failed"      | ❌ Not tested         | HIGH          |
| Concurrent processing + deletion          | ❌ Not tested         | CRITICAL      |
| Partial entity cleanup on failed delete   | ❌ Not tested         | MEDIUM        |
| Multi-document shared entity (GAP-03 fix) | ⚠️ Failing (test env) | FIXED IN IT01 |

---

## Solution Design Matrix

| Solution                                          | Impact | Effort      | Risk   | Priority    |
| ------------------------------------------------- | ------ | ----------- | ------ | ----------- |
| Fix test environment (HTTP pattern)               | HIGH   | LOW (2h)    | LOW    | P0          |
| Add status check before deletion                  | HIGH   | MEDIUM (4h) | LOW    | P1          |
| Add "deleting" status with atomic transition      | MEDIUM | HIGH (8h)   | MEDIUM | P2 (future) |
| Add cancellation token for background tasks       | MEDIUM | HIGH (6h)   | MEDIUM | P3 (future) |
| Add tests for pending/processing/failed scenarios | HIGH   | MEDIUM (4h) | LOW    | P1          |

---

## Risk Assessment

### Risk 1: Deleting "processing" document causes orphaned data

**Probability**: HIGH (if not fixed)
**Impact**: HIGH (data corruption)
**Mitigation**: Status check before deletion (Option 1)

### Risk 2: Tests using wrong pattern don't actually verify behavior

**Probability**: HIGH (current state)
**Impact**: HIGH (false confidence)
**Mitigation**: Fix tests to use HTTP router pattern

### Risk 3: Status check blocks legitimate deletion requests

**Probability**: LOW
**Impact**: MEDIUM (UX degradation)
**Mitigation**:

- Clear error message explaining why deletion blocked
- Document how to wait for processing completion
- Future: Add cancel endpoint to stop processing

---

## Architecture Decision Records

### ADR-001: Test Pattern for Integration Tests

**Context**: Tests calling handlers directly bypass pipeline and don't execute entity extraction.

**Decision**: Use HTTP router pattern (`Server::new().build_router()`) for all integration tests that need to verify pipeline behavior.

**Consequences**:

- Tests execute full stack including middleware
- Entity extraction runs with mock LLM
- Tests are more realistic but slightly slower
- Unit tests can still call handlers directly for isolated logic

### ADR-002: Status Check Before Deletion

**Context**: Need to prevent race condition between processing and deletion.

**Decision**: Add status validation in `delete_document`:

- Reject deletion if status is "pending" (not yet started)
- Reject deletion if status is "processing" (actively running)
- Allow deletion if status is "completed", "failed", or "deleting"

**Consequences**:

- Prevents data corruption from concurrent operations
- User must wait for processing to complete before deleting
- Clear error messages guide user behavior
- Can be enhanced later with cancellation support

---

## Success Criteria for ITERATION 02

1. **All 5 deletion tests pass** (currently 2/5)
2. **Status validation prevents unsafe deletion** (new feature)
3. **Clear documentation** of status lifecycle
4. **Metrics prove safety** (no orphaned data in tests)

---

**Status**: ORIENT COMPLETE ✅  
**Next**: Create DECIDE document with specific implementation plan
