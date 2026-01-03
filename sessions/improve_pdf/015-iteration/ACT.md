# ACT: Loop 015

## Actions Taken

1.  **Fixed Evaluator:** Updated `real_dataset_eval.rs` to look for `.gold.md` or `.md` files, fixing the "F1=0.0" issue.
2.  **Refined Heuristics:** Replaced magic numbers in `lattice.rs` with first-principles geometric checks:
    - **Row Height:** Reject if `avg_row_height > 200pt` (likely page wrapper).
    - **Column Width:** Reject if `avg_col_width < 10pt` (likely grid noise).
    - **Aspect Ratio:** Reject if `cell_aspect_ratio < 0.05`.
    - **Empty Cell Ratio:** Reject if `> 90%` cells are empty.
3.  **Fixed Logic Bug:** Removed erroneous `if !tables.is_empty() { return ... }` in `sota_backend.rs` which caused pages with tables to lose all non-table text.
4.  **Improved Return Type:** Changed `create_table_block` to return `None` on validation failure, preventing "fallback tables" from polluting the table list.

## Results

- **Metrics:**
  - `2900_Goyal_et_al`: F1 **0.948**
  - `agent_2510`: F1 **0.955**
  - `ccn_2512`: F1 **0.833**
  - `AlphaEvolve`: F1 **0.630** (Precision 0.467, Recall 0.967)
- **Visuals:** "Whole Page Table" is gone. Real tables are preserved. Text is present but fragmented.

## Next Steps

- Proceed to Loop 016 to address text fragmentation and low precision.
