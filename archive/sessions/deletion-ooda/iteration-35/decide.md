# OODA-35: Decide

## Action Plan

1. Add `test_parallel_delete_same_document`
   - Upload one document
   - Spawn 5 concurrent delete tasks
   - Verify: 1 OK + 4 NOT_FOUND (or similar idempotent behavior)

2. Add `test_rapid_create_delete_cycles`
   - 10 create/delete cycles
   - Verify no orphan nodes/edges remain

## Success Criteria

- Both tests pass
- Total deletion tests: 45
- No orphan data after rapid operations
