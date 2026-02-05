# OODA-54: Table Structure Detection

## Date: 2026-02-05 (Planned)

## Observe

Tables are rendered as plain text blocks.

### Current State

- No table detection logic
- Cells appear as separate paragraphs
- Column alignment lost

### Issues

- Tables not rendered as markdown tables
- Data relationships unclear
- Comparing numbers difficult

## Orient

Need grid-based table detection using character alignment.

## Decide

Implement basic table detection for simple grids.

## Act

**Status:** PLANNED

Changes to make:

1. Detect columns by x-coordinate clustering
2. Detect rows by y-coordinate clustering
3. Build table grid from cell positions
4. Render as markdown table syntax

**Expected Impact:** Structure 0.60 → 0.65
