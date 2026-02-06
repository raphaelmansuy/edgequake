# IT29 — Observe: Column Detection Misclassification

## Problem

After IT28 fixed em dash handling and same-line block merging, the output
shows correct title (`# AI Services — Elitizon`) but **content reordering**:
"Co‑creation outputs (examples)" appears AFTER its bullet items instead of before.

## Evidence

### Debug output (AFTER-PAGE1 blocks)

```
block 9  (Paragraph): 'Co‑creation workshops: fast alignment...'
block 10 (Paragraph): 'Co‑creation outputs (examples)'        ← displaced
block 11 (Paragraph): 'Use-case portfolio with effort/impact...'
block 12 (Paragraph): '"Thin vertical slice" plan...'
block 13 (Paragraph): 'Blueprint pack...'
```

Before LayoutProcessor, block 10 ("Co‑creation outputs") was at y=573,
but after processing it appeared after blocks at y=598+.

### Root cause chain

```
ColumnDetector.detect() → 2 columns detected
  └── GeometricClusterer.detect_columns()
        └── DBSCAN on x-coordinates: [72, 72, ..., 94, 94, ...]
              └── eps = 10.0 (minimum clamp from 10th percentile)
              └── cluster 1: x=72 (7 headings)
              └── cluster 2: x=94 (7 indented bullets)
              └── gap = 22pt < eps → 2 clusters!
  └── clusters_to_columns() → columns: (0, 94) and (94, 612)

ReadingOrderDetector.determine_order() → multi_column_order
  └── assign_to_column()
        └── headings at x=72 → "spanning" (overlap both columns)
        └── bullets at x=94 → "col1" (center in col (94, 612))
  └── merge_column_orders_with_footer_smart()
        └── spanning blocks before body: blocks 1-8
        └── col1 blocks: blocks 11-13 (bullets)
        └── remaining spanning: block 10 ("Co‑creation outputs")
        └── WRONG ORDER: bullets before their heading!
```

### Key metric

- Page width: 612pt (US Letter)
- Heading x1: 72pt
- Bullet x1: 94pt
- Gap: 22pt — this is **indentation**, NOT a column boundary
- Real 2-column gap would be ~250pt

### Epsilon calculation trace

Pairwise distances of x-coords [72, 72, ..., 94, 94, ...]:

- Many 0.0 (same-x pairs)
- Many 22.0 (cross-group pairs)
- 10th percentile ≈ 0.0
- eps = max(0.0, 10.0) = 10.0 (minimum clamp)
- 22pt > 10pt → clusters don't merge → false 2-column detection
