# IT30 — Orient: First Principles Header Classification

## The fundamental question

**How should headers be distinguished from bold emphasis?**

First principles:
1. Headers are visually LARGER than body text (larger font size)
2. Bold emphasis is at BODY font size but with heavier weight
3. Section numbers are structural markers, not content-based

## Why two fixes are needed

### Fix 1: Digit exclusion in classify_blocks()

The `not_list` guard was over-broad:
```
NOT_LIST = !starts_with(bullet) && !starts_with(digit)
```

This excluded "0) AI Strategy" (15pt, ratio 1.25) because it starts with `0`.
But the guard's purpose was to exclude list items, not section headers.

Key insight: **Font size is the authority for header classification.**
If text is larger than body (ratio ≥ 1.2), it's a header regardless of
starting character. List detection should happen in a separate step.

### Fix 2: Bold-to-header promotion

The `convert_standalone_bold_to_headers()` was too aggressive — it promoted
ANY standalone bold line that was short and started uppercase.

First principles approach: Since classify_blocks() already handles
font-size-based headers, the bold-to-header promotion should only catch
the edge case of section-numbered text that was at body font size.

New rule: **Only promote bold lines with a section number pattern.**
This is structural (number patterns), not content-based (keywords).

## Impact analysis

### Before (IT29 output):
```
## 0) AI Strategy & Co‑Creation     ← bold promotion (should be font-size h4)
## What we deliver                   ← FALSE POSITIVE
## Capabilities                      ← FALSE POSITIVE
## Typical use cases                 ← FALSE POSITIVE
```

### After (IT30 output):
```
#### 0) AI Strategy & Co‑Creation    ← font-size h4 (correct)
**What we deliver**                   ← bold paragraph (correct)
**Capabilities**                      ← bold paragraph (correct)
**Typical use cases**                 ← bold paragraph (correct)
```
