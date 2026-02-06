# IT29 — Act: Column Detection False Positive Fix

## Changes made

### File: `src/layout/geometric.rs` (detect_columns method)

**Guard 1: Inter-cluster gap validation** (after DBSCAN, before clusters_to_columns)

```
Location: GeometricClusterer::detect_columns(), after `let clusters = self.dbscan()`
```

Added validation that checks adjacent DBSCAN cluster centers are separated by
≥15% of page_width (~92pt for US Letter). If any adjacent pair is too close,
collapse to single column.

WHY: DBSCAN splits indentation patterns (x=72 headings vs x=94 bullets, gap=22pt)
into separate clusters, but this is NOT a column boundary. Real columns have
cluster centers ~250pt apart.

**Guard 2: Column balance validation** (after clusters_to_columns, before return)

Added validation that max_column_width / min_column_width ≤ 3.0. If ratio
exceeds 3.0, collapse to single column.

WHY: False positive columns create extreme imbalance (e.g., 94pt vs 518pt = 5.5:1).
Real multi-column layouts have balanced columns (ratio ≤1.5 typically).

**Test: `test_indented_single_column_not_split`** (tests module)

14 bboxes simulating a single-column document with headings at x=72 and
indented bullets at x=94 on a 612pt page. Asserts 1 column detected.

## Test results

```
570 passed; 0 failed; 0 ignored
```

All tests pass including the new `test_indented_single_column_not_split`.

## Output quality comparison

### Before (IT28 output):
Content ordering WRONG — "Co‑creation outputs (examples)" displaced after its bullet items
because ReadingOrderDetector used multi_column_order with 2 false columns.

### After (IT29 output):
Content ordering CORRECT:
```
Co‑creation workshops: fast alignment through working sessions...
Co‑creation outputs (examples)         ← now in correct position
Use-case portfolio with effort/impact...
"Thin vertical slice" plan...
Blueprint pack...
```

## Remaining issues (deferred to IT30)

- Header over-promotion: `convert_standalone_bold_to_headers()` promotes too many
  bold lines to `##` headers. Needs font-size-based classification earlier in pipeline.
- Section number format: `0)` vs `0.` — minor formatting difference from gold.

## Commit

Parent: `3deab994` (IT28)
