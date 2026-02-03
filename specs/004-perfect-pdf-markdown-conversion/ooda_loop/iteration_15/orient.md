# OODA-15: Orient

## Problem Analysis

### Why Stream Table Detection is Critical

Academic papers frequently use **borderless tables** for:

- Comparison tables (A vs B)
- Results tables (metrics across methods)
- Feature matrices
- Algorithm parameters

These tables have NO vector lines in the PDF - they rely on:

1. **Column alignment**: Text in same column shares X-coordinate
2. **Row alignment**: Text in same row shares Y-coordinate
3. **Consistent spacing**: Whitespace between columns

### First Principles: How Stream Detection Works (Camelot)

From Camelot documentation:

```
1. Words grouped into text rows based on Y-axis overlaps
2. Textedges calculated to guess table areas
3. Column count guessed from mode of words per row
4. Column X ranges calculated from word positions
5. Table formed using row Y ranges + column X ranges
```

Key insight: **Mode detection** - if most rows have N words at similar X positions, it's a table with N columns.

### Root Cause Chain

```
PDF extraction (lopdf)
     ↓
TextElement list (individual chars/words)
     ↓
Block formation (merges adjacent text)
     ↓ ← PROBLEM: Table columns merged here
Document structure
     ↓
Lattice table detection (no lines = no detection)
     ↓
Markdown rendering (table missed)
```

### Specific Issue Location

In `backend/extraction_engine.rs`:

- Text elements are grouped into blocks
- Adjacent text at similar Y positions is merged
- No check for column alignment gaps

## Strategy Options

### Option A: Stream Table Detection Processor (NEW)

- Add processor that analyzes block X-coordinates
- Detect table patterns from alignment
- **Complexity**: HIGH (full algorithm)
- **Risk**: May cause false positives

### Option B: Block Formation Column Awareness

- Modify block formation to respect column gaps
- Keep blocks separate when X-gap is significant
- **Complexity**: MEDIUM
- **Risk**: May break existing text merging

### Option C: Post-Processing Table Reconstruction

- Analyze extracted text for table patterns
- Use heuristics (pipe chars, consistent columns)
- **Complexity**: MEDIUM
- **Risk**: May not work for all table types

## First Principles Decision

Option B is most promising because:

1. Fixes problem at source (block formation)
2. Preserves cell boundaries from PDF
3. Enables both Lattice and Stream detection downstream
4. Lower complexity than full Stream algorithm

## Impact Assessment

If block formation preserves column gaps:

- Table cells remain as separate blocks
- TableDetectionProcessor can find multi-block rows
- Structure fidelity should improve significantly

## Algorithm Sketch

```
Block Formation with Column Awareness:
1. Sort text elements by Y position (row grouping)
2. For each row, sort by X position
3. Calculate gaps between adjacent elements
4. If gap > column_threshold (e.g., 30pt):
   - Start new block
5. Merge elements within same column-block
```
