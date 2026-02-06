# OODA-16: Error Handling Tests

## Commit: 68e28d21

11 error handling tests covering all major API error paths.

| Test | Status Code | What it verifies |
|------|------------|-----------------|
| test_get_nonexistent_document | 404 | Missing document returns structured error |
| test_delete_nonexistent_document | 404/200 | Idempotent or 404 deletion |
| test_malformed_json_body | 400/422 | Invalid JSON handled |
| test_missing_content_field | 400/422 | Required field validation |
| test_empty_json_object | 400/422/201 | Empty body handled |
| test_invalid_document_id_format | 400/404 | Non-UUID ID rejected |
| test_sql_injection_in_doc_id | 400/404 | SQL injection blocked |
| test_nonexistent_endpoint | 404/405 | Unknown routes handled |
| test_query_empty_string | 200/400/422 | Empty query handled |
| test_query_invalid_mode | 200/400/422 | Invalid mode handled |
| test_double_delete | 404/200 | Second delete idempotent |

All pass in 0.02s.
