# OODA-19: Orient Phase

## Analysis

### Why Rotated Text Is Problematic

1. **Y-Coordinate Collision**:
   - Rotated margin text has Y coordinates that match body text
   - Line grouping algorithm groups elements by Y (±tolerance)
   - Result: Margin annotations get merged into paragraphs

2. **Current Processing Pipeline**:
   ```
   PDF → ContentParser → Elements → ElementProcessor → TextGrouper → BlockBuilder
                         ↑
                         Rotation information lost here
   ```

### Strategic Options

| Option              | Description                  | Effort | Impact                                          |
| ------------------- | ---------------------------- | ------ | ----------------------------------------------- |
| A. Filter rotated   | Remove rotated text entirely | Low    | Medium (fixes inline issue, but loses metadata) |
| B. Relocate rotated | Move to document start       | Medium | High (matches gold file expectation)            |
| C. Separate channel | Extract as metadata          | Medium | Medium (structured but not in body)             |

### Decision Factors

1. **Gold file expects arXiv at top** → Option B matches expectation
2. **Implementation complexity** → Option A is simplest
3. **Incremental approach** → Start with A, improve later

## Technical Design

### Option A: Filter Rotated Text (Implemented)

1. Add `is_rotated` field to `TextElement`
2. Detect rotation via CTM: `|ctm[0]| < 0.1 && |ctm[3]| < 0.1`
3. Filter out rotated elements in `extraction_engine.rs`

### Files Modified

1. `backend/elements.rs`: Added `is_rotated: bool`
2. `backend/content_parser.rs`: Added `is_rotated_ctm()` detection
3. `backend/extraction_engine.rs`: Filter rotated elements with logging
4. Test files: Added `is_rotated: false` to test helper functions
