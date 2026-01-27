# OODA-28 Act: Additional Edge Case Tests

## Actions Completed

### Added 3 New Tests

| Test                                  | Description                             |
| ------------------------------------- | --------------------------------------- |
| `test_delete_document_unicode_name`   | Unicode characters in document title    |
| `test_delete_document_double_delete`  | Idempotency check (second delete → 404) |
| `test_delete_then_reupload_same_name` | Fresh state after delete + reupload     |

### Test Results

- **33/33 deletion tests pass** (30 existing + 3 new)
- No regressions introduced

### Tests Verify

1. **Unicode Support**: Japanese, emoji, accented chars in titles work
2. **Idempotency**: Deleting non-existent doc returns 404 (not error)
3. **Fresh State**: Reupload creates new document ID, old is gone

## Outcome

Edge case coverage expanded. Total deletion tests: 33

## Commit: test(deletion): add unicode, double-delete, and reupload edge cases
