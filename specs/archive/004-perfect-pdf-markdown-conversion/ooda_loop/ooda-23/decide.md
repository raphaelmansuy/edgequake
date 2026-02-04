# OODA-23 Decide: Hyphenation Merge Fix

## Decision

Implement hyphenation-aware block merging to rejoin words split across lines.

## Algorithm

```rust
fn should_merge_hyphenated(a: &Block, b: &Block, stats: &DocumentStats) -> bool {
    // WHY: PDF text extraction often splits hyphenated words across blocks
    // e.g., "reposito-" + "ries" → "repositories"

    // Condition 1: First block ends with hyphen
    if !a.text.trim_end().ends_with('-') {
        return false;
    }

    // Condition 2: Second block starts with lowercase letter
    // WHY: Proper nouns or new sentences start with uppercase
    let next_start = b.text.trim_start().chars().next();
    if !matches!(next_start, Some(c) if c.is_ascii_lowercase()) {
        return false;
    }

    // Condition 3: Reasonable vertical proximity
    // WHY: Same paragraph should be within 2x line height
    let gap = (b.bbox.y1 - a.bbox.y2).abs();
    let line_height = stats.avg_line_height;
    if gap > line_height * 2.5 {
        return false;
    }

    true
}

fn merge_hyphenated(a: &mut Block, b: &Block) {
    // Remove trailing hyphen and whitespace
    let text_a = a.text.trim_end().trim_end_matches('-');
    // Get continuation text
    let text_b = b.text.trim_start();
    // Join without hyphen (word continues)
    a.text = format!("{}{}", text_a, text_b);
    // Extend bbox to include both
    a.bbox = a.bbox.union(&b.bbox);
}
```

## Location

`src/processors/layout_processing.rs` in `BlockMergeProcessor::process()`

## Expected Outcome

- "reposito-" + "ries remains" → "repositories remains"
- "compli-" + "cate" → "complicate"

## Test Cases

1. Simple hyphenation: "pre-\nprocessing" → "preprocessing"
2. Legitimate hyphen preserved: "self-attention" (same block, no merge)
3. Cross-paragraph boundary: "end-\n\nNew paragraph" → no merge (gap too large)

## Metrics Target

- TPS improvement: +2-3%
- Overall quality: 80.8% → 82-83%
