# OODA Iteration 19 - Act

## Changes Made

### 1. Extracted `has_prose_indicators()` Public Function

**File:** `src/processors/heading_classifier.rs` line ~7
**Change:** Added public function `has_prose_indicators(text: &str) -> bool` that detects
sentence patterns (articles/copulas followed by lowercase words).

### 2. Refactored `is_valid_heading_text()` to Use Shared Function (DRY)

**File:** `src/processors/heading_classifier.rs`
**Change:** Replaced inline prose detection with call to `has_prose_indicators()`.

### 3. Added Prose Check to `processor.rs` Font-Size H1 Detection

**File:** `src/processors/processor.rs` line ~617
**Change:** Added `!has_prose_indicators(text)` guard to the `ratio > 1.5 && is_short` check.
Before: ANY short text with font ratio > 1.5x became H1.
After: Only non-prose short text with font ratio > 1.5x becomes H1.

### 4. Added Prose Check to `structure_detection.rs` `headingish` Check

**File:** `src/processors/structure_detection.rs` line ~360
**Change:** Added `&& !has_prose_indicators(text)` to the `headingish` boolean.

### 5. Re-exported Function

**File:** `src/processors/mod.rs`
**Change:** Added `pub use heading_classifier::has_prose_indicators;`

### 6. New Tests (4 added)

- `test_prose_indicators_sentence_patterns` - Detects "This is the second", "It was a dark"
- `test_prose_indicators_heading_patterns` - Passes "Introduction", "What We Deliver"
- `test_prose_indicators_short_text` - Short text returns false (can't detect)
- `test_prose_indicators_uppercase_after_indicator` - "What Is AI" is NOT prose

## Test Results

```
test result: ok. 566 passed; 0 failed; 0 ignored; 0 measured
```

## Quality Improvement Evidence

### Two-Column Test PDF (003_two_columns.pdf)

**Before (IT18):**

```
# **This is the second**
**column.**
```

**After (IT19):**

```
**This is the second column.**
```

"This is the second" is no longer misclassified as H1 header. It correctly renders
as bold text and gets merged with "column." by BlockMergeProcessor.

## Commit

```
OODA-IT19: Shared prose indicator detection prevents prose-as-heading misclassification
```
