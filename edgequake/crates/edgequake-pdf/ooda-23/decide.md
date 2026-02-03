# OODA-23 Decision

## Chosen Approach: Multi-Point Caption Filtering

Add figure/table caption filtering in ALL heading detection paths:

1. **`structure_detection.rs:HeaderDetectionProcessor`**
   - Add filter early, right after list item check
   - Skip blocks where text starts with "fig.", "figure", "table", "tab."

2. **`processor.rs:StyleDetectionProcessor::detect_headers_with_context`**
   - Add same filter after list item check
   - Return early if text is a caption

3. **`heading_classifier.rs:is_valid_heading_text`** (already done)
   - Backup filter for any missed cases
   - Returns false for caption patterns

## Why This Approach

- **Defense in depth**: Catches captions regardless of which processor runs first
- **Simple check**: `text_lower.starts_with("fig.")` is O(1) and very fast
- **No false positives**: Real section headers never start with "Fig." or "Table"
- **Consistent**: Same filter logic in all three places

## Alternatives Considered

1. **Single filter in one processor**: Would miss captions if other processor runs first
2. **BlockType::Caption preassignment**: Would require another preprocessing pass
3. **Regex-only filtering**: More complex, same result
