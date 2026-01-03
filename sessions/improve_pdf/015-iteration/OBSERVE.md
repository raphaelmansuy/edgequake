# OBSERVE: Loop 015

## Current Status

- **Previous Loop (014):** Fixed "Whole Page Table" bug by implementing center-point text assignment and adding heuristics to reject invalid tables.
- **Visual Result:** Output for `AlphaEvolve.pdf` is now clean Markdown with proper headers and paragraphs.
- **Metric Result:** `real_dataset_eval` reports `f1=0.000` for all files.
- **User Feedback:** "Don't cheat", "Avoid magic numbers", "Think by first principles".

## Anomalies

1.  **Zero Metrics:** Even with visually correct output, the F1 score is 0.0. This implies a disconnect between the generated output and the ground truth expected by the evaluator, or a bug in the evaluator itself.
2.  **Magic Numbers:** The fix in Loop 014 used hardcoded thresholds (`height > 300.0`, `cols > 30`). These are brittle.

## Hypothesis

1.  **Metric Failure:** The evaluator might be comparing against empty or mismatched ground truth files, or the tokenization/normalization logic is too strict.
2.  **Heuristic Fragility:** The current table rejection logic might fail on legitimate large tables (e.g., a full-page data table) or miss smaller false positives.

## Goal

1.  **Fix the Ruler:** Debug the evaluation metric to ensure it accurately reflects improvements.
2.  **Refine the Logic:** Replace magic numbers with first-principles geometric analysis (e.g., signal-to-noise ratio of the grid, text density).
