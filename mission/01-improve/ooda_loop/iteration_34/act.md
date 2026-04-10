# OODA-34 — Act

## Changes Made

### `crates/edgequake-pipeline/src/prompts/parser/tuple_parser.rs`
- Added 14 tests covering:
  - Single entity parse, single relationship parse, mixed entities + relationships
  - Empty input → 0 results
  - BR0006 self-referencing relationship skipped
  - Empty entity name → parse_errors incremented
  - Unknown line type → parse_errors incremented
  - Keyword limit 5
  - is_complete true/false + metadata
  - with_delimiters custom delimiters
  - "relationship" keyword (not just "relation")
  - Parser metadata = "tuple"

### `crates/edgequake-pipeline/src/prompts/parser/json_parser.rs`
- Added 15 tests covering:
  - extract_json_from_response: code block, raw braces, no JSON, generic block
  - sanitize_json: trailing comma, single quotes, control chars, comment at end, unquoted keys
  - Full parse: valid JSON, BR0006 self-ref skipped, empty name skipped, metadata, invalid JSON error, keyword limit 5

## Test Count
- Before: 1,383
- After: 1,412 (+29)

## Commit
- SHA: (pending)
