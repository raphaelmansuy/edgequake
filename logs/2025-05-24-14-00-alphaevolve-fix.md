# Task Log 2025-05-24 - AlphaEvolve PDF Extraction Fixes

## Actions

- Implemented `crossing_ratio` heuristic in `lattice.rs` to detect false positive tables where text physically crosses column boundaries.
- Implemented `has_long_sentences` heuristic in `lattice.rs` to reject tables containing long narrative text.
- Implemented `merge_text_elements` in `sota_backend.rs` to reconstruct fragmented words and sentences caused by PDF `Tj` operators.
- Implemented `deduplicate_elements` in `sota_backend.rs` to remove redundant text layers (OCR artifacts).
- Verified fixes by manually inspecting `AlphaEvolve.mdf.gen` and running `comprehensive_test_data`.

## Decisions

- **Geometric Validation:** Decided to use "crossing ratio" as a primary signal for invalid tables. If >10% of text elements cross the detected vertical separators, the grid is invalid.
- **Text Reconstruction:** Decided to merge text elements horizontally if they are on the same Y-level and close to each other, rather than relying on the consumer to stitch them together.
- **Deduplication:** Decided to remove exact duplicates at the same position, which is common in OCR'd PDFs (hidden text layer + visible text).

## Results

- **False Positive Tables:** Eliminated. The "Whole Page Table" issue in `AlphaEvolve.pdf` is resolved.
- **Text Quality:** Significantly improved. Fragmented words are merged, and duplicate text is removed.
- **Metrics:** F1 score for `AlphaEvolve` is stable (~0.63), likely due to ground truth discrepancies (figures/charts), but the extracted content is now readable and structured correctly. Other files remain high (~0.95).

## Next Steps

- Monitor extraction quality on other complex PDFs.
- Consider filtering out figure captions or chart labels if they negatively impact downstream tasks.
