# OODA-43: Orient + Decide

## Analysis

Sequential stress tests:

1. Upload 20 documents sequentially
2. Delete all 20 in sequence
3. Verify timing is reasonable

## Action Plan

Add 2 tests:

1. `test_sequential_upload_delete_20_docs` - 20 docs stress test
2. `test_batch_cleanup_verification` - Verify clean state after batch

## Success Criteria

- Tests pass within reasonable time
- Total deletion tests: 60
