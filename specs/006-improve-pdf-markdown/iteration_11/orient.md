# OODA-11: Orient - Document Constants in markdown.rs

## Analysis

### 72.0pt (line 248, list indentation baseline)

**Purpose**: Standard PDF left margin
- 72pt = 1 inch at 72 DPI (standard PDF resolution)
- Most PDF generators use 1-inch margins by default
- Subtracting 72.0 gives indent relative to margin

**Rationale**: Lists start at the margin, so base indent = 72pt.

### 20.0pt (line 248, list indent step)

**Purpose**: Indentation per list level
- 20pt ≈ 0.28 inches per level
- Standard typographic indent is 0.25-0.5 inches
- Matches common word processor settings

**Rationale**: Each list level adds ~0.28" indent.

### 10.0pt (line 601, table row threshold)

**Purpose**: Y-tolerance for same-row detection
- 10pt = typical line height difference within a cell
- Matches other tolerances in the codebase (block_gap, etc.)

**Rationale**: Cells on same row should have Y within 10pt.

## Prioritization

1. 72.0 and 20.0 - affect list rendering
2. 10.0 - affects table row grouping
