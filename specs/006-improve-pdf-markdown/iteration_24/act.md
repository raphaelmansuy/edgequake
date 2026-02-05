# OODA-24 Act: Add WHY Documentation to Encodings Module

## Actions Taken

### 1. Ligature Expansion Documentation
Added comprehensive WHY comment with ASCII diagram showing PostScript vs Windows byte value conventions for ligatures.

### 2. Identity Encoding Documentation  
Added WHY comment explaining UTF-16 Big Endian encoding used by CID fonts (CJK text) with byte layout example.

### 3. ToUnicode CMap Parser Documentation
Added detailed WHY comment with ASCII diagram showing complete CMap format:
- codespacerange section
- bfchar (single mappings)
- bfrange (range mappings)

## Results

| Metric | Before | After |
|--------|--------|-------|
| WHY comments | 2 | 10 |
| Tests | 469 | 469 (unchanged) |
| Clippy warnings | 0 | 0 |

## Code Quality Improvements

1. **Ligature function**: Now explains the historical reason for two different byte ranges
2. **Identity decode**: Now explains the CJK use case and byte layout
3. **CMap parser**: Now has clear format specification for debugging

## Files Modified

- `src/backend/encodings.rs` - Added 3 WHY comment blocks with ASCII diagrams
