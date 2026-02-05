# OODA-11: Observe - Undocumented Constants in markdown.rs

## Current State

Several constants in `markdown.rs` lack WHY comments:

1. **line 248**: `(indent - 72.0).max(0.0) / 20.0` - list indentation calculation
   - 72.0: no explanation
   - 20.0: no explanation

2. **line 601**: `(y - prev_y).abs() > 10.0` - table row detection
   - 10.0: no explanation

3. **line 916/921**: Test constants (612.0, 792.0, 72.0) - standard page dimensions

## Evidence

Many constants already have WHY comments:

- ✅ line 187: skip_bold explanation
- ✅ line 198: bold headers explanation
- ✅ line 260: subtract 1 for level explanation
- ❌ line 248: 72.0 and 20.0 undocumented
- ❌ line 601: 10.0 undocumented

## Analysis

These constants relate to:

- PDF page margins (72.0pt = 1 inch = standard margin)
- List indentation steps (20.0pt = ~0.28 inch indent per level)
- Table row Y-tolerance (10.0pt = same as other tolerances)

## Data Needed

- Confirm 72.0pt is standard PDF margin
- Confirm 20.0pt is standard indent step
- Confirm 10.0pt threshold matches other tolerances
