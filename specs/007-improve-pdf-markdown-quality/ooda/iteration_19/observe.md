# OODA Iteration 19 - Observe

## Finding: Prose Indicator Gap in Structure Detection

### Two Heading Detection Paths

The codebase has TWO independent heading detection mechanisms:

1. **`heading_classifier.rs`** (`HeadingClassifier::classify`) - Used by `processor.rs` Strategy 4
   - Has `is_valid_heading_text()` with comprehensive prose detection:
     - Checks for prose indicators ("is", "the", "a", "an", "this", "that", "as", "are", "was")
     - Rejects text where prose indicator at position ≥1 is followed by lowercase word
     - Example: "This is the second" → REJECTED (✓ correct)

2. **`structure_detection.rs`** (font-size-based detection, line ~372)
   - Uses simpler `headingish` boolean:
     - Not empty, < max_len, no '@', no '.', no ','
     - No URL/identifier, no sentence boundary
   - Missing: No prose indicator detection!
   - Example: "This is the second" → ACCEPTED (✗ wrong)

### Impact

The structure_detection processor runs BEFORE the heading classifier processor.
So when structure_detection classifies "This is the second" as SectionHeader(H1),
the heading classifier never gets a chance to reject it.

### Evidence

Two-column test PDF (003_two_columns.pdf):

```
AFTER-PAGE1 block 4 (SectionHeader) lvl=Some(1): 'This is the second'
AFTER-PAGE1 block 5 (Text) lvl=None: 'column.'
```

Expected: "This is the second column." as a single Text block, NOT a header.

### Affected Documents

Any PDF where body text happens to:

- Be in a larger font (e.g., title-zone of columns)
- Not contain commas or periods
- Be short enough to pass length check
