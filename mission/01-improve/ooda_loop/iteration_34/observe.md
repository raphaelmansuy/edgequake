# OODA-34 — Observe

## Target Files

- `crates/edgequake-pipeline/src/prompts/parser/tuple_parser.rs` (183 lines, 0 tests)
- `crates/edgequake-pipeline/src/prompts/parser/json_parser.rs` (245 lines, 0 tests)

## Observations

### TupleParser (tuple_parser.rs)
- Pure function `parse(response, chunk_id)` → `Result<ExtractionResult>`
- Parses `entity<|#|>Name<|#|>TYPE<|#|>Description` lines
- Parses `relation<|#|>Source<|#|>Target<|#|>keywords<|#|>Description` lines
- Normalizes names, skips empties, BR0006 self-ref skip, keyword limit 5
- `is_complete()`: checks for completion delimiter in response
- `with_delimiters()`: custom delimiter constructor
- Metadata: "parser"="tuple", "parse_errors"=count, "is_complete"=bool

### JsonExtractionParser (json_parser.rs)
- Pure function `parse(response, chunk_id)` → `Result<ExtractionResult>`
- Calls `extract_json_from_response()` to unwrap markdown code blocks
- Calls `sanitize_json()` to fix LLM-malformed JSON
- Same BR0006, empty name, keyword limit 5 rules
- `sanitize_json()`: strips control chars, removes comments, trailing commas, single quotes→double, unquoted keys
- `extract_json_from_response()`: handles ```json blocks, raw JSON, nested braces

## Test Coverage Gap
Both files have 0 tests. All functions are pure (no I/O) — ideal for unit testing.
