# DECIDE: Loop 015

## Analysis

- **Metrics Fixed:** The evaluator now correctly finds the ground truth.
- **Heuristics Validated:** The geometric heuristics (Row Height < 200pt, Col Width > 10pt) successfully rejected the "Whole Page Table" false positive.
- **Bug Fixed:** The critical "Early Return" bug in `sota_backend.rs` was identified and fixed, allowing mixed content (tables + text) to be processed correctly.
- **Remaining Issues:**
  - `AlphaEvolve` still has low precision (0.467), indicating excessive text extraction or duplication.
  - Some false positive tables remain (e.g., math symbols interpreted as grid).
  - Text fragmentation is still present (broken paragraphs).

## Decision

- **Close Loop 015:** The primary goal (fix table detection and metrics) is achieved.
- **Start Loop 016:** Focus on **Text Fragmentation and Precision**. The low precision suggests we are generating too much noise or failing to merge text blocks correctly.

## Plan for Loop 016

1.  **Investigate Fragmentation:** Why are paragraphs broken into small chunks?
2.  **Investigate Duplication:** Are we extracting the same text multiple times (e.g. once as table, once as text)?
3.  **Refine Layout Analysis:** Improve `group_into_lines` or paragraph merging logic.
