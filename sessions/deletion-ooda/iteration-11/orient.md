# OODA Iteration 11 – ORIENT

**Objective:** Design stress tests for high volume concurrent deletion

---

## Current State

Existing `test_multiple_concurrent_deletions` uses 5 documents with 3 shared entities. This is a moderate test but not stress level.

## Proposed Stress Tests

### Test 1: High Volume Concurrent Deletions

- 20 documents with overlapping entities
- 10 concurrent deletions
- Verify no race conditions or data corruption

### Test 2: Deletion Under Load 

- Upload documents rapidly
- Start deleting before all uploads complete
- Verify system handles mixed upload/delete operations

### Test 3: Large Entity Network

- 10 documents each sharing 5 entities with next document
- Creates complex entity relationship web
- Test that deletion correctly updates all shared references

## Implementation Approach

Add new test `test_high_volume_concurrent_deletions` that:
1. Creates 20 documents with 10 shared entities
2. Deletes 15 documents concurrently using `tokio::join!`
3. Verifies remaining entities have correct source_ids
4. Verifies deleted documents' chunks are removed

## Risk Assessment

- **Memory**: 20 documents with mock provider should be fine
- **PostgreSQL**: May need separate PG stress test
- **Time**: Concurrent tests are fast (no real LLM calls)

---

## Next Step

Create decide.md with implementation plan.
