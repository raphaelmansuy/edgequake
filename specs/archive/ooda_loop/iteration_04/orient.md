# OODA Iteration 04 - Orient

## Date: 2026-02-04

## Analysis

### Root Cause Hierarchy

```
QUALITY = 0.573 (gap: -0.377)
    │
    ├── ROUGE-L = 0.491 (gap: -0.409)  ◀◀◀ PRIMARY PROBLEM
    │       └── Cause: Blocks sorted by simple Y-then-X
    │       └── Fix: Implement smart sort key (P-overlap)
    │
    ├── Structure = 0.295 (gap: -0.505)
    │       └── Cause: Missing join phases → wrong block boundaries
    │       └── Fix: Implement join_rects_phase1/2/3
    │
    ├── Format = 0.312 (gap: -0.388)
    │       └── Cause: Markdown not generated (secondary)
    │       └── Fix: Later iteration
    │
    └── Word F1 = 0.914 (gap: -0.036)  ✅ ALREADY HIGH
            └── Content extraction is working
```

### Why Smart Sort Key Matters

**Current algorithm (broken)**:

```
Block A: y0=100, x0=50  → sort key = (100, 50)
Block B: y0=80, x0=300  → sort key = (80, 300)
Result: B comes before A (wrong! B is in column 2)
```

**Smart sort key (correct)**:

```
Block A: y0=100, x0=50  → no P overlap → key = (100, 50)
Block B: y0=80, x0=300  → P=A overlaps → key = (A.y0=100, B.x0=300)
Result: A comes before B (correct! same effective Y)
```

The smart sort key ensures blocks in different columns but same row get sorted left-to-right.

### Why Line Tolerance Matters

**5pt tolerance** (current):

```
Line 1: y0=100, "Hello"
Line 2: y0=104, "World"  ← Merged into Line 1 (4pt gap < 5pt)
Result: "HelloWorld"
```

**3pt tolerance** (pymupdf4llm):

```
Line 1: y0=100, "Hello"
Line 2: y0=104, "World"  ← Separate line (4pt gap > 3pt)
Result: "Hello" then "World"
```

The 5pt tolerance was a workaround for PDFium bbox variations, but it causes line merging.

---

## Prioritized Actions

| Priority | Action                       | Impact on Quality            |
| -------- | ---------------------------- | ---------------------------- |
| 1        | Implement smart sort key     | High (fixes ROUGE-L)         |
| 2        | Change line_tolerance to 3pt | Medium (fixes line grouping) |
| 3        | Run evaluation               | Required (measure impact)    |
| 4        | Document changes             | Required (maintainability)   |

---

## Risk Assessment

| Risk                                  | Mitigation                        |
| ------------------------------------- | --------------------------------- |
| 3pt tolerance breaks on PDFium bboxes | Test on all 7 files, can revert   |
| Smart sort key O(n²) for n blocks     | Acceptable for <1000 blocks/page  |
| Breaking existing passing tests       | Run full test suite before commit |

---

## Expected Outcome

If smart sort key is implemented correctly:

- ROUGE-L should improve from 0.491 → 0.7+ (order preserved)
- Quality score should improve from 0.573 → 0.7+
- Word F1 should remain ~0.914 (content unchanged)
