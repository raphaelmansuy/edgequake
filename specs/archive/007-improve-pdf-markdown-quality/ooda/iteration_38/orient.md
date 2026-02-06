# OODA-38 Orient

## Root Cause Analysis

### SectionNumberMerge — Single Mode Limitation

```
BEFORE (Mode A only):
  "3.2" ──x──> "DUAL-LEVEL..." (blocked: title not to RIGHT of number)

AFTER (Mode A + Mode B):
  "3.2" ──Mode B──> "DUAL-LEVEL..." (below, same X position) ✓
```

Root cause: Original code only checked `title_block.bbox.x1 > sec_x` (title to the right).
Academic PDFs often have section numbers on one line with titles on the next.

### Garbled Text — Threshold and Detection Gaps

```
IT37 checks:               IT38 fixes:
├── Outer guard > 50 chars  → Removed (check all text)
├── Per-word > 40 chars     → Lowered to > 35 chars
└── Space ratio < 5%        → Lowered from >80 to >60 chars
    (at > 80 chars)            + URL exception

NEW: CamelCase detection
├── Word > 25 chars
├── Internal uppercase ≥ 2
└── Catches "OriginalRelationsTextincludes" (30 chars, 2 internal upper)

NEW: Proportion guard
├── Long paragraph (>200 chars) with 1 garbled word → NOT filtered
└── Only filter if garbled words > 50% of text length
```

### Section Title — ALL CAPS Recognition

Root cause: `looks_like_section_title` classified ALL-CAPS text as "person name"
because the keyword list was incomplete. Solution: First-principle check for
ALL-CAPS → always a section title (person names use Title Case).

## Priority Assessment

All three fixes are high-impact, low-risk improvements to section structure and text cleanup.
