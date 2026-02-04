# OODA-30 ORIENT: ToUnicode CMap Format Analysis

## PDF ToUnicode CMap Specification

From the PDF Reference (Section 9.10.3), the bfrange format is:

```
n beginbfrange
<srcCode1_start> <srcCode1_end> <dstString1>
<srcCode2_start> <srcCode2_end> <dstString2>
...
endbfrange
```

The specification allows BOTH formats:

1. **Space-separated:** `<21> <21> <0054>`
2. **Concatenated:** `<21><21><0054>`

Many PDF generators (especially Microsoft Office) use the concatenated format.

## Character Mapping Analysis

For Calibri-Bold in Apple-Sandbox-Guide:

| Byte | ASCII | Expected Unicode | Character |
| ---- | ----- | ---------------- | --------- |
| 0x21 | '!'   | U+0054           | 'T'       |
| 0x22 | '"'   | U+0061           | 'a'       |
| 0x23 | '#'   | U+0062           | 'b'       |
| 0x24 | '$'   | U+006C           | 'l'       |
| 0x25 | '%'   | U+0065           | 'e'       |

So `!"#$%` → `Table` after correct decoding!

## ASCII Diagram: Before/After Fix

```
BEFORE FIX:
┌─────────────────────────────────────────────────────────────┐
│ CMap Line: "<21><21><0054>"                                 │
│                                                             │
│ split_whitespace() → ["<21><21><0054>"]  (1 part)          │
│                                                             │
│ if parts.len() >= 3 { ... } → FALSE, entry skipped!        │
│                                                             │
│ Result: Fall back to WinAnsi → garbled text                │
└─────────────────────────────────────────────────────────────┘

AFTER FIX:
┌─────────────────────────────────────────────────────────────┐
│ CMap Line: "<21><21><0054>"                                 │
│                                                             │
│ extract_hex_codes() → ["<21>", "<21>", "<0054>"] (3 parts) │
│                                                             │
│ if hex_codes.len() >= 3 { ... } → TRUE, entry parsed!      │
│                                                             │
│ Mapping: 0x21 → U+0054 ('T')                               │
│                                                             │
│ Result: Correct text extraction!                           │
└─────────────────────────────────────────────────────────────┘
```

## Solution Strategy

Replace `split_whitespace()` with a custom `extract_hex_codes()` function that:

1. Scans for `<` characters
2. Extracts content until matching `>`
3. Returns list of all hex codes regardless of whitespace

This handles BOTH space-separated AND concatenated formats.
