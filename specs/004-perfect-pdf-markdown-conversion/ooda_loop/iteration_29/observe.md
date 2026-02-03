# OODA-29: OBSERVE - Font Encoding Diagnosis

**Status:** IN PROGRESS  
**Date:** 2025-02-03  
**Focus:** Font encoding issue in Apple-Sandbox-Guide-v1.0.pdf

## Current Situation

### Symptom

The Apple-Sandbox-Guide-v1.0.pdf (48 pages, 354KB) produces garbled text:

```
EdgeQuake Output:
  !"#$% '( )'*+%*+,     → Should be: "Table of Contents"
  89"+ "0% :% +"$;4*<   → Should be: "What are we talking about?"
  /*+0'123+4'*          → Should be: "Introduction"
```

### Markitdown Output (Gold Standard)

```
Table	  of	  Contents
1	  –	  Introduction
2	  –	  What	  are	  we	  talking	  about?
```

Markitdown extracts this PDF **perfectly**. Every word is correct.

## Root Cause Analysis

### PDF Font Structure

The PDF uses fonts without a `/ToUnicode` CMap:

```
From diagnose_fonts binary (OODA-27):
  Page 1:
    F1.0: Type1, Encoding: None, ToUnicode: No, Mappings: 0
    F2.0: Type1, Encoding: None, ToUnicode: No, Mappings: 0
```

### What This Means

PDF fonts can map character codes to glyphs in several ways:

```
┌──────────────────────────────────────────────────────────────┐
│                    PDF Font Encoding Flow                     │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  Character Code (e.g., 0x21 = '!')                            │
│           │                                                   │
│           ▼                                                   │
│  ┌─────────────────────────────────┐                          │
│  │ ToUnicode CMap (if present)     │ ──→ Unicode codepoint    │
│  │   e.g., 0x21 → U+0054 ('T')     │                          │
│  └─────────────────────────────────┘                          │
│           │ (missing)                                         │
│           ▼                                                   │
│  ┌─────────────────────────────────┐                          │
│  │ /Encoding dict (BaseEncoding + │ ──→ Glyph name            │
│  │   /Differences array)           │                          │
│  └─────────────────────────────────┘                          │
│           │ (missing or incomplete)                           │
│           ▼                                                   │
│  ┌─────────────────────────────────┐                          │
│  │ WinAnsiEncoding fallback       │ ──→ Wrong character!     │
│  │   e.g., 0x21 → '!'              │                          │
│  └─────────────────────────────────┘                          │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### Why Markitdown Works

Markitdown likely uses one of these approaches:

1. **Adobe Glyph List (AGL)** - Maps glyph names like `/exclam` to Unicode
2. **/Differences array parsing** - Reads custom glyph mappings from font
3. **pdfminer/pymupdf** backend that handles these edge cases

## Current EdgeQuake Fallback Chain

```rust
// From font_handling.rs
pub fn get_encoding(&self) -> Option<&EncodingType> {
    // Check ToUnicode first
    if self.to_unicode.is_some() { return ToUnicode }

    // Check named encoding
    if let Some(encoding) = self.encoding_name.as_ref() {
        match encoding.as_str() {
            "WinAnsiEncoding" => return WinAnsi,
            "MacRomanEncoding" => return MacRoman,
            // ...
        }
    }

    // Fall back to WinAnsi ← THIS IS THE PROBLEM
    Some(&EncodingType::WinAnsi)
}
```

### The Core Issue

When `/Encoding` is `None` and `/ToUnicode` is missing:

- Current code falls back to WinAnsiEncoding
- But the font may use a **custom encoding** defined by `/Differences`
- Result: character code 0x21 ('!') should map to 'T' but maps to '!'

## Font-Level Investigation Needed

Need to examine the actual font dictionary to understand:

1. Does it have a `/Differences` array?
2. What are the glyph names?
3. Can we use Adobe Glyph List to map glyph names → Unicode?

## Test Data

| PDF                  | Pages | Size  | Has ToUnicode | Extraction   |
| -------------------- | ----- | ----- | ------------- | ------------ |
| Apple-Sandbox-Guide  | 48    | 354KB | No            | ❌ Garbled   |
| AI_Services_Elitizon | 1     | 110KB | Yes           | ✅ 98.9% TPS |
| Scottish SMEs        | 4     | 283KB | Yes           | ✅ 85.3% TPS |

## Files to Investigate

1. [font_handling.rs](../../edgequake/crates/edgequake-pdf/src/backend/font_handling.rs) - Font encoding resolution
2. [encodings.rs](../../edgequake/crates/edgequake-pdf/src/backend/encodings.rs) - Encoding tables
3. [text_extraction.rs](../../edgequake/crates/edgequake-pdf/src/backend/text_extraction.rs) - Character mapping

## References

- Adobe Glyph List: https://github.com/adobe-type-tools/agl-aglfn
- PDF Spec 1.7, Section 9.10.2: ToUnicode CMaps
- PDF Spec 1.7, Section 9.6: Character Encoding

## Next Step

Read the font dictionary from Apple-Sandbox-Guide to understand:

1. What type of font is used (Type1, TrueType, CID)
2. Whether `/Differences` array exists
3. What glyph names are used

Then implement a fallback using Adobe Glyph List.
