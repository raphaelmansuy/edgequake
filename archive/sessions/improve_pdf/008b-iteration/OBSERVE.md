# OBSERVE.md - Iteration 008b

**Directory:** `edgequake/crates/edgequake-pdf/src/processors`

**Timestamp:** 2026-01-02

## Target: MarginFilterProcessor Magic Numbers

### Current State Analysis

#### File: `processor.rs` (lines 177-290)

```rust
pub struct MarginFilterProcessor {
    /// Left margin threshold (blocks with x < this are filtered)
    left_margin: f32,       // MAGIC NUMBER: 50.0
    /// Right margin threshold (blocks with x > page_width - this are filtered)
    right_margin: f32,      // MAGIC NUMBER: 30.0
    /// Top margin threshold
    top_margin: f32,        // MAGIC NUMBER: 40.0
    /// Bottom margin threshold
    bottom_margin: f32,     // MAGIC NUMBER: 40.0
}

impl MarginFilterProcessor {
    /// Create with default margins for academic papers.
    pub fn new() -> Self {
        Self {
            left_margin: 50.0,   // ❌ MAGIC NUMBER - Filter content in first 50pt (line numbers)
            right_margin: 30.0,  // ❌ MAGIC NUMBER - Filter right margin content
            top_margin: 40.0,    // ❌ MAGIC NUMBER - Filter header area
            bottom_margin: 40.0, // ❌ MAGIC NUMBER - Filter footer area
        }
    }
}
```

#### Additional Magic Number in is_margin_content()

```rust
fn is_margin_content(&self, block: &Block, page_width: f32, page_height: f32) -> bool {
    // Check if block is single digit/letter at edge of content (likely line number)
    let text = block.text.trim();
    if text.len() <= 2
        && text.chars().all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
    {
        // If it's positioned far from main content area, filter it
        if bbox.x1 < 60.0 || bbox.x1 > page_width - 60.0 {  // ❌ MAGIC NUMBER: 60.0
            tracing::debug!("Filtering likely line number: '{}'", text);
            return true;
        }
    }
    // ... more checks
}
```

### Magic Numbers Identified

| Magic Number | Purpose               | Context                           | First Principles Violation                             |
| ------------ | --------------------- | --------------------------------- | ------------------------------------------------------ |
| **50.0**     | left_margin           | Filter left margin (line numbers) | Should be % of page width or based on column detection |
| **30.0**     | right_margin          | Filter right margin content       | Should be % of page width                              |
| **40.0**     | top_margin            | Filter header area                | Should be based on body font size × line spacing       |
| **40.0**     | bottom_margin         | Filter footer area                | Should be based on body font size × line spacing       |
| **60.0**     | Line number detection | Edge content threshold            | Should be % of page width or column margin             |

### Root Cause: Fixed Pixel Values

**Problem:** Margins don't adapt to:

1. **Page sizes:** Letter (612pt) vs A4 (595pt) vs presentation slides (1024pt)
2. **Font sizes:** 8pt documents need smaller margins, 14pt need larger
3. **Layout types:** Single-column vs multi-column documents
4. **Document styles:** Academic papers vs technical manuals vs books

### First Principles Analysis

#### Typography Fundamentals

1. **Margin Size = Page Dimension × Margin Factor**

   - Standard margins: 8-12% of page width
   - Academic papers: ~1 inch (72pt) on Letter = 72/612 = 11.8%
   - Books: 10-15% depending on binding

2. **Header/Footer Space = Font Size × Header Lines**

   - Typical header: body_size × 3-4 lines
   - Running headers: 1-2 lines above main text
   - Page numbers in footers: 1-2 lines below main text

3. **Line Number Space = Body Font × Characters**
   - Line numbers typically 2-3 characters wide
   - Character width ≈ 0.5 × font_size
   - Line number margin ≈ 3 × 0.5 × body_size = 1.5 × body_size

### Document Layout Standards

**Standard Margins (percentage of page width/height):**

- **Left/Right:** 8-12% of page width
- **Top/Bottom:** 5-8% of page height (smaller than left/right)
- **Line Numbers:** 5-8% of page width (left edge)

**Examples:**

- Letter (612×792): Left=72pt (11.8%), Right=54pt (8.8%), Top=54pt (6.8%), Bottom=54pt (6.8%)
- A4 (595×842): Left=71pt (11.9%), Right=53pt (8.9%), Top=59pt (7.0%), Bottom=59pt (7.0%)
- Slides (1024×768): Left=102pt (10%), Right=77pt (7.5%), Top=38pt (5%), Bottom=38pt (5%)

### Current Behavior Analysis

**Fixed 50pt Left Margin:**

- Letter (612pt): 50/612 = 8.2% ✓ Reasonable
- A4 (595pt): 50/595 = 8.4% ✓ Reasonable
- Slides (1024pt): 50/1024 = 4.9% ❌ Too small (should be ~100pt)
- Narrow column (300pt): 50/300 = 16.7% ❌ Too large (filters content)

**Fixed 60pt Line Number Detection:**

- Letter: 60/612 = 9.8% ✓ Works
- Slides: 60/1024 = 5.9% ❌ Misses line numbers in wide layouts

### Expected Improvements with Adaptive Margins

1. **Wide Pages (slides, posters):** Larger margins scale proportionally
2. **Narrow Columns:** Smaller margins don't filter content
3. **Large Fonts:** Header/footer space scales with font size
4. **Multi-column:** Column margins detected separately

### Implementation Complexity

**Low - Similar to Loop 007:**

- Use existing DocumentStats (page_width, page_height, body_font_size)
- Calculate margins as percentages
- No new infrastructure needed

### Test Cases to Verify

1. **Letter Size PDF:** Margins should filter headers/footers/line numbers
2. **A4 Size PDF:** Similar behavior to Letter
3. **Presentation Slides:** Larger absolute margins (but same % of page)
4. **Multi-column Academic:** Don't filter column boundaries
5. **Large Font Document:** Header/footer space scales with font

### Acceptance Criteria

✅ Zero magic numbers in MarginFilterProcessor
✅ Margins calculated as % of page dimensions
✅ Header/footer space based on body font size
✅ All 113 tests passing
✅ No false positives (content filtered incorrectly)
✅ No false negatives (margins not filtered)

### Risk Assessment

**Low Risk:**

- Non-critical processor (filters margin content)
- If wrong, content might be included/excluded
- Easy to adjust percentages if needed
- Tests will catch regressions

**Mitigation:**

- Conservative percentages (start with current effective %)
- Log what's being filtered for debugging
- Add tests for edge cases
