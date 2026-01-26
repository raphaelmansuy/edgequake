# OODA-46: Act

## Implementation Summary

Added 2 track_id tests to `e2e_document_deletion.rs`:

### Helper Function

```rust
async fn upload_document_with_track_id(
    app: &axum::Router,
    title: &str,
    content: &str,
    track_id: &str,
) -> (StatusCode, Value)
```

### Tests Added

1. `test_document_with_track_id`
   - Uploads document with track_id field
   - Verifies document created successfully

2. `test_same_track_id_deletion`
   - Creates two docs with same track_id
   - Deletes one, verifies other remains
   - Confirms documents are independent despite shared track

## Results

```
✅ OODA-46 TEST PASSED: Document with track_id
✅ OODA-46 TEST PASSED: Same track_id deletion
```

## Test Count

- Before: 64 deletion tests
- After: 66 deletion tests (+2)

## Commit

```
test(deletion): add track_id tests OODA-46

- test_document_with_track_id
- test_same_track_id_deletion
- Add upload_document_with_track_id helper
- 66 deletion tests pass
```
