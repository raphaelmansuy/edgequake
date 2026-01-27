# OODA Iteration 11 – DECIDE

**Objective:** Implementation plan for stress tests

---

## Decision

Add a single high-volume stress test to the existing `e2e_document_deletion.rs` file. This test will:

1. Create 15 documents
2. Create 5 entities shared across overlapping document groups
3. Delete 10 documents concurrently
4. Verify remaining 5 documents' entities are correctly preserved
5. Delete remaining 5 documents
6. Verify all entities are deleted

## Implementation

### Test Name

`test_high_volume_concurrent_deletions_stress`

### Test Structure

```rust
#[tokio::test]
async fn test_high_volume_concurrent_deletions_stress() {
    // 1. Setup: Create 15 documents with 5 shared entities
    // 2. Phase 1: Delete 10 concurrently
    // 3. Verify: 5 entities should be preserved (still have references)
    // 4. Phase 2: Delete remaining 5
    // 5. Verify: All entities should be deleted
}
```

### Entity Distribution

- Entity 1: Docs 1-5 (5 refs)
- Entity 2: Docs 3-8 (6 refs)
- Entity 3: Docs 6-11 (6 refs)
- Entity 4: Docs 9-13 (5 refs)
- Entity 5: Docs 11-15 (5 refs)

After deleting docs 1-10:

- Entity 1: Should be deleted (all refs gone)
- Entity 2: Should be deleted (all refs gone)
- Entity 3: Doc 11 remains (1 ref)
- Entity 4: Docs 11-13 remain (3 refs)
- Entity 5: Docs 11-15 remain (5 refs)

---

## Files to Modify

| File                             | Action   |
| -------------------------------- | -------- |
| `tests/e2e_document_deletion.rs` | ADD test |

---

## Success Criteria

1. All 15 documents created successfully
2. 10 concurrent deletions complete without errors
3. Correct entities preserved after phase 1
4. All entities deleted after phase 2
5. No panics or deadlocks
