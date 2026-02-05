# OODA-10: Observe - Undocumented Constants in pymupdf_grouper.rs

## Current State

Several constants in `pymupdf_grouper.rs` lack WHY comments:

1. **line 110**: `column_overlap: 0.5` - no explanation
2. **line 302**: `COLUMN_GAP_THRESHOLD: f32 = 10.0` - has inline comment but no WHY
3. **line 409**: `VERTICAL_GAP_MAX: f32 = 10.0` - need to check if documented
4. **line 497**: `page_width < 100.0` - no explanation

## Evidence

From grep results, some constants have good WHY comments:
- ✅ line 102-106: line_tolerance explained
- ✅ line 107-109: block_gap explained  
- ✅ line 112-115: left_margin and right_margin explained
- ❌ line 110: column_overlap undocumented
- ❌ line 302: COLUMN_GAP_THRESHOLD inline only

## Analysis

These constants affect:
- Column detection (`column_overlap` controls horizontal overlap threshold)
- Multi-column line splitting (`COLUMN_GAP_THRESHOLD`)
- Page detection (`page_width < 100.0` for unusable pages)

## Data Needed

- Purpose of 0.5 (50%) column overlap threshold
- Relationship between COLUMN_GAP_THRESHOLD and block_gap
- Why 100.0pt is "too small" for a page
