# OODA-24 Observe: Encodings Module Documentation Gap

## Current State

The `encodings.rs` module is 1317 lines but has only 2 WHY comments. This is one of the largest modules in the crate and handles critical PDF font encoding logic.

## Observations

1. **Module Purpose**: Converts PDF font byte codes to Unicode - essential for text extraction
2. **Complexity**: Multiple encoding schemes (WinAnsi, Standard, MacRoman, ToUnicode CMaps, Identity)
3. **Current WHY comments**: Only 2 in 1317 lines
4. **Test coverage**: Good - 14 tests exist

## Areas Lacking Documentation

1. `get_ligature_expansion()` - No WHY explaining the magic byte values
2. `decode()` match arms - Identity encoding lacks explanation
3. `ToUnicodeMap::parse()` - Complex CMap parsing needs algorithm overview
4. `parse_hex_code()` / `parse_hex_string()` - Implicit chunk size assumption
5. `extract_hex_codes()` - Has WHY but could have ASCII diagram

## Metrics

- Lines: 1317
- WHY comments: 2
- Test count: 469 (module contributes ~14)
- Clippy warnings: 0

## Recommended Actions

Add WHY comments to underdocumented functions to explain:
- Magic byte values for ligatures
- Identity encoding's UTF-16BE interpretation
- bfchar vs bfrange CMap formats
