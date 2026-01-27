# OODA-41: Orient + Decide

## Analysis

Metadata handling tests:

1. Upload with custom metadata
2. Verify metadata doesn't break deletion
3. Test metadata with special characters

## Action Plan

Add 2 tests:

1. `test_upload_with_metadata` - Metadata accepted on upload
2. `test_delete_with_metadata_doc` - Metadata doc deletes normally

## Success Criteria

- Tests pass
- Total deletion tests: 56
