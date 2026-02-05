# OODA-10: Orient - Document Constants in pymupdf_grouper.rs

## Analysis

### column_overlap: 0.5

**Purpose**: Controls same-column detection threshold
- 0.5 = 50% horizontal overlap required
- Two blocks are "same column" if their X ranges overlap by 50%+
- Lower value: more lenient, might merge columns
- Higher value: stricter, might fragment columns

**Rationale**: 50% is a good middle ground:
- Handles slight column offset (e.g., indented paragraphs)
- Prevents merging of adjacent columns
- Matches typical academic paper layouts

### COLUMN_GAP_THRESHOLD: 10.0

**Purpose**: Minimum gap to indicate column boundary
- 10pt is less than typical column gutter (14-20pt)
- Provides margin for detection uncertainty
- Word gaps are typically < 5pt

**Rationale**: 10pt threshold catches true column gaps while allowing:
- Word spacing variations (2-5pt)
- Wide character spacing (up to 10pt in some fonts)

### page_width < 100.0

**Purpose**: Detect unusable/empty pages
- 100pt ≈ 1.4 inches = too small for readable content
- Typical pages: US Letter (612pt), A4 (595pt)

## Prioritization

1. `column_overlap` - affects column detection accuracy
2. `COLUMN_GAP_THRESHOLD` - affects line splitting
3. `page_width < 100.0` - edge case handling
