# Iteration 002 - ORIENT Phase

## Root Cause Analysis

The font decoding pipeline has two paths that can fail to handle ligatures:

### Path 1: OneByteEncoding (WIN_ANSI, MAC_ROMAN, STANDARD)

```
Bytes → lookup in encoding table → if None, bytes silently dropped
```

The `WIN_ANSI_ENCODING` table has `None` entries for bytes 0x1B-0x1F, which many PDF fonts use for ligature characters. When `filter_map` encounters `None`, the byte is silently dropped.

### Path 2: ToUnicode CMap

```
Bytes → lookup in CMap → if found, use mapping → if not found, use Latin-1 fallback
```

Some PDFs have malformed ToUnicode CMaps that map ligature bytes to just 'f' instead of the full ligature sequence. The Goyal PDF has:

- CMap entry: `<02>` → `<0066>` (maps to 'f')
- Differences array: `2/fi` (says position 2 is "fi" glyph)

The inconsistency means we get "f" instead of "fi".

## Hypothesis

Add a ligature expansion fallback that:

1. Maps common ligature byte positions to their expanded character sequences
2. Applies when the encoding table returns `None`
3. Applies when the ToUnicode CMap returns just 'f' for a known ligature position

## Decision

Implement `get_ligature_expansion()` function that handles both PostScript Type 1 positions (0x02-0x06) and Windows/Adobe positions (0x1B-0x1F).
