# OODA Iteration 19 - Decide

## Decision: Add `has_prose_indicators()` Public Function

### Changes

1. **`heading_classifier.rs`**: Extract prose indicator logic into a public standalone
   function `has_prose_indicators(text: &str) -> bool` that can be called from
   structure_detection.

2. **`structure_detection.rs`**: Add `has_prose_indicators()` check to the `headingish`
   boolean computation at line ~360.

### Function Specification

```rust
/// Check if text contains prose indicator patterns that suggest it's a sentence,
/// not a heading. Uses first-principles: articles and copulas followed by
/// lowercase words indicate sentence structure.
///
/// Returns true if text looks like prose (NOT a heading).
pub fn has_prose_indicators(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 3 { return false; }
    for i in 1..words.len().min(5) {
        let word_lower = words[i].to_lowercase();
        let is_indicator = matches!(word_lower.as_str(),
            "the" | "a" | "an" | "it" | "this" | "that" | "as" | "is" | "are" | "was"
        );
        if is_indicator && i + 1 < words.len() {
            if words[i+1].chars().next().map(|c| c.is_lowercase()).unwrap_or(false) {
                return true;
            }
        }
    }
    false
}
```

### Test Cases

| Input                       | Result | Reason                     |
| --------------------------- | ------ | -------------------------- |
| "Introduction"              | false  | Single word                |
| "This is the second"        | true   | "is" + "the" (lowercase)   |
| "What We Deliver"           | false  | "We" is uppercase          |
| "Methods and Results"       | false  | "and" not in indicator set |
| "It was a dark"             | true   | "was" + "a" (lowercase)    |
| "Architecture & Governance" | false  | No indicators              |

### Risk Assessment

- **Low risk**: Only affects prose-like text with large fonts
- **No false negatives**: Real headings don't contain these patterns
- **DRY**: Shared logic between two classifiers
