# OODA-39: Orient + Decide

## Analysis

Document status lifecycle tests needed:
1. Verify document starts with correct status
2. Verify deletion response includes document status
3. Test that re-deleting returns meaningful info

## Action Plan

Add 2 lifecycle tests:
1. `test_document_status_on_creation` - Verify status after upload
2. `test_deletion_response_includes_status` - Verify status in delete response

## Success Criteria

- Tests pass
- Total deletion tests: 52
