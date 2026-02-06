# OODA-38 Act

## Files Modified

### 1. `src/processors/layout_processing.rs`

- **SectionNumberMergeProcessor::process()** (line ~1245): Added two-mode matching
  - Mode A (same-line): `y_gap < 25.0 && title_block.bbox.x1 > sec_x`
  - Mode B (next-line): `y_gap < 40.0 && title_y_center > sec_y && (x1 - sec_x).abs() < 20.0`
  - Best-match tracking: `best_same_line.or(best_next_line)` ensures Mode A priority
- **looks_like_section_title()** (line ~1115): Added ALL-CAPS fast path
  - `alpha_chars.iter().all(|c| c.is_uppercase())` → return true

### 2. `src/processors/text_cleanup.rs`

- **is_garbled()** (line ~674): Removed outer guard, restructured checks:
  - Check (a): Per-word > 35 chars (lowered from 40)
  - Check (b): CamelCase > 25 chars with ≥ 2 internal uppercase
  - Proportion guard: garbled words must be > 50% of text OR text < 200 chars
- **Space-ratio check** (line ~730): Lowered threshold from 80 to 60 chars, added URL exception

## Tests Added (5 new, 462 total)

- `test_section_number_merge_same_line`: Mode A merge verification
- `test_section_number_merge_next_line`: Mode B merge verification
- `test_section_number_no_merge_far_below`: Y-gap rejection verification
- `test_garbled_camelcase_detection`: CamelCase word detection
- `test_garbled_word_in_long_paragraph_not_filtered`: Proportion guard

## Quality Improvements

| Metric                                     | Before                                   | After                                        |
| ------------------------------------------ | ---------------------------------------- | -------------------------------------------- |
| LightRAG output size                       | 58993 bytes                              | 58858 bytes (-135 bytes noise)               |
| Section "3.2"                              | Standalone                               | Merged: "3.2. DUAL-LEVEL RETRIEVAL PARADIGM" |
| Section "3.1"                              | "3.1. THE LIGHTRAG ARCHITECTURE" (wrong) | "3.1. GRAPH-BASED TEXT INDEXING" (correct)   |
| Sections 7.3.1-7.3.3                       | Standalone numbers                       | Properly merged with titles                  |
| "AgricultureEnvironmentalProductionImpact" | Visible                                  | Removed                                      |
| Elitizon regression                        | 5332 bytes, 84 blocks                    | 5332 bytes, 84 blocks (no change)            |

## Build & Test

- 462 tests passing, 0 failures
- 0 clippy warnings in edgequake-pdf
- Commit: OODA-38
