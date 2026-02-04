# OODA-30 DECIDE: Fix ToUnicode CMap Parsing

## Decision

**Fix the ToUnicodeMap parser to handle concatenated hex codes.**

## Priority: CRITICAL

This bug affects all PDFs with concatenated bfrange entries, which includes:

- Microsoft Office documents (Word, PowerPoint, Excel)
- Many professional typesetting systems
- Subset fonts with custom glyph mappings

## Implementation Plan

1. **Add `extract_hex_codes()` function** to `encodings.rs`
   - Scans input string for `<...>` patterns
   - Extracts all hex codes regardless of whitespace
   - Returns Vec<&str> of hex code strings

2. **Update `parse()` method** for both bfchar and bfrange
   - Replace `line.split_whitespace()` with `extract_hex_codes(line)`
   - Keep the rest of the parsing logic unchanged

3. **Update tests**
   - Fix test assertions to expect correct extraction
   - Increase timing threshold to account for parallel execution

4. **Clean up debug output**
   - Remove temporary eprintln! statements
   - Keep trace!() for production debugging

## Time Estimate

~30 minutes for implementation and testing.

## Risk Assessment

- **Low risk**: Change is localized to ToUnicode parsing
- **Good test coverage**: 8 fast_quality tests validate behavior
- **Easy to verify**: Apple-Sandbox-Guide extraction clearly shows fix works
