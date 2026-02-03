# OODA-14: Decide

## Decision: Dual Regex Pattern for Numbered Lists

### Approach

Implement two complementary patterns in `ListDetectionProcessor`:

1. **Primary Pattern** (unchanged): `r"^\d+[\.)]\s+"`
   - Matches: "1. Text", "2) Item"
   - Requires space after marker

2. **Secondary Pattern** (new): `r"^\d+\.[A-Z]"`
   - Matches: "1.Explore", "2.Begin"
   - Requires uppercase letter after period
   - Excludes: "1.1", "2.3" (decimal section numbers)

### Rationale

- Preserves all existing detection (no regression)
- Adds coverage for edge case PDFs
- Avoids unsupported regex lookahead
- Simple boolean OR in detection logic

### Implementation Changes

1. **structure_detection.rs**:
   - Add `number_no_space_regex` with pattern `r"^\d+\.[A-Z]"`
   - Update list detection condition to check both patterns

2. **markdown.rs**:
   - Already handles content extraction for both formats
   - Normalizes output to "N. content" with proper spacing

### Expected Outcome

- Fix numbered list detection for edge case PDFs
- Maintain section header detection quality
- No regression in overall quality metrics
