# OODA-23 Action

## Changes Made

### 1. `structure_detection.rs` - HeaderDetectionProcessor

Added filter at line ~135, after `is_short_for_heading` check:

```rust
// OODA-23: Filter out figure/table captions
let text_lower = text.to_lowercase();
let is_caption = text_lower.starts_with("fig.")
    || text_lower.starts_with("figure")
    || text_lower.starts_with("table")
    || text_lower.starts_with("tab.");
if is_caption {
    continue; // Don't classify captions as headers
}
```

### 2. `processor.rs` - StyleDetectionProcessor

Added filter at line ~409, after `is_list_item` check in `detect_headers_with_context`:

```rust
// OODA-23: Filter out figure/table captions
let is_caption = text_lower.starts_with("fig.")
    || text_lower.starts_with("figure")
    || text_lower.starts_with("table")
    || text_lower.starts_with("tab.");
if is_caption {
    return; // Don't classify captions as headers
}
```

### 3. `heading_classifier.rs` - is_valid_heading_text (previous fix)

Already had the filter in `is_valid_heading_text`:

```rust
// OODA-23: Filter out figure/table captions
let lower = text.to_lowercase();
if lower.starts_with("fig.")
    || lower.starts_with("figure")
    || lower.starts_with("table")
    || lower.starts_with("tab.") {
    return false;
}
```

## Results

**Before fix**: 5 figure captions as H3 headings in `agent_2510.09244v1.md`
**After fix**: 0 figure captions as H3 headings

Figure captions now appear as body text with emphasis:

```
*Fig. 1. Key Components of an Agent's LLM Architecture*
```

## Quality Impact

Overall quality score: 87.5% (unchanged)

Note: No improvement in overall score because other heading issues exist (Keywords being classified as heading, Abstract having extra text merged in). These are separate issues to address in future OODA cycles.

## Files Changed

- `src/processors/structure_detection.rs` - +12 lines
- `src/processors/processor.rs` - +9 lines
- `src/processors/heading_classifier.rs` - +13 lines (previous session)
