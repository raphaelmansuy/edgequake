# IT29 — Orient: First Principles Analysis of Column Detection

## The fundamental question

**When should a PDF page be classified as multi-column?**

A multi-column layout has these properties (first principles):

1. Content is organized into 2+ **separate reading streams**
2. Each column occupies a **significant fraction** of page width (≥25%)
3. Columns are separated by a **visible gutter** (typically 20-80pt)
4. The x-coordinate clusters of column content are **well-separated**

## Why DBSCAN fails for indentation

DBSCAN clusters by density, not by semantic meaning. When a document has:

- Headings at x=72 (left margin)
- Bullet items at x=94 (indented 22pt)

DBSCAN sees two density peaks separated by 22pt. With adaptive epsilon
(10th percentile of pairwise distances = 10pt), these form separate clusters.

But this is NOT multi-column layout — it's **single-column with indentation**.

## Two independent guards (defense in depth)

### Guard 1: Inter-cluster gap validation

```
Real 2-column on 612pt page:         Single-column with indent:
┌───────────┬───────────┐            ┌──────────────────────┐
│ col1      │ col2      │            │████ heading (x=72)   │
│ x≈72      │ x≈340     │            │  ████ bullet (x=94)  │
│           │           │            │████ heading (x=72)   │
└───────────┴───────────┘            └──────────────────────┘
gap between centers: ~268pt          gap between centers: 22pt
threshold: 15% × 612 = 92pt         22 < 92 → SINGLE COLUMN
268 > 92 → MULTI COLUMN ✓           ✓ correct
```

15% of page width is ~92pt for US Letter, ~89pt for A4.
This threshold is well above any indentation gap (typically 10-30pt)
and well below any real column separation (150-300pt).

### Guard 2: Column balance validation

After building columns from clusters, check width ratio:

```
Real 2-column:                       False positive:
┌──────┬──────┐                     ┌──┬────────────────┐
│ 280pt│ 280pt│                     │94│     518pt      │
│      │      │                     │  │                │
└──────┴──────┘                     └──┴────────────────┘
ratio: 1.0 ✓                       ratio: 5.5 → SINGLE COLUMN
```

Threshold: 3.0x max/min ratio. Even asymmetric layouts (60/40 split)
give ratio ≈1.5, well under 3.0.

## Risk analysis

- Guard 1 might miss very narrow 2-column layouts → Guard 2 catches
- Guard 2 might miss asymmetric layouts → Guard 1 catches (clusters far apart)
- Together: double protection against false positives

## Downstream impact

When detect_columns() returns 1 column instead of 2:

- ReadingOrderDetector uses single_column_order() (sort by Y)
- Blocks maintain document flow order
- "Co‑creation outputs (examples)" stays BEFORE its bullet items ✓
