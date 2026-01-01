# Test Plan: SOTA Table Extraction

## Target Document

`edgequake/crates/edgequake-pdf/test-data/real_dataset/one_tool_2512.20957v2.pdf`

## Success Criteria

1.  **Table 1 (Comparison of different agent pipelines):**
    - Must be extracted as a Markdown table.
    - Columns: Agent Pipeline, Model, Function-level (Recall, Precision, F1, IoU), File-level (Recall, Precision, F1, IoU).
    - Rows: RepoSearcher, RepoNavigator, etc.
    - **Critical:** Multi-row headers must be handled or flattened reasonably.
2.  **Table 2 (SWE-bench Pro):**
    - Similar structure to Table 1.
    - Must not be mixed with body text.
3.  **Table 3 (Issue Resolution):**
    - Smaller table.
4.  **Table 4 (Ablation Study):**
    - Checkmarks (✓/✗) handling.
5.  **No Regression:**
    - Two-column body text must still be extracted correctly.
    - Footers must still be filtered.

## Test Procedure

1.  **Run Extraction:**
    ```bash
    cargo run --example sota_test
    ```
2.  **Inspect Output:**
    - Check `one_tool_2512.20957v2.md`.
    - Search for `|` characters (Markdown table indicators).
    - Verify content alignment.
3.  **Automated Check (Optional):**
    - Write a script to parse the Markdown and verify table dimensions.

## Iteration Strategy (OODA)

1.  **Observe:** Run the extractor. Look at the Markdown.
2.  **Orient:** Identify broken tables (merged columns, split rows, missing headers).
3.  **Decide:** Adjust `x_gap_threshold`, `y_overlap_threshold`, or `table_density_threshold`.
4.  **Act:** Apply code change and re-run.
