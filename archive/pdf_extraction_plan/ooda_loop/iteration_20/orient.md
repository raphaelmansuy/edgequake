# OODA-20 Orient: Understanding Block Merge Pipeline

## Mental Model

The block merge pipeline has three stages:

1. **BBox Calculation** (block_builder.rs) - Compute block boundaries
2. **Column Detection** (geometric.rs) - Identify page layout columns
3. **Block Merge** (layout_processing.rs) - Combine adjacent blocks in same column

Each stage depends on correct output from the previous stage.

## Root Causes

### Root Cause 1: BBox Width = 0

**Location**: `block_builder.rs::calculate_line_bbox()`

The function computed:

```rust
max_x = elements.iter().map(|e| e.x).max()  // WRONG: just x position
```

Should be:

```rust
max_x = elements.iter().map(|e| e.x + estimated_width).max()  // text end position
```

**Impact**: Zero-width bboxes have undefined center points, causing column assignment chaos.

### Root Cause 2: No Minimum Column Width

**Location**: `geometric.rs::detect_columns()`

DBSCAN clustering found columns from any X-position clusters, including:

- Indentation patterns (x=322 vs x=300)
- Bullet point alignments

Without a minimum width filter, these became spurious columns.

**Impact**: Logically-contiguous text got split across "columns".

### Root Cause 3: Center-Based Column Assignment

**Location**: `layout_processing.rs::get_block_column()`

Used block center point:

```rust
let center = block.bbox.center();
columns.iter().position(|col| col.contains_point(center))
```

**Problem**: Wide blocks (spanning indentation) get different column than narrow continuation.

**Better**: Use left edge (x1) - where reading starts.

## Design Constraints

1. **Must remain generic** - No PDF-specific heuristics
2. **Must handle real layouts** - Academic papers have complex indentation
3. **Must be predictable** - Same content should always merge the same way

## Implications

| Root Cause        | Fix Location         | Risk Level                |
| ----------------- | -------------------- | ------------------------- |
| Zero-width BBox   | block_builder.rs     | LOW - localized           |
| No min width      | geometric.rs         | MEDIUM - affects all docs |
| Center assignment | layout_processing.rs | MEDIUM - changes behavior |

All three issues must be fixed together for proper block merging.
