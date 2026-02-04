# OODA-26 Observe: Two-Column Reading Order Issue

## Mission Refresh

Re-read specs/004-perfect-pdf-markdown-conversion.md at start of iteration.

## Current State

### OODA-25 Results

- Caption continuation detection implemented
- Blockquote format for captions
- 415 lib tests pass
- Commit: f09eddc7

### Critical Issue: Column Interleaving

Looking at the one_tool PDF output, text from left and right columns is interleaved:

**Current Output (interleaved):**

```
...trained end-to-end via Reinforcement                      ← LEFT column (ends abruptly)
> Figure 1.Illustration...tool:jump,                         ← FIGURE CAPTION (from right column area)
In the domain of software engineering (SWE)...repositowhich  ← MIXED: left "reposi-" + right "which"
...limited. SWE-BENCH (Jimenez et al., 2023)...              ← LEFT column continues
Learning (RL) directly from a pretrained model,              ← RIGHT column fragment
```

**Gold Standard (column-first reading):**

```
...trained end-to-end via Reinforcement Learning (RL) directly from a pretrained model,
without any closed-source distillation. Experiments demonstrate that RL-trained
RepoNavigator achieves state-of-the-art performance...

> Figure 1 Description: An illustration of an LLM navigating...
```

## Root Cause Analysis

### Problem: Y-Order vs Column-Order Reading

The current reading order algorithm sorts blocks by Y position (top-to-bottom),
which works for single-column documents but fails for multi-column layouts.

```
┌────────────────────────────────────────────────┐
│ TITLE (Y=50)                                   │
├─────────────────────┬──────────────────────────┤
│ LEFT COLUMN         │ RIGHT COLUMN             │
│ Para 1 (Y=100)      │ Para A (Y=100)           │
│ Para 2 (Y=150)      │ Para B (Y=150)           │
│ Para 3 (Y=200)      │ [FIGURE 1] (Y=180-220)   │
│ Para 4 (Y=250)      │ Para C (Y=250)           │
│ Para 5 (Y=300)      │ Para D (Y=300)           │
└─────────────────────┴──────────────────────────┘

Current Y-order: TITLE → Para1 → ParaA → Para2 → ParaB → Figure1 → Para3 → ParaC...
Correct column-order: TITLE → Para1 → Para2 → Para3 → Para4 → Para5 → ParaA → ParaB → Figure1 → ParaC → ParaD
```

### Code Investigation Needed

1. Where is reading order computed?
   - `src/layout/reading_order.rs`
   - `src/processors/layout_processing.rs` - LayoutProcessor

2. How are columns detected?
   - `detect_columns()` function
   - Column boundaries stored in `LayoutInfo`

3. Is column-aware reading order implemented?
   - Need to verify if blocks are sorted within columns

## Key Files to Examine

- `src/layout/reading_order.rs` - Reading order algorithm
- `src/processors/layout_processing.rs` - LayoutProcessor and BlockMergeProcessor
- `src/schema/document.rs` - Document structure with LayoutInfo

## Next Steps

1. Check current reading order implementation
2. Verify column detection is working
3. Implement column-first reading order if missing
