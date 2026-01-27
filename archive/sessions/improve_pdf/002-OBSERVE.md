# OODA Loop 2 - OBSERVE

## Focus: Cell Text Extraction Quality

### Current Problem

- **Table Accuracy:** 2.4% (validator metric)
- **Tables Detected:** ~55 across 5 PDFs (good!)
- **Issue:** Detected tables have poor cell content accuracy

### Code Analysis

**File:** `crates/edgequake-pdf/src/backend/lattice.rs`  
**Function:** `extract_text_in_rect` (line 675)

### Current Implementation

```rust
let char_width = if elem.font_size > 0.0 {
    elem.font_size * 0.5
} else {
    5.0
};
let text_width = elem.text.len() as f32 * char_width;
let cx = elem.x + text_width / 2.0;
```

### First Principles Problem

**Assumption:** All characters have the same width (monospace)  
**Reality:** PDFs use proportional fonts (e.g., "W" is wider than "i")

**Impact:**

- `"WWW".len() = 3` → estimated width = `3 * (font_size * 0.5)`
- `"iii".len() = 3` → estimated width = `3 * (font_size * 0.5)` (SAME!)
- But "WWW" is ~3x wider than "iii" in real fonts

**Consequence:**

- Wide words ("CHAPTER", "SECTION") have their center miscalculated
- They get assigned to the wrong cell (first cell instead of actual cell)
- Table cells end up with missing or incorrect text

### Evidence

From validator showing 2.4% table accuracy:

- Tables are FOUND (detection works)
- Content is WRONG (cell assignment fails)

### Root Cause

**Mathematical error:** Using character count as proxy for visual width ignores:

1. Proportional spacing (variable character widths)
2. Kerning (pair-specific adjustments)
3. Font metrics (actual glyph widths)

**First principles truth:** PDF TextElement should provide actual text bounding box, not just (x, y) position.
