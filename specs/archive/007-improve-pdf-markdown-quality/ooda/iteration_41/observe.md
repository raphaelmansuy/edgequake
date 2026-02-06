# IT41 — Observe: URL Spacing Regression from IT40

## Mission Re-Read

Re-read `specs/007-improve-pdf-markdown-quality.md` at session start.

## Current State

- IT40 committed `a6a2da49` — font-aware word boundary detection
- 462 lib tests passing, 0 clippy warnings

## Bug Found: URL Spacing Regression

After IT40's font-aware threshold change (22% for proportional fonts), URLs developed spurious spaces:

```markdown
IT39 (before): https://github.com/HKUDS/LightRAG
IT40 (after): https : //github . com/HKUDS/LightRAG
```

Also affected:

- Email addresses: `zrguo1 01 @hku . hk` → `zrguo101@hku.hk`
- arXiv URLs: `https : //arxiv . org/abs/2409 . 05591`

## Root Cause Analysis

The 22% threshold for proportional fonts is detecting gaps around URL/path punctuation (`:`, `/`, `.`, `@`) as word boundaries.

In typeset PDFs:

- Body text: kerning ~5-15%, spaces ~20-25%
- URL punctuation: kerning can be 20-30% due to fixed-width rendering

The IT40 threshold of 22% catches these URL punctuation gaps as word boundaries, creating spurious spaces.

## Elitizon Regression Risk

When first attempting to fix with a blanket "all punctuation uses 33%" rule:

- URLs: ✅ Fixed
- Elitizon "AI Agent Design & Building": ❌ Regressed to "Design &Building"

The & ampersand is general punctuation that SHOULD allow word boundary detection, unlike URL punctuation.

## Two Types of Punctuation

| Type     | Examples        | Word Boundary?       | Threshold Needed |
| -------- | --------------- | -------------------- | ---------------- |
| URL/path | `: / . @ - _`   | NO (part of token)   | 33%              |
| General  | `& , ; ! ? ( )` | YES (separate words) | 22%              |

## File Location

- `src/layout/pymupdf_structs.rs` — `Span::can_append()` function
