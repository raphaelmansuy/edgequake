# OBSERVE: Loop 016

## Current Status

- **Previous Loop (015):** Fixed table detection and critical backend bug. Metrics are good for most files, but `AlphaEvolve` lags in precision.
- **Issue:** Text is highly fragmented. Paragraphs are broken into single lines or short chunks.
- **Metric:** `AlphaEvolve` Precision is 0.467 (very low). Output size is 2.5x ground truth.

## Evidence

- `AlphaEvolve.mdf.gen` shows:
  ```markdown
  **In this white paper, we present\*\*\***Alpha Evolve**\* **capabilities of state-of-the-art LLMs...\*\*
  **or optimizing critical pieces of computational infrastructure.**
  ```
  This should be a single paragraph.
- The file size (101KB vs 42KB) suggests massive duplication or noise.

## Hypothesis

1.  **Fragmentation:** The line merging logic (`group_into_lines` or similar) is failing to merge lines that are close together.
2.  **Duplication:** We might be extracting text from multiple layers (e.g. hidden OCR layer + visible text) or duplicating content during processing.
3.  **Formatting Noise:** Excessive use of `**bold**` might be confusing the tokenizer or just bloating the file.

## Goal

- **Merge Paragraphs:** Ensure continuous text is grouped into single blocks.
- **Reduce Noise:** Eliminate duplicates and excessive formatting.
- **Improve Precision:** Target F1 > 0.8 for `AlphaEvolve`.
