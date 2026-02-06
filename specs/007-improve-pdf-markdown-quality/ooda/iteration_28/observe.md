# OODA IT28 — Observe

## Current Output Title

```
# AI Services Elitizon —
```

Expected (gold standard):
```
# AI Services — Elitizon
```

## Root Cause Analysis

Three interrelated issues causing incorrect title rendering:

### Issue 1: Em Dash Word Order
The em dash character (U+2014, —) has a very narrow glyph height (~3.7pt) compared to regular letters (~22pt at font_size=30). The `can_add_span()` in `pymupdf_structs.rs` checks baseline/top alignment with 3.0pt tolerance, which fails for the em dash (baseline diff = 7.5pt, top diff = 5.9pt). Result: em dash gets placed on a separate line, then merged into the block after "Elitizon" instead of between "AI Services" and "Elitizon".

Raw character positions confirm correct extraction order:
- "s" (Services): x0=219.9
- "—" (em dash): x0=242.1 ← correctly between
- "E" (Elitizon): x0=280.6

### Issue 2: Em Dash → Hyphen Conversion  
`fix_ocr_text()` in `text_cleanup.rs` converts `\u{2014}` to `-`. This loses the em dash character.

### Issue 3: Em Dash Space Suppression
Both `Line::text()` in `pymupdf_structs.rs` and `convert_text_block_to_schema_block()` in `pdfium_backend.rs` treat em dashes as hyphens, suppressing space insertion around them.

## Metrics
- 569 tests pass
- Block 0 text before fix: `'AI Services Elitizon —'` (wrong order, converted to -)
- Block 0 text after fix: `'AI Services — Elitizon'` (correct order, preserved —)
