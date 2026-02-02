# OODA-08 Act: Column-Crossing Table Filter Implementation

## Changes Implemented

### 1. Early Column Detection (extraction_engine.rs:438-442)

```rust
// OODA-08: Detect column layout BEFORE table filtering
// WHY: In two-column layouts, tables that span both columns are likely
// false positives (side-by-side tables merged). We need to filter these out.
let column_boundary = self.detect_columns(&elements, page_width);
tracing::info!("Page {} OODA08 column boundary: {:?}", page_num, column_boundary);
```

### 2. Column-Crossing Filter (extraction_engine.rs:510-537)

```rust
// OODA-08: Filter out tables that cross the column boundary
if let Some(boundary) = column_boundary {
    // WHY: In two-column layouts, a table with bbox spanning both columns
    // is likely a false positive from the lattice engine detecting
    // graphical rules that span the gutter. We filter these out UNLESS
    // they're in the header/footer area (which may legitimately span both).
    let crosses_boundary = table.bbox.x1 < boundary - 10.0
        && table.bbox.x2 > boundary + 10.0;

    if crosses_boundary {
        let is_top_area = table.bbox.y1 < page_height * 0.20;
        let is_bottom_area = table.bbox.y1 > page_height * 0.85;

        if !is_top_area && !is_bottom_area {
            debug!(
                "Filtered out table: crosses column boundary {:.1} (bbox x1={:.1}, x2={:.1})",
                boundary, table.bbox.x1, table.bbox.x2
            );
            return false;
        }
    }
}
```

## Test Results

### Build Status

✅ Build successful with warnings

### Column Detection Results

| Page Range | Column Boundary   | Result                           |
| ---------- | ----------------- | -------------------------------- |
| Pages 1-11 | Some(295.0-310.0) | ✅ Correctly detected two-column |
| Pages 12+  | None              | ❌ Detected as single-column     |

### Root Cause Analysis

The column detection returns `None` on pages 12+ because:

1. **Pages with full-width content**: Pages 18, 19, 21, 22, 29, 34 contain
   appendix tables that genuinely span both columns
2. **Lattice false positives**: The Lattice engine detects graphical rules
   (table borders, separators) that span the full page width
3. **Text element distribution**: On these pages, the `detect_columns` function
   sees `left_starts=43, right_starts=0-1` because:
   - Text elements inside detected "tables" may have adjusted coordinates
   - Or the content is genuinely single-column (appendix/references)

### Tables Still Passing Filters

```
Page 18: bbox { x1: 55.44, x2: 541.44 } - column_boundary: None
Page 19: bbox { x1: 55.44, x2: 541.44 } - column_boundary: None
Page 21: bbox { x1: 55.44, x2: 541.44 } - column_boundary: None
Page 34: bbox { x1: 55.44, x2: 541.44 } - column_boundary: None
```

These tables pass because `column_boundary = None`, so the filter doesn't apply.

## Problem Diagnosis

The original hypothesis was partially correct but incomplete:

1. ✅ **Correct**: Tables spanning both columns in two-column layouts should be filtered
2. ❌ **Incomplete**: Column detection fails on pages where:
   - The content is genuinely single-column (appendix tables)
   - The Lattice engine has already grouped all text into a "table"

### Actual Root Cause

The Lattice engine is detecting **graphical lines** in the PDF that form
rectangles spanning the full page width. These lines might be:

- Table borders in the original PDF
- Column separator lines
- Page margin decorations

When the engine creates a "table" from these lines, it groups ALL text
elements between those lines into the table content, losing the two-column
structure entirely.

## Next Steps (OODA-09)

The column-crossing filter is correct but insufficient. We need to:

1. **Investigate Lattice engine behavior**: Why is it detecting full-page tables?
2. **Improve within-table column detection**: Detect if table content has
   two-column structure INSIDE the table bbox
3. **Alternative approach**: Skip tables where the text density doesn't match
   typical table patterns (e.g., many short cells vs long paragraphs)

## Files Modified

- `edgequake/crates/edgequake-pdf/src/backend/extraction_engine.rs`
  - Lines 438-442: Early column detection
  - Lines 510-537: Column-crossing filter

## Verification Commands

```bash
# Build
cargo build --release

# Test with logging
RUST_LOG=edgequake_pdf=info cargo run --release --bin edgequake-pdf -- \
  /path/to/agentfail_2601.22984v1.pdf -o /tmp/test.md 2>&1 | \
  grep -E "OODA08|Table passed"
```
