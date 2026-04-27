# OODA-12: DECIDE — Fix Column Assignment

## Prioritized Changes

### Priority 1: Fix Gap Zone Column Assignment (HIGH IMPACT)

**File**: `src/backend/text_grouping.rs`
**Lines**: 241-262

**Change**:

- Replace `if elem.x < column_boundary` with `if elem.x < page_center`
- Use page center (page_width/2) as the tie-breaker for gap zone elements
- This ensures elements at X > page_center go to RIGHT column

**Rationale**:

- Column boundary detection finds gap center, not column edge
- Right column content starts LEFT of the gap center
- Page center is a more reliable divider

### Priority 2: Add Reference Pattern to BlockMergeProcessor (MEDIUM IMPACT)

**File**: `src/processors/layout_processing.rs`
**Lines**: 207-220

**Change**:

- Add `is_academic_ref` check: `starts_with('[')` + contains digits + contains `]`
- Prevent merging when next block starts with `[N]` pattern

**Rationale**:

- Even if blocks are correctly separated, BlockMerge might re-merge them
- Academic references should never be merged with preceding text

## Risk Assessment

| Change                  | Risk | Mitigation                  |
| ----------------------- | ---- | --------------------------- |
| Page center tie-breaker | Low  | More reliable than boundary |
| Academic ref pattern    | Low  | Only affects `[N]` patterns |

## Expected Outcome

| Metric            | Current | Expected |
| ----------------- | ------- | -------- |
| v2 Structure      | 50.4%   | 55-60%   |
| Overall Structure | 74.5%   | 77-79%   |
| Overall Quality   | 79.1%   | 80-82%   |
