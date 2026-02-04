````markdown
# OODA-07 Decide: Fix CamelCase Splitting

## Decision

Modify the regex to only split at word boundaries, preserving standalone CamelCase terms.

## Implementation Plan

### Change 1: Update regex pattern

**File:** `text_cleanup.rs:377-380`

**Current:**

```rust
// Fix lower-UPPER-lower pattern
if let Ok(re) = Regex::new(r"([a-z])([A-Z][a-z])") {
    result = re.replace_all(&result, "$1 $2").to_string();
}
```
````

**New:**

```rust
// OODA-07: Only split concatenated words at word boundaries
// This preserves CamelCase terms like BrowseComp, DeepHalluBench
// Pattern: lowercase word ending + Uppercase (after space/punctuation or at inline position)
// Match: "text methodsThe model" → "text methods The model"
// Preserve: "BrowseComp" (no preceding space to match)
if let Ok(re) = Regex::new(r"(\s)([a-z]+)([A-Z][a-z])") {
    result = re.replace_all(&result, "$1$2 $3").to_string();
}
```

### Change 2: Add additional CamelCase repairs

Add common terms that might still get split (defensive):

```rust
// Repair common CamelCase terms (defensive)
result = result.replace("Browse Comp", "BrowseComp");
result = result.replace("Report Bench", "ReportBench");
result = result.replace("Deep Hallu", "DeepHallu");
```

### Change 3: Add test case

Add test to verify CamelCase preservation:

```rust
#[test]
fn test_camelcase_preservation() {
    let processor = PostProcessor::default();
    // Should NOT split CamelCase terms
    assert_eq!(processor.fix_concatenated_words("BrowseComp"), "BrowseComp");
    assert_eq!(processor.fix_concatenated_words("DeepHalluBench"), "DeepHalluBench");
    // SHOULD split concatenated sentence words
    assert_eq!(processor.fix_concatenated_words("text methodsThe model"), "text methods The model");
}
```

## Expected Outcome

- CamelCase terms preserved: BrowseComp, DeepHalluBench, ReportBench
- Concatenated words still fixed: "methodsThe" → "methods The" (when preceded by space)
- No regression in existing functionality

```

```
