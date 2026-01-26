# Iteration 04: ORIENT Phase

**Date:** 2025-01-26
**Focus:** Evaluating Race Condition Risks and Mitigations

## Risk Assessment Matrix

| Race Condition | Likelihood | Impact | Priority |
|---------------|------------|--------|----------|
| RACE-01: Same doc concurrent delete | Low | Low | P2 |
| RACE-02: Delete during processing | Low | Medium | P3 (mitigated) |
| RACE-03: Status check race | Very Low | Medium | P3 |
| RACE-04: Graph lost update | Medium | High | **P0** |
| GAP-06: Partial deletion | Low | High | **P1** |

## Risk Analysis

### RACE-01: Same Document Concurrent Delete (Low Priority)

**Why Low Priority:**
- Unlikely in practice (users rarely delete same doc twice simultaneously)
- Result is idempotent (doc gets deleted either way)
- Second request may get 404 or redundant success

**Mitigation Options:**
1. Add deletion lock (mutex per document_id)
2. Use optimistic concurrency (version check)
3. Accept as-is (document gets deleted)

**Recommendation:** Accept as-is, document eventual consistency in API docs

### RACE-04: Graph Lost Update (HIGH Priority)

**Why High Priority:**
- Common in multi-document workspaces
- Entities shared across many documents
- Concurrent deletions could corrupt source_ids
- Data integrity at risk

**Example Scenario:**
```
Initial State: ALICE entity → source_ids = ["doc-a-chunk-0", "doc-b-chunk-0"]

Thread A (deleting doc-a):                Thread B (deleting doc-b):
1. Read ALICE → sources = [a, b]          1. Read ALICE → sources = [a, b]
2. Filter → remaining = [b]               2. Filter → remaining = [a]
3. Write source_ids = [b]                 3. Write source_ids = [a]

Result: source_ids = [a] (Thread B wins)
Expected: source_ids = [] (entity should be deleted)
```

**Mitigation Options:**
1. **Test first** - Prove the race condition exists
2. Atomic compare-and-swap on source_ids
3. Deletion queue with sequential processing
4. Database-level transactions (PostgreSQL)

**Recommendation:** Add test to prove/disprove, then fix if proven

### GAP-06: Partial Deletion (HIGH Priority)

**Why High Priority:**
- If deletion fails mid-cascade, data is inconsistent
- No rollback mechanism
- Recovery requires manual intervention

**Mitigation Options:**
1. Wrap cascade in database transaction (PostgreSQL)
2. Two-phase delete: mark-then-sweep
3. Deletion queue with retry mechanism
4. Soft delete with background cleanup

**Recommendation:** For PostgreSQL, use transaction. For Memory, accept risk (testing only)

## Strategic Decision Framework

### Immediate Actions (This Iteration)

1. **Add concurrent deletion test** - Prove RACE-04 exists or doesn't
2. **Add idempotent deletion test** - Verify deleting deleted doc is safe

### Future Actions (Backlog)

1. PostgreSQL transaction wrapper for cascade delete
2. Document eventual consistency behavior
3. Add optional deletion lock for critical operations

## Test Design

### Test: Concurrent Deletion Race

```rust
#[tokio::test]
async fn test_concurrent_deletion_shared_entity() {
    // Create entity shared by doc_a and doc_b
    // Spawn two concurrent deletions
    // Verify entity is correctly deleted (or source_ids updated consistently)
}
```

### Test: Idempotent Deletion

```rust
#[tokio::test]
async fn test_delete_already_deleted_document() {
    // Create and delete document
    // Delete again
    // Verify 404 returned (expected behavior)
}
```

## Conclusion

RACE-04 is the most critical risk. We need to:
1. Add a test that attempts concurrent deletion
2. Observe behavior
3. If race is confirmed, implement fix in subsequent iteration

The test itself is valuable documentation of the system's behavior.
