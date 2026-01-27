# ORIENT.md - Iteration 008b

**Directory:** `edgequake/crates/edgequake-pdf/src/processors`

**Timestamp:** 2026-01-02

## Strategy: Percentage-Based Adaptive Margins

### Root Cause Solution

**Margins are relative to page dimensions, not absolute pixels.**

### First Principles Approach

#### 1. Margin Calculation Formula

```rust
// Standard margin percentages (based on typography standards)
left_margin   = page_width  × 0.08  // 8% of page width
right_margin  = page_width  × 0.05  // 5% of page width (smaller - less margin content)
top_margin    = page_height × 0.05  // 5% of page height
bottom_margin = page_height × 0.05  // 5% of page height

// Line number detection threshold
line_number_edge = page_width × 0.10  // 10% from edges
```

#### 2. Why These Percentages?

**Left Margin (8%):**

- Letter (612pt): 612 × 0.08 = 48.96pt ≈ 49pt (current: 50pt) ✓
- A4 (595pt): 595 × 0.08 = 47.6pt ≈ 48pt ✓
- Slides (1024pt): 1024 × 0.08 = 81.92pt ≈ 82pt (current: 50pt ❌)

**Right Margin (5%):**

- Letter: 612 × 0.05 = 30.6pt ≈ 31pt (current: 30pt) ✓
- A4: 595 × 0.05 = 29.75pt ≈ 30pt ✓
- Slides: 1024 × 0.05 = 51.2pt ≈ 51pt (current: 30pt ❌)

**Top/Bottom Margins (5%):**

- Letter (792pt height): 792 × 0.05 = 39.6pt ≈ 40pt (current: 40pt) ✓
- A4 (842pt height): 842 × 0.05 = 42.1pt ≈ 42pt ✓
- Slides (768pt height): 768 × 0.05 = 38.4pt ≈ 38pt (current: 40pt ✓)

**Line Number Detection (10%):**

- Letter: 612 × 0.10 = 61.2pt ≈ 61pt (current: 60pt) ✓
- Slides: 1024 × 0.10 = 102.4pt ≈ 102pt (current: 60pt ❌)

**Result:** Percentages match current behavior for standard pages but scale correctly for non-standard sizes!

### Proposed Architecture

#### Option 1: Calculate in new() Using DocumentStats (Preferred)

```rust
pub struct MarginFilterProcessor {
    // No configuration - margins calculated from page dimensions!
}

impl MarginFilterProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Processor for MarginFilterProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        let stats = DocumentStats::from_document(&document);

        // Calculate adaptive margins once
        let left_margin = stats.page_width * 0.08;
        let right_margin = stats.page_width * 0.05;
        let top_margin = stats.page_height * 0.05;
        let bottom_margin = stats.page_height * 0.05;
        let line_number_edge = stats.page_width * 0.10;

        for page in &mut document.pages {
            page.blocks.retain(|block| {
                !self.is_margin_content(
                    block,
                    page.width,
                    page.height,
                    left_margin,
                    right_margin,
                    top_margin,
                    bottom_margin,
                    line_number_edge,
                )
            });
        }

        Ok(document)
    }
}
```

#### Option 2: Calculate Per-Page (Handles Mixed Page Sizes)

```rust
fn process(&self, mut document: Document) -> Result<Document> {
    for page in &mut document.pages {
        // Calculate margins for THIS page's dimensions
        let left_margin = page.width * 0.08;
        let right_margin = page.width * 0.05;
        let top_margin = page.height * 0.05;
        let bottom_margin = page.height * 0.05;
        let line_number_edge = page.width * 0.10;

        page.blocks.retain(|block| {
            !self.is_margin_content(
                block,
                page.width,
                page.height,
                left_margin,
                right_margin,
                top_margin,
                bottom_margin,
                line_number_edge,
            )
        });
    }

    Ok(document)
}
```

**Recommendation:** Option 2 (per-page) is more robust for documents with mixed page sizes.

### Updated is_margin_content() Signature

```rust
fn is_margin_content(
    &self,
    block: &Block,
    page_width: f32,
    page_height: f32,
    left_margin: f32,      // Adaptive - passed in
    right_margin: f32,     // Adaptive - passed in
    top_margin: f32,       // Adaptive - passed in
    bottom_margin: f32,    // Adaptive - passed in
    line_number_edge: f32, // Adaptive - passed in
) -> bool {
    let bbox = &block.bbox;

    // Left margin check
    if bbox.x2 < left_margin {
        if block.text.trim().len() <= 3 {
            tracing::debug!("Filtering left margin content: '{}'", block.text.trim());
            return true;
        }
    }

    // Right margin check
    if bbox.x1 > page_width - right_margin {
        if block.text.trim().len() <= 3 {
            tracing::debug!("Filtering right margin content: '{}'", block.text.trim());
            return true;
        }
    }

    // Line number detection (using adaptive threshold)
    let text = block.text.trim();
    if text.len() <= 2
        && text.chars().all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
    {
        if bbox.x1 < line_number_edge || bbox.x1 > page_width - line_number_edge {
            tracing::debug!("Filtering likely line number: '{}'", text);
            return true;
        }
    }

    // Top margin (headers) - unchanged logic
    if bbox.y2 < top_margin && block.text.len() < 100 {
        // Skip - we want to keep page headers
    }

    // Bottom margin (footers - page numbers)
    if bbox.y1 > page_height - bottom_margin && block.text.len() < 100 {
        let trimmed = block.text.trim();
        if trimmed.parse::<i32>().is_ok() {
            tracing::debug!("Filtering footer page number: '{}'", trimmed);
            return true;
        }
    }

    false
}
```

### Backward Compatibility

Keep `with_margins()` constructor for tests:

```rust
#[cfg(test)]
pub fn with_margins(_left: f32, _right: f32, _top: f32, _bottom: f32) -> Self {
    // Parameters ignored - now calculated adaptively
    Self {}
}
```

### Comparison: Before vs After

#### Before (Magic Numbers)

```rust
left_margin: 50.0      // Fixed
right_margin: 30.0     // Fixed
top_margin: 40.0       // Fixed
bottom_margin: 40.0    // Fixed
line_number_edge: 60.0 // Fixed
```

**Problems:**

- Doesn't scale with page size
- Presentation slides (1024pt) use same margins as Letter (612pt)
- Narrow columns might have content filtered

#### After (Adaptive Percentages)

```rust
left_margin:      page_width  × 0.08  // Scales proportionally
right_margin:     page_width  × 0.05  // Scales proportionally
top_margin:       page_height × 0.05  // Scales proportionally
bottom_margin:    page_height × 0.05  // Scales proportionally
line_number_edge: page_width  × 0.10  // Scales proportionally
```

**Advantages:**

- Works on any page size
- Presentation slides get appropriately larger margins
- Narrow pages get smaller margins (less aggressive filtering)
- No magic numbers!

### Alternative: Font-Based Margins (More Complex)

Could also base margins on body font size:

```rust
left_margin = stats.body_font_size × 4.0    // ~4 characters wide
top_margin  = stats.body_font_size × 3.0    // ~3 lines tall
```

**Pros:**

- Even more adaptive (scales with font size)
- Theoretical foundation (characters × lines)

**Cons:**

- More complex
- Doesn't handle page size variation as well
- Typography standards use % of page, not font multiples

**Decision:** Use percentage-based approach (simpler, standard practice).

### Expected Impact

1. **Presentation Slides:** Margins scale from 50pt→82pt (left), 60pt→102pt (line numbers)
2. **Narrow Columns:** Margins scale down, less content filtered incorrectly
3. **Large Pages:** Margins scale up appropriately
4. **Standard PDFs:** Nearly identical behavior (50pt→49pt, 60pt→61pt)

### Risk Mitigation

1. **Testing:** Run all 113 tests to verify no regressions
2. **Logging:** Keep debug logs showing what's filtered
3. **Conservative Percentages:** 8% and 5% are standard, proven values
4. **Per-Page Calculation:** Handles mixed-size documents correctly

### Implementation Steps

1. Remove fields from MarginFilterProcessor struct
2. Update new() to return empty struct
3. Update process() to calculate per-page margins
4. Update is_margin_content() signature with 5 margin parameters
5. Update all is_margin_content() calls to use adaptive values
6. Keep with_margins() for backward compatibility (parameters ignored)
7. Run tests and verify

### Performance Impact

- **Overhead:** 5 multiplications per page (negligible)
- **Frequency:** Once per page during processing
- **Memory:** Zero (margins calculated on-demand)
- **Result:** No measurable performance impact
