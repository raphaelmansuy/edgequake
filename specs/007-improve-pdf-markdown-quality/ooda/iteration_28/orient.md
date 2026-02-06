# OODA IT28 — Orient

## First Principles Analysis

### Why narrow-height glyphs fail line grouping

PDF fonts have widely varying glyph bounding boxes. An em dash (—) is a thin horizontal line with height ~3-4pt, while letters like "A" have height ~22pt at the same font size. Line grouping algorithms that rely on baseline or top alignment fail for these special glyphs because their y-coordinates don't match the surrounding text.

### Three-layer fix required

1. **Line grouping** (pymupdf_structs.rs): `can_add_span()` needs a vertical OVERLAP check as fallback when baseline/top tolerance fails
2. **Character normalization** (text_cleanup.rs): Em/en dashes should be preserved, not converted to hyphens
3. **Space insertion** (pymupdf_structs.rs + pdfium_backend.rs): Em dashes are NOT word-joining hyphens — they need spaces

### Risk Assessment

- Adding vertical overlap check could merge spans from different lines → mitigated by requiring >80% of span height to be within line range
- Preserving em dashes could affect garbled text filter → verified: filter already handles them correctly
- Removing em dash from hyphen-no-space could add unwanted spaces → em dashes already naturally have gaps from adjacent text in PDFs
