# OODA Iteration 15 - Decide

## Selected Implementation: Add Circular Reference Safety Tests

### Tests to Add

1. **test_deletion_with_bidirectional_relationships**
   - Create doc with ALICE and BOB entities
   - Add A→B and B→A relationships
   - Delete document
   - Verify both entities and both relationships deleted
   - Verify no infinite loop (test completes)

2. **test_deletion_with_self_referential_entity**
   - Create doc with RECURSIVE_NODE entity
   - Add self-reference relationship: RECURSIVE_NODE → RECURSIVE_NODE
   - Delete document
   - Verify entity and relationship deleted

3. **test_deletion_with_cycle_preserves_shared**
   - Create doc1 with ALICE
   - Create doc2 with BOB, CHARLIE
   - Add cycle: ALICE → BOB → CHARLIE → ALICE
   - Delete doc1
   - Verify ALICE deleted, BOB and CHARLIE preserved
   - Verify relationships to/from ALICE deleted

### File to Modify

`edgequake/crates/edgequake-api/tests/e2e_document_deletion.rs`

### Acceptance Criteria

- [ ] All 3 new tests pass
- [ ] No infinite loops or timeouts
- [ ] Tests complete in reasonable time (<1s each)
- [ ] Existing 22 tests still pass
