# OODA-29: DECIDE - Implement /Differences Array Parsing

## Decision

Implement `/Differences` array parsing with Adobe Glyph List fallback.

## Scope

**In Scope:**

1. Parse `/Differences` array from font encoding dictionary
2. Map glyph names to Unicode using a glyph name lookup table
3. Add new `DifferencesEncoding` variant to Encoding enum
4. Add fast quality test for Apple-Sandbox-Guide

**Out of Scope:**

1. Complex CID fonts (deferred)
2. TrueType cmap tables (deferred)
3. Embedded font programs (deferred)

## Implementation Steps

### Step 1: Add Glyph Name to Unicode Mapping

Create a HashMap-based lookup from the most common 500 glyph names:

```rust
// Common glyph names from Adobe Glyph List
pub static GLYPH_TO_UNICODE: phf::Map<&'static str, u16> = phf_map! {
    "A" => 0x0041,
    "B" => 0x0042,
    "T" => 0x0054,
    "a" => 0x0061,
    "space" => 0x0020,
    "exclam" => 0x0021,
    // ... 500 most common
};
```

### Step 2: Parse /Differences Array

Add function to font_handling.rs:

```rust
fn parse_differences(doc: &LopdfDocument, enc_dict: &Dictionary) -> Option<HashMap<u8, char>> {
    let diffs = enc_dict.get(b"Differences").ok()?.as_array().ok()?;
    let mut map = HashMap::new();
    let mut code = 0u8;

    for obj in diffs {
        match obj {
            Object::Integer(n) => code = *n as u8,
            Object::Name(name) => {
                let glyph = String::from_utf8_lossy(name);
                if let Some(&unicode) = GLYPH_TO_UNICODE.get(glyph.as_ref()) {
                    map.insert(code, char::from_u32(unicode as u32).unwrap_or('?'));
                }
                code = code.wrapping_add(1);
            }
            _ => {}
        }
    }

    Some(map)
}
```

### Step 3: Add DifferencesEncoding Variant

Update encodings.rs:

```rust
pub enum Encoding {
    OneByteEncoding(&'static CodedCharacterSet),
    ToUnicodeMap(ToUnicodeMap),
    Identity,
    DifferencesEncoding(HashMap<u8, char>),  // NEW
}
```

### Step 4: Update get_encoding()

In font_handling.rs, modify the encoding resolution:

```rust
Object::Reference(id) => {
    if let Ok(enc_dict) = doc.get_dictionary(*id) {
        // NEW: Try parsing /Differences first
        if let Some(diff_map) = parse_differences(doc, enc_dict) {
            return Encoding::DifferencesEncoding(diff_map);
        }
        // Fallback to BaseEncoding check
        // ...
    }
}
```

### Step 5: Add Fast Quality Test

Add test for Apple-Sandbox-Guide:

```rust
#[tokio::test]
async fn test_apple_sandbox_font_encoding() {
    let pdf_path = PathBuf::from("../../../zz_test_docs/Apple-Sandbox-Guide-v1.0.pdf");
    if !pdf_path.exists() { return; }

    let extractor = create_extractor();
    let pdf_bytes = fs::read(&pdf_path).unwrap();
    let result = extractor.extract_to_markdown(&pdf_bytes).await.unwrap();

    // Key test: "Table of Contents" should NOT be "!"#$%"
    assert!(result.contains("Table"), "Should extract 'Table' correctly");
    assert!(!result.contains("!"#$%"), "Should not have garbled encoding");
}
```

## Files to Create/Modify

| File                           | Action | Lines |
| ------------------------------ | ------ | ----- |
| `src/backend/glyph_list.rs`    | Create | ~150  |
| `src/backend/encodings.rs`     | Modify | ~20   |
| `src/backend/font_handling.rs` | Modify | ~50   |
| `tests/fast_quality.rs`        | Modify | ~30   |

## Time Budget

- Implementation: 30 minutes
- Testing: 15 minutes
- Documentation: 5 minutes

**Total: ~50 minutes**

## Success Metrics

1. Apple-Sandbox-Guide page 2 extracts "Table of Contents" instead of "!"#$%"
2. All existing fast quality tests pass
3. Total fast quality test time <10 seconds

## Fallback Plan

If /Differences parsing is more complex than expected:

1. Commit the observation/orient/decide docs
2. Create a simpler heuristic (detect garbled text, skip page)
3. Address in OODA-30 with more investigation
