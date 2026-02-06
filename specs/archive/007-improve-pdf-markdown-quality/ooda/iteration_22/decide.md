# OODA Iteration 22 – Decide

## Decision

Implement two fixes in `pdfium_backend.rs::convert_text_block_to_schema_block()`:

### Fix 1: Inter-span word gap detection (same-line)

Between consecutive spans within the same line, check horizontal gap using the same threshold as `Line::text()` in `pymupdf_structs.rs`: `avg_size * 0.15`. If gap exceeds threshold AND neither span starts/ends with a hyphen, insert `TextSpan::plain(" ")`.

### Fix 2: Inter-line separator change ("\n" → " ")

Change the `TextSpan::plain("\n")` inserted between lines to `TextSpan::plain(" ")`. This prevents the newline from being absorbed and trimmed in the rendering pipeline.

## Rationale

- Both fixes are localized to pdfium_backend.rs — the span generation point
- They align span representation with how `Line::text()` and `Block::text()` produce text
- The `render_text()` span validity check uses `split_whitespace()` normalization, so `" "` and `"\n"` are equivalent for validation
- No renderer changes needed — the existing `trailing_space` logic handles `" "` correctly
- Same threshold (0.15) and hyphen-check logic as the battle-tested `Line::text()` method

## Files to modify

- `src/backend/pdfium_backend.rs`: Both fixes
- Tests: Update span count expectations

## Risk assessment

- Low risk: changes are additive (inserting TextSpans) and localized
- Test coverage: 569 tests provide safety net
- Visual verification: Convert AI_Services_Elitizon.pdf and check output
