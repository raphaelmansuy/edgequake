# OODA-20 Observe: Line Wrapping / Block Merge Issues

## Date

2026-02-03

## Problem Statement

After fixing the text truncation bug (OODA-19), text was correctly extracted but appearing as separate lines in markdown:

```markdown
Elitizon designs and delivers production-grade AI systems with a focus on agentic

**workflows, software delivery automation, and context intelligence. We help**

teams move from prototypes to reliable, governed deployments with measurable ROI.
```

Expected: Single paragraph without line breaks.

## Root Cause Analysis

### Issue 1: Zero-Width Bounding Boxes

In `block_builder.rs`, `calculate_line_bbox()` computed `max_x` as `max(e.x)` instead of `max(e.x + estimated_width)`.

- Block 2: bbox=[300, 69, **300**, 85] → zero width
- Block 3: bbox=[300, 87, **300**, 103] → zero width

This caused blocks to be assigned to different columns inconsistently.

### Issue 2: Spurious Column Detection

The geometric column detector was finding 3 columns:

- Column 0: x=0 → x=300
- Column 1: x=300 → x=321.6 (only 21.6pt wide!)
- Column 2: x=321.6 → x=612

The narrow column 1 was caused by indented bullet points (x=322) vs paragraph text (x=300).

### Issue 3: Column Assignment by Center Point

`get_block_column()` used block center point for column assignment:

- Block "Elitizon designs..." (width 722pt): center.x = 661 → Column 2
- Block "ROI." (width 35pt): center.x = 317.5 → Column 1

Adjacent blocks got different column assignments, preventing merge.

## Evidence

### Block Bounding Boxes (Before Fix)

```
block 2 bbox=[300,69,300,85]: 'Elitizon designs...'  # zero width!
block 3 bbox=[300,87,300,103]: 'workflows...'         # zero width!
```

### Column Detection Output

```
BlockMerge: Column 0 bbox: x1=0.0 y1=0.0 x2=300.0 y2=535.8
BlockMerge: Column 1 bbox: x1=300.0 y1=0.0 x2=321.6 y2=535.8   # 21.6pt wide!
BlockMerge: Column 2 bbox: x1=321.6 y1=0.0 x2=612.0 y2=535.8
```

### Merge Rejection Log

```
MERGE-REJECT: 'teams move from' vs 'ROI.' - different columns (col 1 vs col 0)
```
