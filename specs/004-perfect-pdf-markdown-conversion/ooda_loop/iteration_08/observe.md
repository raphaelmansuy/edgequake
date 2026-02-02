# OODA-08 Observe: Table Structure Issues

## Problem Statement

Tables in two-column academic papers are being rendered with wrong structure:

- Side-by-side tables (Table 2, Table 3) merged into single wide table
- Caption text mixed with table headers
- 14+ columns detected when should be 4-5

## Evidence from Logs

```
Table passed all filters: bbox=BoundingBox { x1: 55.44, y1: 314.51, x2: 541.44, y2: 734.404 }, text_len=846
```

**Key Observation**: Table x-range spans 55.44 to 541.44 (~490 pts width), but column boundary is at 305.0

The table crosses the column boundary! In two-column layouts, a single table shouldn't span both columns unless it's a full-width table at the top/bottom of the page.

## Code Analysis

### 1. Lattice Engine (backend/lattice.rs)

Returns 0 tables for academic papers because they don't have graphical borders:

```
Lattice detected 0 tables on page 7
```

The lattice engine is NOT the source of these tables.

### 2. Table Filters (extraction_engine.rs:446-514)

Current filters check:

- ✅ Min size (50x50)
- ✅ Max size (80% of page)
- ✅ Edge proximity (20pt margin)
- ✅ Empty tables
- ✅ Text density
- ❌ **MISSING: Column boundary crossing check**

### 3. TableDetectionProcessor (processors/table_detection.rs)

This processor DOES skip multi-column pages (line 62-67):

```rust
if page.columns.len() > 1 {
    tracing::info!("Skipping multi-column page ({} columns)", page.columns.len());
    continue;
}
```

But this only affects the TABLE DETECTION PROCESSOR. The extraction engine's filter runs BEFORE this processor.

### 4. TextTableReconstructionProcessor

Creates tables from text patterns (caption + data rows). This could be creating tables that span columns.

## Root Cause Analysis

1. **Primary Issue**: Extraction engine filters tables BEFORE column detection
2. The column boundary is detected in `group_into_lines` which runs AFTER table filtering
3. Tables that should be split into left/right column tables are passed as single wide tables

## Timeline of Operations

```
1. extract_page() starts
2. lattice_engine.detect_tables() → 0 tables (no grid lines)
3. Filter tables by size/position ← MISSING column check here
4. group_into_lines() ← Column detection happens HERE
5. Processors run (TableDetectionProcessor, TextTableReconstructionProcessor)
```

The problem: Column detection happens AFTER table filtering.

## Sample Output (Broken Table)

```markdown
| midline | are | proprietary. | Bold | denotes | lowest | hallucination | score.Answerable | queries; | No-Ans.: | Adversarial | queries | (correct | re- |
| ------- | --- | ------------ | ---- | ------- | ------ | ------------- | ---------------- | -------- | -------- | ----------- | ------- | -------- | --- |
```

This is caption text from Table 2 and Table 3 merged and parsed as table row.

## Files to Modify

1. `backend/extraction_engine.rs:446-514` - Add column boundary check to table filter
2. `backend/column_detection.rs` - Ensure column detection can run independently

## Markitdown Reference

Markitdown does NOT produce structured markdown tables - it extracts raw text. The tables in the paper are rendered as plain text paragraphs.
