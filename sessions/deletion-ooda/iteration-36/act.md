# OODA-36: Act

## Implementation Summary

Added 3 error boundary condition tests to `e2e_document_deletion.rs`:

### 1. `test_delete_empty_document_id`

- Sends DELETE to `/api/v1/documents/` (empty path segment)
- Verifies returns 404 or 405 (not 500)

### 2. `test_delete_extremely_long_id`

- Creates 10KB document ID ("x".repeat(10_000))
- Verifies no crash, returns 404 (not valid document)

### 3. `test_delete_sql_injection_pattern`

- Tests SQL injection-like patterns:
  - `'; DROP TABLE documents; --`
  - `1 OR 1=1`
  - `1; SELECT * FROM users`
  - `" OR ""="`
- URL-encodes patterns for safe transmission
- Verifies all return 404 (not 500)

## Dependencies Added

```toml
[dev-dependencies]
urlencoding = "2.1"
```

## Results

```
✅ OODA-36 TEST PASSED: Empty document ID handled correctly (404 Not Found)
✅ OODA-36 TEST PASSED: Long document ID handled safely (404 Not Found)
✅ OODA-36 TEST PASSED: SQL injection patterns are safe
```

## Test Count

- Before: 45 deletion tests
- After: 48 deletion tests (+3)

## Commit

```
test(deletion): add error boundary condition tests (OODA-36)

- test_delete_empty_document_id: empty path handling
- test_delete_extremely_long_id: DoS protection
- test_delete_sql_injection_pattern: injection safety
- 48/48 deletion tests pass
```
