# OODA-36: Decide

## Action Plan

1. Add `test_delete_empty_document_id` - Empty string ID
2. Add `test_delete_extremely_long_id` - 10KB ID
3. Add `test_delete_sql_injection_pattern` - SQL-like patterns

## Dependencies

- Add `urlencoding = "2.1"` to dev-dependencies for safe encoding

## Success Criteria

- All 3 tests pass
- No 500 errors for malformed input
- Total deletion tests: 48
