# OODA-23 Orientation

## Processing Pipeline Order

1. **HeaderDetectionProcessor** (`structure_detection.rs`) - runs early
   - Matches subsection patterns like `1.1 Motivation`
   - Uses font-size based detection (ratio > 1.2, 1.35, 1.6)
   - Sets `block.block_type = BlockType::SectionHeader`

2. **StyleDetectionProcessor** (`processor.rs`) - runs after
   - Detects headers based on font ratio and text patterns
   - Checks for title-case text with larger fonts
   - Also sets `BlockType::SectionHeader`

3. **SectionPatternProcessor** (`processor.rs`) - runs after
   - Uses regex for numbered sections
   - Falls back to HeadingClassifier for font-based detection
   - Strategy 4 uses `heading_classifier.rs`

## Why Initial Fix Was Insufficient

The initial fix was added only to:

- `heading_classifier.rs:is_valid_heading_text()` (Strategy 4)
- `processor.rs:SectionPatternProcessor` (Strategy 2 regex filter)

But figure captions were being classified in `structure_detection.rs` and `processor.rs:StyleDetectionProcessor` BEFORE reaching Strategy 4.

## Pattern Analysis

Figure captions have a consistent pattern:

- Start with "Fig." or "Figure" or "Table" or "Tab."
- Followed by a number and period
- Then the caption title

These are easily identifiable with a simple `starts_with` check on lowercase text.
