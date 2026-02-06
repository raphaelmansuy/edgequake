# OODA Iteration 27 – Orient

## Analysis of garbled text filter

The filter uses a heuristic: if >35% of words are ≤2 chars and not in the valid list, the text is considered "garbled" (OCR artifacts, font rendering noise).

```
┌─────────────────────────────────────────────────────────────┐
│  "0) AI Strategy & Co‑Creation"                             │
│                                                             │
│  Words: ["0)", "AI", "Strategy", "&", "Co‑Creation"]        │
│  Short words not in valid list: "0)" and "&" = 2/5 = 40%   │
│  Threshold: 35%                                             │
│  Result: FILTERED (false positive)                          │
│                                                             │
│  Fix: Add "&" to valid list + skip digit+delimiter patterns │
│  After fix: 0/5 = 0% → KEPT (correct)                      │
└─────────────────────────────────────────────────────────────┘
```

## Fix approach

1. Add `"&"` to `valid_short_words` — ampersand is a standard conjunction
2. Add section number skip: `if digit + (')' | '.') → skip` — section numbering pattern
