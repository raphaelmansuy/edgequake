# OODA-46: Orient + Decide

## Analysis

Track ID tests:
1. Upload with track_id
2. Multiple documents with same track_id
3. Delete doesn't affect other track_id docs

## Action Plan

Add 2 tests:
1. `test_document_with_track_id` - Upload with track_id
2. `test_same_track_id_deletion` - Delete one, others remain

## Success Criteria

- Tests pass
- Total deletion tests: 66
