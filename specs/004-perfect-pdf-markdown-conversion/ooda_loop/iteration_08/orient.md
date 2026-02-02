# OODA-08 Orient: Root Cause Analysis

## First Principles Analysis

### What is a table in a two-column layout?

1. **Column-local table**: Fits within one column (width < 50% of page)
2. **Full-width table**: Spans entire page (typically at top/bottom with clear separation)

### Why are side-by-side tables being merged?

The PDF has two tables positioned side-by-side:

- Table 2: Left column (x ≈ 55 to 290)
- Table 3: Right column (x ≈ 310 to 541)

When text elements are extracted, they're grouped into blocks. If both tables share similar Y coordinates, they might be treated as a single wide table.

### Key Insight

The problem isn't in the lattice table detection (returns 0 tables). The problem is in **how text blocks are being converted to tables** by the TextTableReconstructionProcessor or similar logic.

Looking at the pipeline:

1. Blocks are created from text elements
2. In two-column mode, blocks should be constrained to their column
3. But tables bypass this - they're detected page-wide

## Risk Assessment

| Approach                                    | Risk                       | Benefit                               |
| ------------------------------------------- | -------------------------- | ------------------------------------- |
| Filter tables crossing column boundary      | Low - simple check         | High - prevents merged tables         |
| Run column detection before table filtering | Medium - restructure order | High - enables column-aware filtering |
| Split wide tables at column boundary        | High - complex logic       | Medium - may not work for all cases   |

## Signal Analysis

The strongest signal: Tables with `x1 < column_boundary < x2` in two-column layouts should be rejected or split.

## Architecture Gap

Current flow:

```
[PDF] → [Elements] → [Lattice Tables (empty)] → [Filter] → [Column Detection] → [Processors]
                                                    ↑
                                              Missing column awareness
```

Proposed flow:

```
[PDF] → [Elements] → [Column Detection] → [Lattice Tables] → [Column-Aware Filter] → [Processors]
```

## Decision Framework

**Immediate Fix**: Add column-boundary check to table filter in extraction_engine.rs

- Run column detection BEFORE table filtering
- Reject tables that cross the column boundary in two-column layouts
- Exception: Tables at very top or bottom of page (title tables, page-wide figures)

**Signal Value**: HIGH - This single change will prevent merged side-by-side tables.
