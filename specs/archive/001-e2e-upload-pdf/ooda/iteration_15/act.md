# OODA-15: Edge Case Tests

## Commit: 60b7cb26

10 edge case tests covering boundary conditions not handled by e2e_data_model.rs.

| Test | What it verifies |
|------|-----------------|
| test_large_document_upload | ~50KB text → chunking works |
| test_content_with_null_bytes | Null bytes don't crash pipeline |
| test_title_path_traversal | ../../etc/passwd in title → plain text |
| test_very_long_title | 2000-char title handled |
| test_deeply_nested_metadata | 20-level nested JSON metadata |
| test_extra_unknown_json_fields | Forward-compatible JSON (extra fields ignored) |
| test_mixed_line_endings | \r\n + \n + \r mixed |
| test_content_only_newlines_tabs | Whitespace-only content |
| test_rapid_sequential_uploads | 10 uploads in sequence + list verification |
| test_code_content_special_chars | Code with angle brackets, quotes, backslashes |

All pass in 0.03s.
