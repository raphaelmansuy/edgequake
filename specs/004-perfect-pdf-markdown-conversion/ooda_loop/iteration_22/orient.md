# OODA-22 Orient: Root Cause Analysis

## Date: 2025-02-03

## First Principles: What Makes Good PDF-to-Markdown Conversion?

```
┌─────────────────────────────────────────────────────────────────────────┐
│           FIRST PRINCIPLES: PDF → MARKDOWN CONVERSION                   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  1. TEXT FIDELITY                                                        │
│     ├── Every word in the PDF appears in the output                     │
│     ├── Words are spelled correctly (no broken UTF-8)                   │
│     └── Spacing between words is correct (not merged, not split)        │
│                                                                          │
│  2. STRUCTURE PRESERVATION                                               │
│     ├── Headers maintain hierarchy (H1 > H2 > H3)                       │
│     ├── Sections appear in correct order                                 │
│     ├── Lists are formatted as lists (not paragraphs)                   │
│     └── Tables are formatted as tables                                  │
│                                                                          │
│  3. READING ORDER                                                        │
│     ├── Single column: top to bottom                                    │
│     ├── Multi-column: left column fully, then right column              │
│     └── Spanning content: before columns resume                         │
│                                                                          │
│  4. SEMANTIC COHERENCE                                                   │
│     ├── Sentences are complete (not broken mid-word)                    │
│     ├── Paragraphs are unified (not fragmented)                         │
│     └── Block boundaries respect content boundaries                     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

## Current Pipeline Analysis

```
┌────────────────────────────────────────────────────────────────────────┐
│                    CURRENT EXTRACTION PIPELINE                          │
├────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   lopdf PDF parser                                                      │
│        │                                                                │
│        ▼                                                                │
│   ┌─────────────────┐                                                  │
│   │ extraction      │ → TextElement[]                                  │
│   │ _engine.rs      │    (per-character/word with X,Y,font)           │
│   └────────┬────────┘                                                  │
│            │                                                            │
│            ▼                                                            │
│   ┌─────────────────┐                                                  │
│   │ column_         │ → column_boundary: Option<f32>                   │
│   │ detection.rs    │    (where to split left/right)                   │
│   └────────┬────────┘                                                  │
│            │                                                            │
│            ▼                                                            │
│   ┌─────────────────┐                                                  │
│   │ text_           │ → MergedLine[]                                   │
│   │ grouping.rs     │    (words grouped into lines)                    │
│   └────────┬────────┘                                                  │
│            │                                                            │
│            ▼ ⚠️ PROBLEM AREA                                           │
│   ┌─────────────────┐                                                  │
│   │ layout_         │ → Block[]                                        │
│   │ processing.rs   │    (lines grouped into blocks)                   │
│   └────────┬────────┘    ⚠️ Reading order corrupted here               │
│            │                                                            │
│            ▼                                                            │
│   ┌─────────────────┐                                                  │
│   │ table_          │ → Document                                       │
│   │ detection.rs    │    (tables identified)                           │
│   └────────┬────────┘                                                  │
│            │                                                            │
│            ▼                                                            │
│   ┌─────────────────┐                                                  │
│   │ renderers/      │ → Markdown string                                │
│   │ markdown.rs     │                                                  │
│   └─────────────────┘                                                  │
│                                                                         │
└────────────────────────────────────────────────────────────────────────┘
```

## Problem Localization

### Problem 1: Reading Order in layout_processing.rs

From `BMP-AFTER-MERGE` output:

```
[7] X=55 Y=430 '1. Introduction'
>>>  [10] X=307 Y=171 'Figure 1...'
```

Block 10 (right column, Y=171) appears AFTER block 7 (left column, Y=430).
This means right column content with LOWER Y (higher on page) appears later.

**Root Cause Hypothesis:**
The block merging in layout_processing.rs is sorting by Y after merging,
which interleaves left and right column blocks.

### Problem 2: Block Merging in text_grouping.rs

From logs:

```
BMP-BEFORE-MERGE PAGE1 (68 blocks, 2 columns)
BMP-AFTER-MERGE PAGE1 (26 blocks)
```

42 blocks merged into other blocks. If merge crosses sentence boundaries:

- "AlphaEvolve orchestrates" becomes "AlphaEvolve\*\*orchestrates"
- Bold formatting bleeds across

**Root Cause Hypothesis:**
Merge logic checks Y-proximity and X-proximity but not sentence completeness.

### Problem 3: Gold File Quality

01_2512.25075v1.gold.md starts with:

```
5
2
0
2

c
e
D
1
3
```

This is the arXiv identifier displayed vertically in the margin.
Including this in the gold artificially lowers our TPS score.

**Root Cause:**
Gold files were generated by a different tool that included margin artifacts.

## Prioritized Fix Order

| Priority | Fix                  | Impact     | Effort | Files                |
| -------- | -------------------- | ---------- | ------ | -------------------- |
| 1        | Clean gold files     | +5-10% TPS | Low    | test-data/\*.gold.md |
| 2        | Fix reading order    | +5-7% SFS  | Medium | layout_processing.rs |
| 3        | Sentence-aware merge | +3-4% TPS  | Medium | text_grouping.rs     |
| 4        | Citation spacing     | +1% TPS    | Low    | text_grouping.rs     |

## Recommended Approach

Start with **Priority 1: Gold File Cleanup** because:

1. Lowest effort, highest TPS impact
2. Doesn't require code changes
3. Makes subsequent measurements more accurate

Then **Priority 2: Reading Order** because:

1. Highest structural fidelity impact
2. Already have OODA-12/29 partial fixes to build on
3. Clear root cause identified

## Risk Assessment

| Fix            | Risk                     | Mitigation                |
| -------------- | ------------------------ | ------------------------- |
| Gold cleanup   | May remove valid content | Review each file manually |
| Reading order  | May break single-column  | Test both layouts         |
| Sentence merge | May under-merge          | Tune threshold carefully  |
