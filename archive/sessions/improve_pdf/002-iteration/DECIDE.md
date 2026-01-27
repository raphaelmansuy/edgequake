# Iteration 002 - DECIDE Phase

## Solution Design

### Component: `get_ligature_expansion(byte: u8) -> Option<&'static str>`

A lookup function that maps common ligature byte positions to their expanded character sequences:

```rust
fn get_ligature_expansion(byte: u8) -> Option<&'static str> {
    match byte {
        // PostScript Type 1 fonts typically use these positions
        0x02 => Some("fi"),
        0x03 => Some("fl"),
        0x04 => Some("ff"),
        0x05 => Some("ffi"),
        0x06 => Some("ffl"),
        // Windows/Adobe standard positions
        0x1B => Some("ffl"),
        0x1C => Some("ffi"),
        0x1D => Some("ff"),
        0x1E => Some("fl"),
        0x1F => Some("fi"),
        _ => None,
    }
}
```

### Integration Points

1. **OneByteEncoding::decode()**: When `map[b]` returns `None`, call `get_ligature_expansion(b)` before dropping the byte.

2. **ToUnicodeMap::decode()**:
   - When CMap returns just `'f'` (0x0066) for a byte, check if it's a ligature position
   - If yes, override with the proper ligature expansion
   - When CMap has no mapping, use ligature fallback before Latin-1 fallback

### Risk Assessment

- **Low risk**: Only affects bytes that would otherwise be dropped or corrupted
- **Reversible**: Changes are localized to the font decoding logic
- **Testable**: Can verify by checking word integrity in output

## Approval

✅ Proceed with implementation
