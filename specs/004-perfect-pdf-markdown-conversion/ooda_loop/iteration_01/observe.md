# Iteration 01: OBSERVE

**Date**: 2026-02-02  
**Mission**: Perfect PDF to Markdown Conversion  
**Focus**: Root cause analysis of extraction failures in `zz_test_docs/` PDFs

---

## 1. Test Document Analysis

### 1.1 Qwen.pdf - CRITICAL FAILURE (0 bytes output)

**PDF Characteristics:**

- Pages: 1
- Size: 852,645 bytes (832.66 KB)
- Font count: 25 fonts

**Font Breakdown:**
| Type | Count | Details |
|------|-------|---------|
| Type0 (CID) | 2 | `AAAAAA+iconfont`, `IAAAAA+Times-Roman` (Identity-H encoding) |
| Type3 | 23 | Custom glyph procedures with `/Differences` arrays |

**Root Cause Identified:**
Type3 fonts use **glyph procedures** (mini-PostScript programs) to define character shapes. Each glyph is referenced by a name like `/g32`, `/g105`, etc. in the `/Differences` array.

Our extraction engine fails because:

1. `FontInfo::from_dict()` in [font_handling.rs](../../../edgequake/crates/edgequake-pdf/src/backend/font_handling.rs) doesn't recognize Type3 font encoding
2. The `/Encoding` for Type3 is a dictionary with `/Differences`, not a named encoding like "WinAnsiEncoding"
3. Without proper encoding resolution, text shows as empty strings

**PDFMiner Success:**
PDFMiner extracts 689 characters successfully, proving the PDF is valid.

### 1.2 001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf - PARTIAL SUCCESS

**Issues Identified:**

1. Table cells contain duplicated/misaligned text
2. Multi-column detection misfires (reports 3 columns when content is 1 column with margins)
3. Bold/italic preservation inconsistent

### 1.3 AgenticPlatformReference Architecture.pdf - PARTIAL SUCCESS

**Issues Identified:**

1. ASCII art diagrams not preserved as code blocks
2. Some structural headers missing proper formatting

---

## 2. Code Architecture Analysis

### 2.1 Extraction Pipeline

```
┌──────────────┐    ┌─────────────────┐    ┌────────────────┐
│  PDF Bytes   │───>│ ExtractionEngine│───>│   Document IR  │
└──────────────┘    └─────────────────┘    └────────────────┘
                           │
                           ├── get_page_fonts()
                           ├── get_page_content()
                           ├── ContentParser::parse()
                           ├── TextGrouper::group_into_lines()
                           └── BlockBuilder::build()
```

### 2.2 Font Handling Flow

```
┌────────────────────┐
│  Font Dictionary   │
└─────────┬──────────┘
          │
          v
┌─────────────────────────────────┐
│   FontInfo::from_dict()         │
│   - Extract BaseFont name       │
│   - Detect bold/italic          │
│   - Get encoding                │
│     ├── ToUnicode CMap (best)   │
│     ├── Named encoding          │
│     └── Identity fallback       │
└─────────────────────────────────┘
          │
          v
┌─────────────────────────────────┐
│   FontInfo::get_encoding()      │
│   Line 76-117 in font_handling  │
│   - Checks ToUnicode first      │
│   - Checks Encoding entry       │
│   - MISSING: Type3 /Differences │
└─────────────────────────────────┘
```

### 2.3 Key Files for Font Encoding

| File                                                                                       | Lines  | Responsibility                                   |
| ------------------------------------------------------------------------------------------ | ------ | ------------------------------------------------ |
| [font_handling.rs](../../../edgequake/crates/edgequake-pdf/src/backend/font_handling.rs)   | 1-275  | Font metadata extraction, encoding resolution    |
| [encodings.rs](../../../edgequake/crates/edgequake-pdf/src/backend/encodings.rs)           | 1-1210 | Encoding tables, ToUnicode parsing, decode logic |
| [content_parser.rs](../../../edgequake/crates/edgequake-pdf/src/backend/content_parser.rs) | 1-607  | PDF content stream parsing, text extraction      |

---

## 3. Type3 Font Deep Dive

### 3.1 Type3 Font Structure (from Qwen.pdf)

```
/F5 {
  /Subtype: /Type3
  /Encoding: {
    /Type: /Encoding
    /Differences: [0, '/g0', '/g0', ..., '/g32', ..., '/gAB', ..., '/gEE']
  }
  /ToUnicode: (stream reference)
  /CharProcs: (dictionary of glyph procedures)
  /FontMatrix: [scale values]
}
```

### 3.2 What the `/Differences` Array Means

The array format is: `[startCode, name1, name2, ...]`

- Position 0-49: mapped to `/g0` (placeholder)
- Position 50: mapped to `/g32`
- Position 171: mapped to `/gAB`
- Position 182: mapped to `/gB6`
- etc.

The glyph names (`/g32`, `/gAB`) correspond to entries in `/CharProcs` and must be cross-referenced with `/ToUnicode` to get the actual character.

### 3.3 Correct Handling Strategy

1. If Type3 font has `/ToUnicode`: Use it (this should work but we need to verify)
2. If no ToUnicode: Parse `/Differences` to build glyph-to-index mapping
3. Cross-reference with any available glyph hints

---

## 4. Current Encoding Resolution Gap

In `font_handling.rs` line 76-117, `get_encoding()`:

```rust
fn get_encoding(doc: &LopdfDocument, font_dict: &Dictionary) -> Encoding {
    // Check for ToUnicode CMap first (most reliable)
    if let Ok(to_unicode) = font_dict.get(b"ToUnicode") {
        // ... parse ToUnicode stream
    }

    // Check Encoding entry
    if let Ok(enc) = font_dict.get(b"Encoding") {
        match enc {
            Object::Name(name) => {
                // Handle named encodings: WinAnsiEncoding, etc.
            }
            Object::Reference(id) => {
                // Handle encoding dictionary - but only checks BaseEncoding!
                // MISSING: Does NOT parse /Differences array
            }
            _ => {}
        }
    }

    // Default fallback
    Encoding::OneByteEncoding(&encodings::WIN_ANSI_ENCODING)
}
```

**Gap:** When `/Encoding` is a dictionary (Type3 style), we only check `/BaseEncoding` but ignore `/Differences`. For Type3 fonts, there's usually NO BaseEncoding - only Differences!

---

## 5. Observations Summary

### Critical Issues

1. **Type3 font encoding not handled** → Qwen.pdf produces 0 bytes
2. **ToUnicode may exist but parsing fails** → Need to verify ToUnicode parsing for Type3

### Medium Issues

3. **Table detection produces malformed markdown** → Cell boundaries misaligned
4. **Multi-column detection false positives** → Margin areas detected as columns
5. **ASCII art not preserved** → Needs monospace/code block detection

### Test Infrastructure Gaps

6. **No gold standard for zz_test_docs/** → No reference output to compare
7. **Quality metrics not automated** → Manual inspection required
8. **Edge case coverage limited** → Type3, CID, encrypted PDFs not tested

---

## 6. Next Steps (→ Orient)

1. Analyze ToUnicode stream handling for Type3 fonts
2. Determine if ToUnicode is being parsed correctly
3. Design Type3 /Differences parsing strategy
4. Create minimal reproduction test case
