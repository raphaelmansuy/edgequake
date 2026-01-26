# OODA-31 Observe: Batch Deletion Test

## Gap Identified

No test for deleting multiple documents in a batch-like pattern.
Current tests focus on single or sequential deletes.

## Need

Test that simulates bulk cleanup scenarios:

1. Upload N documents
2. Delete all N in quick succession
3. Verify all cleaned up correctly

## Current Coverage

- Single deletion: ✅
- Sequential (5 docs): ✅
- Bulk (10+ docs): ❌ Not tested

## Action: Add bulk deletion test
