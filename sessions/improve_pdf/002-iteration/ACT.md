# Iteration 002 - ACT Phase

## Implementation Summary

### Changes Made

**File: `edgequake/crates/edgequake-pdf/src/backend/sota_backend.rs`**

1. **Added `get_ligature_expansion()` function** (lines 843-865):

   - Maps PostScript Type 1 positions (0x02-0x06) to ligatures
   - Maps Windows/Adobe positions (0x1B-0x1F) to ligatures
   - Returns the expanded character sequence (e.g., "fi" instead of single byte)

2. **Modified `Encoding::decode()` for OneByteEncoding** (line ~880):

   - Added fallback: when encoding table returns `None`, call `get_ligature_expansion()`
   - Before: bytes with `None` mapping were silently dropped
   - After: ligature bytes are expanded to proper character sequences

3. **Modified `ToUnicodeMap::decode()`** (lines ~1068-1085):
   - Added detection for corrupted ligature mappings (CMap maps to just 'f')
   - When byte is a ligature position AND CMap returns just 'f', override with expansion
   - Added fallback for unmapped bytes to use ligature expansion before Latin-1

## Verification

### Test Results

- All 102 tests pass
- All 5 real_dataset PDFs processed without errors

### Metrics After Fix

| Metric                        | Before | After | Change |
| ----------------------------- | ------ | ----- | ------ |
| Broken ligature words         | 12     | 0     | -100%  |
| "first" in Goyal PDF          | 0      | 4     | +∞     |
| "classification" in Goyal PDF | 0      | 8     | +∞     |
| Goyal PDF char count          | 32139  | 32220 | +81    |

The +81 character increase is from expanding ligature bytes (each "fi" → 2 chars).

## Status

✅ **COMPLETE** - Ligature handling fix successfully deployed
