# OODA-41: Observe - 3-Phase Rectangle Joining Algorithm

## Date: 2026-02-04

## Mission Reference

- Target: F1 ≥ 0.90 vs pymupdf4llm gold standards
- Current: F1 = 0.686 (average)
- Gap: 21.4 percentage points

## Key Observation from pymupdf4llm Source

### `multi_column.py` Algorithm Flow

```
┌────────────────────────────────────────────────────────────────────────────┐
│  INPUT: Text blocks from page.get_text("dict")["blocks"]                   │
└────────────────────────────────┬───────────────────────────────────────────┘
                                 ▼
┌────────────────────────────────────────────────────────────────────────────┐
│  STEP 1: Extract text bboxes, filter out vertical text                    │
│  - Only (almost) horizontal text: abs(1 - line.dir[0]) > 1e-3             │
│  - Ignore image text if no_image_text=True                                │
└────────────────────────────────┬───────────────────────────────────────────┘
                                 ▼
┌────────────────────────────────────────────────────────────────────────────┐
│  STEP 2: Sort bboxes by (background_color, y0, x0)                         │
│  - Different backgrounds tracked separately                                │
│  - Important for sidebars, callout boxes                                  │
└────────────────────────────────┬───────────────────────────────────────────┘
                                 ▼
┌────────────────────────────────────────────────────────────────────────────┐
│  STEP 3: Initial block joining (can_extend check)                          │
│  - Never join across different columns (x1 < x0)                          │
│  - Never join across different backgrounds                                │
│  - Check for conflicts with other blocks                                  │
└────────────────────────────────┬───────────────────────────────────────────┘
                                 ▼
┌────────────────────────────────────────────────────────────────────────────┐
│  PHASE 1: join_rects_phase1() - Disabled currently (too aggressive)       │
│  - Join rects that "touch" (valid intersection)                           │
│  - Allow gap <= 10pt vertically                                           │
└────────────────────────────────┬───────────────────────────────────────────┘
                                 ▼
┌────────────────────────────────────────────────────────────────────────────┐
│  PHASE 2: join_rects_phase2() - CRITICAL                                  │
│  - Align column boundaries: x0 and x1 within 3pt → normalize              │
│  - Sort by (x0, y0)                                                       │
│  - Join if: |r.x0 - r0.x0| <= 3 AND |r.x1 - r0.x1| <= 3 AND              │
│            |r0.y1 - r.y0| <= 10  (gap between bottom and top)            │
└────────────────────────────────┬───────────────────────────────────────────┘
                                 ▼
┌────────────────────────────────────────────────────────────────────────────┐
│  PHASE 3: join_rects_phase3() - SMART SORTING                             │
│  - Never join across columns (x1 < x0 OR x1 < x0)                         │
│  - Never join different background colors                                 │
│  - Sort key: for each box, find left-most rect with vertical overlap     │
│    - If found: key = (left_rect.y0, box.x0)                              │
│    - Else: key = (box.y0, box.x0)                                        │
│  - This ensures "Q" (right) comes after "P" (left) when overlapping      │
└────────────────────────────────┬───────────────────────────────────────────┘
                                 ▼
┌────────────────────────────────────────────────────────────────────────────┐
│  OUTPUT: Sorted list of column-aware bboxes in reading order              │
└────────────────────────────────────────────────────────────────────────────┘
```

## Our Current Implementation

### `reading_order.rs` Flow

```
┌────────────────────────────────────────────────────────────────────────────┐
│  INPUT: Block objects from extraction engine                               │
└────────────────────────────────┬───────────────────────────────────────────┘
                                 ▼
┌────────────────────────────────────────────────────────────────────────────┐
│  STEP 1: assign_to_column() for each block                                │
│  - Uses bbox.intersects(column) check                                     │
│  - Has spanning/footer heuristics                                         │
│  - NO boundary normalization                                              │
└────────────────────────────────┬───────────────────────────────────────────┘
                                 ▼
┌────────────────────────────────────────────────────────────────────────────┐
│  STEP 2: sort_by_position() per column                                    │
│  - Sort by (y1, x1)                                                       │
│  - NO tolerance-based grouping                                            │
└────────────────────────────────┬───────────────────────────────────────────┘
                                 ▼
┌────────────────────────────────────────────────────────────────────────────┐
│  STEP 3: merge_column_orders() with spanning elements                     │
│  - Simple Y-threshold interleaving                                        │
│  - NO smart sort key like pymupdf4llm                                     │
└────────────────────────────────────────────────────────────────────────────┘
```

## Critical Gaps Identified

| Feature                                | pymupdf4llm | edgequake-pdf | Impact               |
| -------------------------------------- | ----------- | ------------- | -------------------- |
| Boundary normalization (3pt tolerance) | ✅          | ❌            | Columns misaligned   |
| Vertical gap tolerance (10pt for join) | ✅          | ❌            | Blocks over-split    |
| Smart sort key (left-rect.y0, box.x0)  | ✅          | ❌            | Wrong reading order  |
| Background color tracking              | ✅          | ❌            | Sidebars interleaved |
| Never-join-across-columns check        | ✅          | Partial       | Column bleeding      |

## Files to Modify

1. `src/layout/reading_order.rs` - Add 3-phase joining
2. `src/layout/column_detector.rs` - Add boundary normalization
3. `src/backend/text_grouping.rs` - Use new column-aware sorting

## Test Document for Validation

- `01_2512.25075v1.pdf` - Worst performer (F1=0.553), two-column arXiv paper
- Current issues: Column interleaving, text jumping between columns

## Evidence

```bash
# Current F1 scores
python3 scripts/compare_against_pymupdf.py --pdf-dir edgequake/crates/edgequake-pdf/test-data/real_dataset

Average F1: 0.686
01_2512.25075v1: F1=0.552  ← Worst, primary target
one_tool_2512.20957v2: F1=0.596
AlphaEvolve: F1=0.620
```
