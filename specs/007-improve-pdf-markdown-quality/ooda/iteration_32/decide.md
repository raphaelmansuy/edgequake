# IT32 — Decide

## Actions

1. Fix `extract_with_progress` to call `merge_same_line_blocks` (parity with `extract`)
2. Remove verbose debug logging from `extract_document` and `extract_document_with_progress`
3. Fix `test_extract_to_markdown_with_progress` to gracefully skip when pdfium unavailable
4. Verify all 440 tests pass

## Scope

Quick wins only. Save table detection for IT33.
