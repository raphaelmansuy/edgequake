# OODA Loop 2 - DECIDE

## Selected Fix: Starting-Point Containment

### Change

**File:** `crates/edgequake-pdf/src/backend/lattice.rs`  
**Function:** `extract_text_in_rect` (line ~690)

**Current (center-point):**

```rust
let char_width = if elem.font_size > 0.0 {
    elem.font_size * 0.5
} else {
    5.0
};
let text_width = elem.text.len() as f32 * char_width;
let cx = elem.x + text_width / 2.0;
let cy = elem.y;

let inside = cx >= min_x - tol && cx <= max_x + tol
    && cy >= min_y - tol && cy <= max_y + tol;
```

**New (starting-point):**

```rust
// Use starting point (x, y) for containment instead of center point.
// This avoids errors from character width estimation (monospace assumption).
let inside = elem.x >= min_x - tol && elem.x <= max_x + tol
    && elem.y >= min_y - tol && elem.y <= max_y + tol;
```

### First Principles Justification

**Truth:** We don't have accurate character widths from PDF extraction.  
**Consequence:** Center-point calculation is unreliable.  
**Solution:** Use position we DO have accurately: starting point (x, y).

**Why this works:**

1. Table cells are defined by column boundaries (vertical lines)
2. Text starts at its leftmost point (elem.x)
3. If text STARTS in column A, it belongs to column A (with rare exceptions)
4. Exceptions (long text spanning columns) are handled by crossing_ratio check

**Trade-offs:**

- ✅ Robust to font variations (no width calculation)
- ✅ Simple (remove 10 lines of code)
- ⚠️ May misassign very long text that spans multiple columns
- ⚠️ Right-aligned text might be less accurate (but rare in tables)

### Predicted Impact

**Table Accuracy:** 2.4% → 18-25%  
**Reasoning:**

- 80%+ of table text is single words/short phrases
- Starting-point correctly assigns these
- Remaining errors are multi-column spans (already filtered by crossing_ratio)

### Implementation

Remove character width estimation entirely. Simplify to direct position check.
