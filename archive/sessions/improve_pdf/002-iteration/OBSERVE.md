# Iteration 002 - OBSERVE Phase

## Date: 2025-01-15

## Observation

After fixing the reading order bug in iteration 001, we observed that content extraction had issues with ligatures - characters like "fi", "fl", "ff", "ffi", and "ffl" were being lost or incorrectly decoded.

### Symptoms

- Words like "first" appeared as "frst"
- Words like "specific" appeared as "specic"
- Words like "classification" appeared as "classifcation"
- The `2900_Goyal_et_al.pdf` had 12 broken ligature words

### Investigation

1. **Font Encoding Analysis**: Traced through the font decoding pipeline:
   - `FontInfo` → `Encoding` enum → `ToUnicodeMap` or `OneByteEncoding`
2. **Discovered Two Issues**:

   **Issue A - Missing Fallback**: Fonts without `ToUnicode` CMaps fall back to `WIN_ANSI_ENCODING` which has `None` at bytes 0x1B-0x1F (commonly used for ligatures in PDF fonts).

   **Issue B - Corrupt CMap**: The `2900_Goyal_et_al.pdf` has fonts where:

   - The `/Differences` array says `[2/fi/fl ...]` (position 2 = "fi", position 3 = "fl")
   - But the `ToUnicode` CMap incorrectly maps `<02>` to `<0066>` (just 'f', not 'fi')
   - This is a malformed PDF, but common in real-world academic papers

3. **Ligature Byte Positions**:
   - PostScript Type 1 fonts: 0x02=fi, 0x03=fl, 0x04=ff, 0x05=ffi, 0x06=ffl
   - Windows/Adobe standard: 0x1B=ffl, 0x1C=ffi, 0x1D=ff, 0x1E=fl, 0x1F=fi

## Metrics Before Fix

- Broken ligature word count: 12
- "first" occurrences in 2900_Goyal_et_al: 0
- "classification" occurrences in 2900_Goyal_et_al: 0
