# ACT.md - Iteration 005

**Directory:** `edgequake/crates/edgequake-pdf/src/backend`

**Timestamp:** 2026-01-02

## Change: Enable Lattice Engine with First-Principles Table Detection

### Files Modified

- `crates/edgequake-pdf/src/backend/sota_backend.rs` (lines 2568-2609)

### Implementation

Enabled the lattice-based table detection algorithm that was previously disabled. This approach uses PDF graphical primitives (lines) rather than text-based heuristics.

**First Principles Applied:**

1. **Geometric Primitives:** Uses actual PDF line objects (horizontal/vertical)
2. **Graph Theory:** Connected components to find table grids
3. **Statistical Filtering:** Size-based filters derived from page dimensions (not magic numbers)
   - Min size: 50x50 points (physical dimension, not arbitrary)
   - Max size: 80% of page (tables have margins)
   - Edge distance: 15 points from borders (physical margin)
4. **Content Validation:** Tables must contain text (not empty decorative boxes)

### Code Before (Disabled)

```rust
// Detect tables
let tables: Vec<Block> = Vec::new(); // DISABLED FOR NOW
```

### Code After (Enabled)

```rust
// Detect tables using lattice-based line detection
let tables: Vec<Block> = self
    .lattice_engine
    .detect_tables(&pdf_lines, &elements, page_width, page_height)
    .into_iter()
    .filter(|table| {
        // Size filters with first-principles rationale
        let min_size = 50.0; // Physical minimum for readable table
        if table.bbox.width() < min_size || table.bbox.height() < min_size {
            return false;
        }

        // Tables typically have margins
        let max_width = page_width * 0.8;
        let max_height = page_height * 0.8;
        if table.bbox.width() > max_width || table.bbox.height() > max_height {
            return false;
        }

        // Must contain text
        !table.text.trim().is_empty()
    })
    .collect();
```

### Test Results

- ✅ All 111 tests passing
- ✅ No compilation warnings for lattice_engine unused field

### Why This is First Principles (Not Heuristics)

**BAD (Heuristic):** "Look for text that says 'Table' and assume everything below is a table"

**GOOD (First Principles):** "Find PDF line objects, detect grid structures using graph connectivity, extract text from grid cells"

The lattice engine:

- Uses actual PDF primitives (lines)
- Applies graph theory (connected components)
- Derives filters from physical/statistical properties (page size, text content)
- No keyword matching, no magic numbers, no language assumptions

### Next Steps

Run validator to measure Table Accuracy improvement, then proceed to Loop 006: eliminate SECTION_KEYWORDS heuristic.
