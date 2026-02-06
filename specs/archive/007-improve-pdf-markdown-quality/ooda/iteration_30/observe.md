# IT30 — Observe: Header Over-Promotion in Markdown Output

## Problem

After IT29 fixed content ordering, the output shows excessive `##` headers:

- `## What we deliver` (3 times)
- `## Capabilities` (2 times)
- `## Typical use cases`, `## Key outputs`, `## Outcomes`
- `## Spec → Code workflows`, `## Context Graph`, etc.

These should be bold paragraphs, not headers.

## Evidence

### Font size analysis

| Text                         | Font Size | Ratio | Classification                     |
| ---------------------------- | --------- | ----- | ---------------------------------- |
| AI Services — Elitizon       | 30.0pt    | 2.50  | SectionHeader(1) ✓                 |
| Executive summary            | 15.0pt    | 1.25  | SectionHeader(4) ✓                 |
| 0) AI Strategy & Co‑Creation | 15.0pt    | 1.25  | Paragraph ❌ (digit exclusion bug) |
| What we deliver              | 12.0pt    | 1.00  | Paragraph ✓ (body size)            |
| Capabilities                 | 12.0pt    | 1.00  | Paragraph ✓ (body size)            |
| Key outputs                  | 12.0pt    | 1.00  | Paragraph ✓ (body size)            |

### Two root causes

1. **classify_blocks() digit exclusion bug**: The `not_list` check excluded ALL
   text starting with a digit, preventing "0) AI Strategy..." (font=15pt,
   ratio=1.25) from being classified as a header.

2. **convert_standalone_bold_to_headers() too aggressive**: Promoted ANY standalone
   bold line (short, uppercase, no punctuation) to `## header`. This caught
   body-size bold labels like "What we deliver" as false positives.

## Pipeline trace

```
PDF extraction → font_size=12.0 (body), font_size=15.0 (section headers)
  ↓
classify_blocks(body_size=12.0, threshold=14.4):
  - "What we deliver" (12pt < 14.4) → Paragraph ✓
  - "0) AI Strategy" (15pt ≥ 14.4) → excluded by digit check → Paragraph ❌
  ↓
MarkdownRenderer:
  - Paragraph + bold → "**What we deliver**"
  - Paragraph + bold → "**0) AI Strategy & Co‑Creation**"
  ↓
convert_standalone_bold_to_headers():
  - "**What we deliver**" → `## What we deliver` ← FALSE POSITIVE
  - "**0) AI Strategy**" → `## 0) AI Strategy` ← correct intent, wrong source
```
