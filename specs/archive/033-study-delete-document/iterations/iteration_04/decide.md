# Iteration 04: DECIDE Phase

**Date:** 2025-01-26
**Focus:** Concurrent Deletion Testing

## Decision

### CHANGE-IT04-01: Add Concurrent Deletion Test

**Priority:** P0 - Critical for understanding system behavior

**Description:** Add test that attempts concurrent deletion of two documents that share an entity. This will prove whether RACE-04 (lost update) exists.

**Test Design:**

1. Create shared entity with source_ids referencing both documents
2. Create metadata for both documents
3. Spawn two concurrent delete requests using `tokio::join!`
4. Verify the entity is correctly handled:
   - Either deleted (if both sources removed)
   - Or has consistent source_ids (one source remaining)

**Expected Outcomes:**

- **If test passes:** Concurrent deletion is safe (possibly by accident)
- **If test fails:** RACE-04 confirmed, needs fix

### CHANGE-IT04-02: Add Idempotent Deletion Test

**Priority:** P1 - API contract validation

**Description:** Verify that deleting an already-deleted document returns 404.

**Test Design:**

1. Create document
2. Delete document → expect 200
3. Delete again → expect 404

### CHANGE-IT04-03: Add Deletion During Insertion Test

**Priority:** P2 - Edge case validation

**Description:** Test what happens when deletion is attempted while a new document is being inserted with same ID.

**Deferred:** This is a complex scenario that requires careful test setup.

## Implementation Plan

```
1. Add test_concurrent_deletion_of_shared_entity
2. Add test_idempotent_deletion_returns_404
3. Run tests
4. Document results in act.md
5. If RACE-04 confirmed → plan fix for iteration_05
```

## Acceptance Criteria

- [ ] Concurrent deletion test runs without panic
- [ ] Idempotent deletion test passes (404 on second delete)
- [ ] Results documented with observations
- [ ] If race detected, GAP-07 added to registry
