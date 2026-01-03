# Loop 014 - OBSERVE Phase

## Objective

Diagnose the root cause of table cell text assignment failure. Loop 013 proved that tolerance tuning is insufficient. We suspect a fundamental coordinate system mismatch between detected grid lines and text elements.

## Current State

- **Table Accuracy:** 2.4%
- **Issue:** Column boundaries are detected (structure looks okay), but text is almost exclusively assigned to the first column or missing.
- **Hypothesis:**
  1.  **Coordinate System Mismatch:** PDF Y-axis (bottom-up) vs. internal logic (top-down) confusion.
  2.  **Grid Misalignment:** Vertical lines (used for grid) might be visually distinct from text alignment (e.g., text is centered, lines are borders).
  3.  **TextElement Coordinates:** The `x, y` in `TextElement` might be relative to a different origin or transformed.

## Action Plan

1.  **Instrument Code:** Add detailed logging to `lattice.rs` inside `create_table_block` and `extract_text_in_rect`.
    - Log Table BBox.
    - Log calculated `unique_x` (cols) and `unique_y` (rows).
    - Log a sample of `TextElement` coordinates inside the table area.
    - Log the specific bounds being checked for a specific cell (e.g., row 0, col 1) and why text fails the check.
2.  **Run Evaluation:** Execute `cargo run -p edgequake-pdf --example real_dataset_eval -- --write` and capture the output.
3.  **Analyze:** Compare the logged coordinates to understand the offset/mismatch.

## Expected Outcome

Clear evidence of why `x=150` (text) is not falling into `x=[140, 160]` (cell), or similar discrepancies.
