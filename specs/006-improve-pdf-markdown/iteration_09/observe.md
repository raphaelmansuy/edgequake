# OODA-09: Observe - Undocumented Magic Numbers in text_grouping.rs

## Current State

The `text_grouping.rs` file has several magic numbers without WHY comments:

1. **Line 307**: `if elem.y < 100.0` - No explanation why 100.0pt
2. **Line 407**: `elem.y > 15.0 && elem.y < 80.0` - Author zone bounds
3. **Line 413**: `left.y > 15.0 && left.y < 80.0` - Same author zone
4. **Line 422**: `elem.text.len() < 30 && elem.y > 20.0` - Text length threshold
5. **Line 566-567**: `30.0` gap threshold for vertical split

## Evidence

From grep results, many numbers have WHY comments already:
- ✅ Line 141: Y-normalization explanation
- ✅ Line 145: Figure caption position
- ✅ Line 245: REFERENCES section
- ❌ Line 307: 100.0 undocumented
- ❌ Line 407: 15.0, 80.0 undocumented
- ❌ Line 566: 30.0 undocumented

## Context from Code

These numbers relate to:
- Academic paper page layout (US Letter: 792pt tall)
- Author name zones (typically top 10% of page)
- Vertical gap detection for section boundaries

## Data Needed

- Typical US Letter page dimensions: 612pt × 792pt
- Author zone: ~10-12% from top (60-80pt)
- Header zone: top 2% (~15pt)
- Significant vertical gap: ~4% of page height (~30pt)
