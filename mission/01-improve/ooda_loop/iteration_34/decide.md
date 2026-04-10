# OODA-34 — Decide

## Plan

### TupleParser Tests (~10 tests)
1. Parse single entity line → 1 entity, correct fields
2. Parse single relationship line → 1 relationship, correct fields
3. Parse mixed entities + relationships → correct counts
4. Empty input → 0 entities, 0 relationships
5. BR0006: self-referencing relationship skipped
6. Empty entity name → skipped, parse_errors incremented
7. Unknown line type with delimiter → parse_errors incremented
8. Keyword limit: >5 keywords → only 5 kept
9. is_complete with/without completion delimiter
10. with_delimiters custom delimiters work

### JsonExtractionParser Tests (~8 tests)
1. extract_json_from_response: ```json block
2. extract_json_from_response: raw JSON
3. extract_json_from_response: no JSON → returns input
4. sanitize_json: trailing comma removed
5. sanitize_json: single quotes to double quotes
6. sanitize_json: control chars stripped
7. Full parse: valid JSON with entities + relationships
8. Full parse: BR0006 self-ref skipped

## Files Modified
- `crates/edgequake-pipeline/src/prompts/parser/tuple_parser.rs` (add tests module)
- `crates/edgequake-pipeline/src/prompts/parser/json_parser.rs` (add tests module)
