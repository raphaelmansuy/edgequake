# OODA-40: Orient + Decide

## Analysis

Hash deduplication tests:
1. Same content uploaded twice → can verify hashes match
2. Delete one, other should remain (if different documents)
3. Verify hash consistency

## Action Plan

Add 2 tests:
1. `test_content_hash_consistency` - Same content = same hash
2. `test_delete_does_not_affect_same_content_other_doc` - Dedup behavior

## Success Criteria

- Tests pass
- Total deletion tests: 54
