# OODA-16: Observe Phase

## Mission Status

- **Quality**: Text 85.0%, Structure 81.2%, Overall 83.1%
- **Target**: 95%+
- **Gap**: 11.9 percentage points

## Problem Identified

Table 1 in AlphaEvolve (FunSearch vs AlphaEvolve comparison) is NOT being detected as a table.

### Root Cause Analysis

```
┌────────────────────────────────────────────────────────────────────┐
│                    AlphaEvolve PDF - Page 2                        │
├────────────────────────────────────────────────────────────────────┤
│  Column 1 (x=62-289)        │  Column 2 (x=288-530)               │
│  ┌──────────────────────┐   │                                      │
│  │ ... paragraph ...    │   │  ... paragraph ...                  │
│  └──────────────────────┘   │                                      │
│  ┌──────────────────────┐   │  ┌──────────────────────┐           │
│  │ FunSearch[83]        │   │  │ AlphaEvolve           │           │
│  │ evolves single fn    │   │  │ evolves entire file   │           │
│  │ ...                  │   │  │ ...                   │           │
│  └──────────────────────┘   │  └──────────────────────┘           │
│  Table 1|Capabilities...    │  AlphaEvolve and our...             │
└────────────────────────────────────────────────────────────────────┘
```

The blocks ARE correctly separated (left column at x=69.9, right column at x=288.3), but:

**Issue**: `TableDetectionProcessor` skips ALL multi-column pages!

From [table_detection.rs](../../edgequake/crates/edgequake-pdf/src/processors/table_detection.rs#L62-L67):

```rust
// Skip multi-column layouts to avoid treating columns as table
if page.columns.len() > 1 {
    tracing::info!("  Skipping multi-column page ({} columns)", page.columns.len());
    continue;
}
```

### Why This Design Existed

Original reasoning was sound:

- In multi-column layouts, paragraph text appears side-by-side
- Without the skip, column text would be wrongly detected as "table rows"
- BUT: This also prevents detecting REAL tables within columns

### Observed Block Layout (Page 2, AlphaEvolve)

```
Block 15: x1=69.9  "FunSearch[83]"                    <- Table header, left col
Block 16: x1=69.9  "evolves single function..."       <- Table content, left col
Block 17: x1=69.9  "evolves code in Python..."        <- Table content, left col
Block 29: x1=288.3 "AlphaEvolve"                      <- Table header, right col
Block 30: x1=288.3 "evolves entire code file"         <- Table content, right col
Block 31: x1=288.3 "evolves any language"             <- Table content, right col
```

The blocks ARE properly aligned at two X-positions (69.9 and 288.3), which IS table structure!

## Key Insight

The current logic is:

- Multi-column page → skip table detection entirely

But it should be:

- Multi-column page → detect tables WITHIN each column

A table within a 2-column layout:

- Has blocks at X positions within ONE column's bounds (e.g., 62-289 for left column)
- Has consistent Y-alignment like any other table
- Should be processed the same as single-column tables

## Data to Verify

```
AlphaEvolve Column Bounds:
- Column 1: x = 62 to 289 (width ~227pt)
- Column 2: x = 288 to 530 (width ~242pt)

Table 1 Blocks:
- Left side: x = 69.9 (within Column 1: 62-289) ✓
- Right side: x = 288.3 (within Column 2: 288-530) ✓

BUT these are SAME Y-position (412.9, 432.4, etc.)!
```

Wait - if left and right table cells are at the SAME Y but in DIFFERENT columns, the current design correctly sees them as separate column content, not table cells.

**NEW INSIGHT**: This is actually a 2-column TABLE that spans BOTH columns! The table cells at y=412 are:

- Left: "FunSearch[83]" at x=69.9
- Right: "AlphaEvolve" at x=288.3

This is a borderless table that spans the FULL page width, not contained within a single column.

## Revised Problem Statement

Table 1 in AlphaEvolve is a **full-width borderless table** that happens to coincide with the 2-column layout. The table detection should:

1. NOT skip multi-column pages entirely
2. Detect tables that span multiple columns
3. Avoid false positives from regular column text

## Metrics Impact

If we fix table detection for multi-column pages:

- AlphaEvolve Structure could improve from 76.2% to ~85%+ (Table 1 properly formatted)
- Other multi-column papers may also benefit
