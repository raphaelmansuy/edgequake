# DECIDE Phase: Fix Proposals

## Fix Strategy

Apply fixes in order of impact and simplicity. Test after each fix.

---

## FIX-001: Improve Heading Detection Heuristics

### Problem

`looks_like_section` is too restrictive - only matches digit-prefixed or ALL_CAPS text.

### Solution

Expand heuristic to include common heading patterns:

1. Title case (first letter of major words capitalized)
2. Short text (< 80 chars)
3. No trailing punctuation
4. Larger font than body

### Code Change Location

`src/processors/processor.rs` lines 380-382

### Before

```rust
let looks_like_section = text.starts_with(|c: char| c.is_ascii_digit())
    || text.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c.is_ascii_digit());
```

### After

```rust
let looks_like_section = text.starts_with(|c: char| c.is_ascii_digit())
    || text.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c.is_ascii_digit())
    || (is_first_char_upper && !text.contains('@') && !text.ends_with('.'));
```

---

## FIX-002: Improve Page Number Filtering

### Problem

5% bottom margin (39.6pt on letter size) too small - page numbers may be at ~50pt.

### Solution A: Increase margin to 8%

```rust
let bottom_margin = page_height * 0.08; // Was 0.05
```

### Solution B: Add explicit page number pattern (preferred)

Check for standalone numbers 1-3 digits anywhere in bottom 10% of page.

### Code Change Location

`src/processors/layout_processing.rs` `is_margin_content()` function

### Implementation

```rust
// Add after line 332 (current footer check)
// Extended page number detection - check bottom 10%
let extended_footer = bbox.y1 <= page_height * 0.10;
if extended_footer {
    let text = trimmed;
    // Standalone page number: 1-4 digits, optionally with "Page" prefix
    let is_page_number = text.parse::<u32>().is_ok() && text.len() <= 4;
    let is_page_label = text.to_lowercase().starts_with("page ")
        && text[5..].trim().parse::<u32>().is_ok();
    if is_page_number || is_page_label {
        return true;
    }
}
```

---

## FIX-003: Add List Pattern Exclusion

### Problem

List items like "1. First item" detected as section headers because they start with digit.

### Solution

Add list pattern check before section header classification.

### Code Change Location

`src/processors/processor.rs` `detect_headers()` function

### Implementation

```rust
// Add at start of detect_headers()
// Exclude list items
let list_pattern = regex::Regex::new(r"^\d+\.\s").unwrap(); // lazy_static for perf
if list_pattern.is_match(text) {
    return; // Skip - it's a list item, not a header
}
```

---

## FIX-004: Normalize Bullet Characters

### Problem

PDF uses `•` (U+2022) but markdown needs `-` or `*`.

### Solution

Normalize bullets in ListDetectionProcessor or markdown renderer.

### Code Change Location

`src/renderers/markdown.rs` or `src/processors/structure_detection.rs`

### Implementation

In markdown renderer when outputting list items:

```rust
// Replace bullet characters with markdown dash
let text = block.text.trim_start_matches(|c| c == '•' || c == '◦' || c == '▪');
output.push_str("- ");
output.push_str(text.trim());
```

---

## FIX-005: Add Text-Based Table Detection

### Problem

Tables without vertical lines (booktabs style) not detected.

### Solution

Add column alignment detection using text position clustering.

### Code Change Location

`src/processors/table_detection.rs` `TextTableReconstructionProcessor`

### Approach

1. Group text elements by Y position (same row)
2. Find consistent X-position clusters across rows (columns)
3. If 3+ rows have 2+ aligned columns, treat as table

---

## FIX-006: Tune Paragraph Merge Threshold

### Problem

BlockMergeProcessor merges paragraphs that should be separate.

### Solution

Increase minimum Y-gap for merging based on line height.

### Code Change Location

`src/processors/layout_processing.rs` `BlockMergeProcessor`

---

## Implementation Order

1. FIX-002 (page numbers) - Quick win
2. FIX-001 (heading detection) - High impact
3. FIX-003 (list exclusion) - Prevents misclassification
4. FIX-004 (bullet normalization) - Polish
5. FIX-005 (text tables) - Complex but important
6. FIX-006 (paragraph merge) - Fine-tuning
