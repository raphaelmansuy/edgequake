# OODA Loop 2 - ORIENT

## Root Cause: Character Width Estimation

### Current State

**TextElement structure:**

```rust
pub struct TextElement {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub font_size: f32,
    // ...no width field
}
```

**Problem:** PDF text operators (`Tj`, `TJ`) provide:

- Position: `text_matrix[4]`, `text_matrix[5]`
- Font size
- **NOT** actual rendered width

### First Principles Analysis

**PDF Text Rendering Math:**

1. Character width (in glyph space) = font's character width metric
2. Rendered width = (glyph_width / 1000) _ font_size _ horizontal_scale
3. This varies per character ("W" ≠ "i")

**Current Hack:**

```rust
let char_width = elem.font_size * 0.5;  // Assumes ALL chars are half font size wide
let text_width = elem.text.len() * char_width;
```

**Why this fails:**

- Assumes monospace (all characters same width)
- Uses fixed ratio (0.5) which is empirically chosen, not font-aware
- "CHAPTER" (7 chars) estimated as `7 * font_size * 0.5`
- But real "CHAPTER" in Times-Bold might be `9.2 * font_size` width units

### Options

#### Option 1: Extract actual width from PDF (CORRECT but hard)

- Read font widths table from PDF font dictionary
- Calculate actual glyph widths
- **Pros:** Accurate
- **Cons:** Complex font parsing, encoding issues

#### Option 2: Better estimation heuristic (PRAGMATIC)

- Use variable character width estimates (W=0.8, i=0.3, etc.)
- Average English text: ~0.55 \* font_size per char (not 0.5)
- **Pros:** Fast, 80% accurate for most texts
- **Cons:** Still an approximation

#### Option 3: Use loose containment instead of center-point (ROBUST)

- Don't calculate center at all
- Use: "Any text that STARTS in this cell belongs to it"
- **Pros:** Robust to width errors
- **Cons:** Multi-column words might still misassign

### Decision: Option 3 (Loose Containment)

**Rationale:**

- Simplest fix
- No font parsing required
- Handles majority of real-world table cases
- Can combine with improved width heuristic later

**Implementation:**
Instead of:

```rust
let cx = elem.x + text_width / 2.0;
// Check if CENTER is in cell
```

Use:

```rust
// Check if STARTING POINT is in cell with tolerance
let inside = elem.x >= min_x - tol && elem.x <= max_x + tol && ...
```

### Expected Impact

**Current:** 2.4% table accuracy  
**After:** 15-25% table accuracy (estimated)

**Why:** Most table cells contain single words or short phrases. Starting-point containment correctly assigns ~80% of table text.
