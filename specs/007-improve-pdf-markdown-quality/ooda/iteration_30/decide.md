# IT30 — Decide: Fix Header Classification

## Changes

### 1. Fix digit exclusion in classify_blocks() (pdfium_backend.rs)

Remove the digit-starting exclusion from `not_list` check. The purpose
of the guard is to exclude bullet items (•, -, *), not numbered sections.
Font size is the authority for header classification.

### 2. Make convert_standalone_bold_to_headers() conservative (markdown.rs)

Replace the broad criteria (short + uppercase + no punctuation) with a
narrow section-number pattern matcher:
- `**0) AI Strategy**` → `## 0) AI Strategy` ✓ (has section number)
- `**What we deliver**` → stays as `**What we deliver**` ✓ (no section number)
- `**1. Introduction**` → `## 1. Introduction` ✓ (has section number)

Regex pattern: `^(\d+[\).\-:\s]|section\s+\d|chapter\s+\d|part\s+[IVX\d])`

### 3. Update tests (markdown.rs)

Update 3 tests to reflect new conservative behavior:
- `test_convert_standalone_bold_basic` → split into `with_section_number` and `without_section_number`
- `test_convert_standalone_bold_preserves_caption` → "Table of Contents" no longer promoted
- `test_convert_standalone_bold_multiple_lines` → use numbered sections in test input

## Expected outcome

- No more false `## What we deliver` headers (×3 removed)
- No more false `## Capabilities`, `## Outcomes`, etc.
- Numbered sections correctly classified as h4 by font size
- 571 tests pass
