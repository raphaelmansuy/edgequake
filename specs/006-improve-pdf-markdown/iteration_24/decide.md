# OODA-24 Decide: Add WHY Documentation to Encodings Module

## Decision

Add targeted WHY comments to three underdocumented areas in `encodings.rs`:

## Implementation Plan

### 1. Ligature Expansion Function

Add WHY comment explaining the magic byte values:

- 0x02-0x06: PostScript Type 1 font convention
- 0x1B-0x1F: Windows/Adobe standard positions

### 2. Identity Encoding Decode

Add WHY comment explaining:

- Identity = direct UTF-16 Big Endian encoding
- Used by CID fonts (Chinese, Japanese, Korean)
- Why we process 2 bytes at a time

### 3. ToUnicode CMap Parser

Add ASCII diagram showing CMap format:

```
/CIDInit /ProcSet findresource begin
...
beginbfchar
<src_code> <unicode>
endbfchar
beginbfrange
<start> <end> <dst_start>
<start> <end> [<val1> <val2> ...]
endbfrange
```

## Expected Outcome

- WHY comments: 2 → 5+
- Maintainability improved for encoding logic
- Test count: Unchanged (469) - documentation only
