# IT29 — Decide: Fix Column Detection False Positives

## Decision

Add two independent guards in `GeometricClusterer::detect_columns()`
(src/layout/geometric.rs) to prevent single-column documents with
indentation from being misclassified as multi-column.

## Changes

### 1. Inter-cluster gap validation (geometric.rs, detect_columns())

After DBSCAN returns 2+ clusters, sort by center_x and check that
ALL adjacent cluster centers are separated by ≥15% of page_width.

If any pair is too close → collapse to single column.

### 2. Column balance validation (geometric.rs, detect_columns())

After clusters_to_columns() builds column regions, check that
max_width / min_width ≤ 3.0.

If ratio exceeds 3.0 → collapse to single column.

### 3. Regression test (geometric.rs, tests)

Add `test_indented_single_column_not_split()` with 14 bboxes:

- 7 at x=72 (headings)
- 7 at x=94 (indented bullets)
- Page width 612pt
- Assert: 1 column detected

## What NOT to change (deferred to IT30)

- Header over-promotion in `convert_standalone_bold_to_headers()` —
  needs first-principles font-size classification earlier in pipeline,
  not keyword heuristics in the renderer
- Bold text preservation — our output is richer than gold, which is good

## Expected outcome

- Content ordering fixed: "Co‑creation outputs (examples)" before its bullets
- All existing tests pass (no regression)
- New test validates the fix
