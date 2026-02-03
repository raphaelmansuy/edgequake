# OODA-29: ORIENT - Font Encoding Gap Analysis

## Summary

The Apple-Sandbox-Guide produces garbled text because EdgeQuake cannot handle fonts with custom encodings defined via `/Differences` arrays.

## Architecture Gap

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Current Encoding Resolution                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Font Dictionary                                                     │
│  ┌───────────────────┐                                               │
│  │ /BaseFont: F1.0   │                                               │
│  │ /Encoding:        │──→ ❌ No named encoding (WinAnsi, etc.)      │
│  │   /Type: Encoding │                                               │
│  │   /Differences: [ │──→ ❌ NOT PARSED (lopdf doesn't implement)   │
│  │     33 /T /a /b..│                                               │
│  │   ]               │                                               │
│  │ /ToUnicode: None  │──→ ❌ No CMap available                       │
│  └───────────────────┘                                               │
│           │                                                          │
│           ▼                                                          │
│  ┌───────────────────┐                                               │
│  │ FALLBACK:         │                                               │
│  │ WinAnsiEncoding   │──→ ❌ Wrong! Char 33='!' but should be 'T'   │
│  └───────────────────┘                                               │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Root Cause Chain

1. **PDF uses Type1 fonts with custom encoding**
   - Common in older PDFs (2011 document)
   - Font defines character codes via `/Differences` array

2. **lopdf doesn't parse `/Differences` array**
   - Explicitly noted in code: "not implemented: dictionary differences encoding"
   - Falls back to StandardEncoding which is also wrong

3. **EdgeQuake falls back to WinAnsiEncoding**
   - Our fallback at line 128 of font_handling.rs
   - WinAnsi maps byte 33 (0x21) to '!' 
   - But the font maps byte 33 to 'T'

## Solution Options

### Option A: Implement /Differences Array Parsing (HIGH EFFORT)

Parse the font's `/Encoding` dictionary to extract:
```
/Differences [ 33 /T /a /b /l /e /space /o /f ... ]
```

This means:
- Byte 33 (0x21) → glyph name 'T' → Unicode U+0054
- Byte 34 (0x22) → glyph name 'a' → Unicode U+0061
- etc.

**Requires:**
1. Parse `/Differences` array from font encoding dict
2. Map glyph names to Unicode using Adobe Glyph List (lopdf has this!)
3. Build a custom encoding table per font

**Effort:** ~200 lines of Rust code  
**Risk:** Medium - glyph name edge cases

### Option B: Use lopdf's Glyph Name Table (MEDIUM EFFORT)

lopdf already has the Adobe Glyph List in `glyphnames.rs` (4502 lines!).

We could:
1. When encountering unknown encoding, try to extract /CharProcs or /Widths
2. Use heuristics to map character codes to glyph names
3. Convert glyph names to Unicode via lopdf's Glyph table

**Effort:** ~100 lines  
**Risk:** High - heuristics may fail

### Option C: PDF.js / Poppler Fallback (LOW EFFORT, HIGH DEPENDENCY)

For PDFs that fail extraction quality threshold:
1. Detect garbled output via heuristics (high non-ASCII ratio)
2. Fall back to external tool (pdftotext, pdf.js)

**Effort:** ~50 lines  
**Risk:** Adds external dependency

## Recommended Solution: Option A

Implement `/Differences` array parsing because:
1. It's the correct PDF spec behavior
2. lopdf already has the glyph name → Unicode mapping
3. Many legacy PDFs use this encoding method
4. It's a permanent fix, not a workaround

## Implementation Plan

### Phase 1: Parse /Differences Array

```rust
/// Parse /Differences array from encoding dictionary
/// Returns HashMap<u8, u16> mapping byte → Unicode
fn parse_differences(doc: &Document, enc_dict: &Dictionary) -> HashMap<u8, u16> {
    let mut map = HashMap::new();
    
    if let Ok(Object::Array(diffs)) = enc_dict.get(b"Differences") {
        let mut current_code: u8 = 0;
        for obj in diffs {
            match obj {
                Object::Integer(n) => current_code = *n as u8,
                Object::Name(glyph_name) => {
                    if let Some(unicode) = glyph_name_to_unicode(glyph_name) {
                        map.insert(current_code, unicode);
                    }
                    current_code += 1;
                }
                _ => {}
            }
        }
    }
    map
}
```

### Phase 2: Map Glyph Names to Unicode

```rust
/// Convert glyph name to Unicode codepoint using Adobe Glyph List
fn glyph_name_to_unicode(name: &[u8]) -> Option<u16> {
    let name_str = std::str::from_utf8(name).ok()?;
    
    // Use lopdf's Glyph constants (4500+ mappings)
    match name_str {
        "A" => Some(Glyph::A),
        "B" => Some(Glyph::B),
        // ... (generate from glyphnames.rs)
        _ => None
    }
}
```

### Phase 3: Integrate into Encoding Resolution

Modify `get_encoding()` in font_handling.rs:
```rust
Object::Reference(id) => {
    if let Ok(enc_dict) = doc.get_dictionary(*id) {
        // NEW: Check for /Differences array
        if enc_dict.has(b"Differences") {
            let diff_map = parse_differences(doc, enc_dict);
            return Encoding::DifferencesEncoding(diff_map);
        }
        // Existing: Check BaseEncoding
        if let Ok(base) = enc_dict.get(b"BaseEncoding") {
            // ...
        }
    }
}
```

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Glyph name not in AGL | Medium | Medium | Fall back to character code |
| Complex font hierarchies | Low | High | Log and skip unknown fonts |
| Performance overhead | Low | Low | Cache encoding per font |

## Success Criteria

1. Apple-Sandbox-Guide extracts "Table of Contents" correctly
2. Fast quality tests still pass (<5 seconds)
3. No regression on existing test PDFs
4. TPS for Apple-Sandbox-Guide ≥ 80%

## Files to Modify

1. `edgequake-pdf/src/backend/font_handling.rs` - Add /Differences parsing
2. `edgequake-pdf/src/backend/encodings.rs` - Add DifferencesEncoding type
3. Add new file: `edgequake-pdf/src/backend/glyph_list.rs` - AGL mapping

## Estimated Effort

- Parse /Differences: 50 lines
- Glyph name mapping: 100 lines (mostly generated)
- Integration: 30 lines
- Tests: 50 lines

**Total: ~230 lines of new code**
