# OODA-30: Observe Phase — TrueType CMap Table Parsing

## Context

OODA-29 added /Differences array parsing and the glyph_list module. However, investigation revealed that **Apple-Sandbox-Guide.pdf still produces garbled text** on Page 2:

**Expected:** "Table of Contents", "Introduction", "Setting Up Sandboxing"  
**Actual:** `!"#$% '( )'*+%*+,`, `!"#$%&'(#!%"`, etc.

## Investigation: check_fonts.py Analysis

Using `scripts/check_fonts.py`, we analyzed the fonts in Apple-Sandbox-Guide:

| Font ID  | Base Font               | Encoding               | /FontFile2 | Status     |
| -------- | ----------------------- | ---------------------- | ---------- | ---------- |
| F1.0     | WVZFEA+CenturyGothic    | MacRomanEncoding       | No         | ✅ Works   |
| F2.0     | BGDIHV+FranklinGothic   | MacRomanEncoding       | No         | ✅ Works   |
| **F3.1** | **LHKJDD+Calibri-Bold** | **None (no encoding)** | **Yes**    | ❌ Garbled |
| **F4.1** | **YLMVTF+Cambria**      | **None (no encoding)** | **Yes**    | ❌ Garbled |
| **F5.1** | **LQKBQQ+Calibri**      | **None (no encoding)** | **Yes**    | ❌ Garbled |

### Key Finding

Fonts F3.1, F4.1, F5.1 are **subset TrueType fonts** (prefix like LHKJDD+ indicates subsetting):

- They have NO explicit encoding in the PDF font dictionary
- They embed a `/FontFile2` stream containing the TrueType font program
- The raw bytes (33, 34, 35...) map to custom glyph indices specific to this subset

### Why markitdown Works

markitdown (and other tools like PyMuPDF, pdftotext) parse the **embedded TrueType font's cmap table** to build the byte→Unicode mapping. The cmap table is inside the /FontFile2 stream.

## TrueType cmap Table Format

From the TrueType spec (Apple/Microsoft):

1. **Location:** Inside the TrueType font binary (found in /FontFile2 stream)
2. **Structure:** Header → Encoding Records → Subtables
3. **Relevant Format:** Format 4 (segment mapping) is most common for BMP Unicode

### Platform/Encoding to Use

For subset fonts:

- Platform 3 (Windows), Encoding 1 (Unicode BMP) is most reliable
- Platform 0 (Unicode), Encoding 3 (Unicode 2.0 BMP only) is alternative

### Glyph Index Resolution

For subset fonts, the byte in the PDF stream IS the glyph index. The cmap table provides Unicode→glyphID mapping. We need the **inverse**: glyphID→Unicode.

## Solution Approach

Use `ttf-parser` crate (v0.25.1) which provides:

- Zero-allocation, safe TrueType parsing
- `Face::from_slice()` to parse embedded font
- `face.glyph_index(char)` for char→glyph lookup
- We need to **iterate all cmap entries** to build glyphID→char reverse map

### Implementation Plan

1. Add `ttf-parser = "0.25"` to Cargo.toml
2. Create `truetype_cmap.rs` module with:
   - `parse_embedded_truetype(font_data: &[u8]) -> Option<HashMap<u16, char>>`
   - Iterate Platform 3/Encoding 1 cmap subtable
   - Build glyphID→Unicode mapping
3. Add `Encoding::EmbeddedTrueType(HashMap<u16, char>)` variant
4. Update `get_encoding()` to try /FontFile2 parsing when no encoding

## Expected Outcome

After this fix:

- Apple-Sandbox-Guide Page 2 should extract correctly
- Other subset TrueType PDFs should also work
- No regression on existing tests (fonts with ToUnicode/named encoding)

## Files to Modify

1. `Cargo.toml` — add ttf-parser dependency
2. `src/backend/mod.rs` — export new module
3. `src/backend/truetype_cmap.rs` — NEW: TrueType cmap parser
4. `src/backend/encodings.rs` — add EmbeddedTrueType variant
5. `src/backend/font_handling.rs` — use /FontFile2 parsing
6. `src/tests/fast_quality/` — add Apple-Sandbox-Guide test

## References

- Apple TrueType Reference: https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6cmap.html
- Microsoft OpenType cmap: https://learn.microsoft.com/en-us/typography/opentype/spec/cmap
- ttf-parser crate: https://docs.rs/ttf-parser/latest/ttf_parser/
