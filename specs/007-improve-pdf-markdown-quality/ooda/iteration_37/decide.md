# OODA IT37 — Decide

## Priority Actions

### 1. Enhanced Garbled Text Detection (HIGH)
- Add long-word check: any word > 40 chars (not URL/path) → garbled
- Add low-space-ratio check: > 80 chars with < 5% spaces → garbled
- URL/path exception: tokens containing "://", "/", ".com", ".org" are excluded

### 2. Header Number-Title Spacing (HIGH)
- Add `normalize_section_number_spacing()` to HeaderDetectionProcessor
- Pattern: `^\d+[A-Z]` → insert space (e.g., "1INTRO" → "1 INTRO")
- Run in first pass BEFORE level check (some headers already have levels set)

### 3. Renderer Fallback (HIGH)
- Modify `render_header()` to compare span-derived text with block.text
- If they differ (normalization applied), use block.text instead of spans
- Safe: only affects headers where text was explicitly normalized

## Test Plan
- Add 8 unit tests: 4 for normalize_section_number_spacing, 4 for garbled detection
- Validate with LightRAG PDF: headers spaced, page 3 cleaned
- Validate with Elitizon PDF: no regression
- Run full test suite: all 449+ tests pass
- Run clippy: 0 warnings in edgequake-pdf
