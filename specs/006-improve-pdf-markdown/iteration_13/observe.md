# OODA-13: Observe - Undocumented Constants in pdfium.rs

## Current State

The `pdfium.rs` file has one undocumented magic number:

1. **line 266**: `fs * 0.25` - space character width
   - No explanation why 0.25 of font size

## Evidence

Most constants already have good WHY comments:

- ✅ line 6: Why PDFium explanation
- ✅ line 58: Bold detection >= 700 weight explained
- ✅ line 227: Font name matching unreliability
- ✅ line 240: Monospace font name pattern issues
- ✅ line 246: Spaces don't have tight bounds
- ✅ line 260: Spaces inherit style from previous char
- ❌ line 266: 0.25 space width ratio undocumented

## Analysis

The 0.25 factor means space width = 25% of font size:

- 12pt font → 3pt space width
- This is a typical space width in proportional fonts
- Monospace fonts have equal width (~0.6 of font size)
- 0.25 is a conservative estimate for word boundary detection
