# OODA-29 ACT: Implement /Differences Array Parsing

## Actions Taken

### 1. Created `glyph_list.rs` Module (270 lines)
- **File:** `src/backend/glyph_list.rs`
- **Purpose:** Adobe Glyph List (AGL) subset for font encoding
- **Contents:**
  - ~300 glyph name → Unicode mappings
  - ASCII letters (A-Z, a-z)
  - Digits (zero-nine)
  - Punctuation and symbols
  - Typographic characters (bullet, emdash, ellipsis)
  - Ligatures (fi, fl, ff, ffi, ffl)
  - Accented Latin characters
  - Math symbols
  - 7 unit tests

### 2. Added `DifferencesEncoding` to `encodings.rs`
- Added new variant to `Encoding` enum:
  ```rust
  DifferencesEncoding(HashMap<u8, char>)
  ```
- Implemented decode logic with WinAnsi fallback for unmapped bytes

### 3. Updated `font_handling.rs` for /Differences Parsing
- Added `parse_differences()` function (50 lines)
- Updated `get_encoding()` priority order:
  1. ToUnicode CMap (most reliable)
  2. Encoding dictionary with /Differences array
  3. Named encoding (WinAnsi, MacRoman, Standard)
  4. WinAnsi fallback
- Added 2 new unit tests for DifferencesEncoding

### 4. Updated `mod.rs`
- Added `glyph_list` module export

## Test Results

### Unit Tests
- glyph_list: 7/7 passed
- font_handling: 9/9 passed (2 new tests)
- fast_quality: 7/7 passed in 2.05s

### Validation
All existing tests continue to pass with no regressions.

## Investigation Findings

### Apple-Sandbox-Guide Analysis
The garbled text `!"#$%` instead of "Table of Contents" is NOT caused by missing /Differences parsing. Root cause is **subset TrueType fonts without explicit encoding**:

- F3.1 (`Calibri-Bold`) - No encoding, subset font
- F4.1 (`Cambria`) - No encoding, subset font  
- F5.1 (`Calibri`) - No encoding, subset font

These fonts use custom glyph indices (33→T, 34→a, etc.) that don't match standard encodings. Proper extraction requires parsing the embedded TrueType `cmap` table.

### Fonts Working Correctly
- F1.0 (`CenturyGothic`) - MacRomanEncoding ✅
- F2.0 (`FranklinGothic-Book`) - MacRomanEncoding ✅
- F7.0 (`ArialMT`) - MacRomanEncoding ✅

## Next Steps (OODA-30)

To fix Apple-Sandbox-Guide extraction, we need:

1. **TrueType cmap Table Parsing**
   - Read font program from `/FontFile2` stream
   - Parse TrueType `cmap` table (platform 3, encoding 1)
   - Build custom byte→Unicode mapping

2. **Alternative Approach**
   - Use PDF.js or poppler-rs for subset font handling
   - Consider pdftotext integration for comparison

## Files Changed

| File | Changes |
|------|---------|
| `src/backend/glyph_list.rs` | New file (270 lines) |
| `src/backend/mod.rs` | Added glyph_list module |
| `src/backend/encodings.rs` | Added DifferencesEncoding variant |
| `src/backend/font_handling.rs` | Added parse_differences(), 2 tests |

## Metrics

- **Lines Added:** ~350
- **Tests Added:** 9
- **Tests Passing:** All (7 fast_quality + 9 font_handling + 7 glyph_list)
- **Time:** ~45 minutes

## Conclusion

The /Differences array parsing infrastructure is now in place and working. However, the Apple-Sandbox-Guide issue requires TrueType cmap table parsing, which is a more complex feature for OODA-30.
