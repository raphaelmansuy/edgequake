# OODA Loop 018 - DECIDE

**Timestamp:** 2026-01-03 15:25:00

**Directory:** crates/edgequake-pdf/src/processors

## Proposed Change

### Patch: Add Font-Size Based Heading Detection

**File:** `crates/edgequake-pdf/src/processors/processor.rs`

**Target:** `SectionPatternProcessor::process()`

### First Principles Reasoning

**Truth 1:** Headings are geometrically distinct from body text

- Larger font size (typically 1.3x - 2x body text)
- Often bold weight
- Usually short (< 100 chars)
- Isolated on their own line

**Truth 2:** Current detection only uses:

- Section number patterns (e.g., "3.2 Title")
- Special keywords ("Abstract", "Introduction", etc.)
- This misses headings without numbers or keywords

**Truth 3:** Font size is available in `block.spans[].style.size`

- We can calculate median font size for document (body text)
- Detect blocks with significantly larger fonts
- This is geometry-based, not heuristic-based

### Implementation Plan

```rust
// Add to SectionPatternProcessor

fn detect_body_font_size(&self, document: &Document) -> f32 {
    // Collect all font sizes from non-heading blocks
    let mut sizes: Vec<f32> = Vec::new();
    for page in &document.pages {
        for block in &page.blocks {
            if block.block_type == BlockType::Text || block.block_type == BlockType::Paragraph {
                for span in &block.spans {
                    if let Some(size) = span.style.size {
                        sizes.push(size);
                    }
                }
            }
        }
    }

    // Return median (50th percentile) - more robust than mean
    sizes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if sizes.is_empty() {
        12.0 // default
    } else {
        sizes[sizes.len() / 2]
    }
}

fn is_heading_by_font_size(&self, block: &Block, body_size: f32) -> (bool, u8) {
    // Check if block has consistently larger font
    let mut large_count = 0;
    let mut total_count = 0;
    let mut max_size = 0.0f32;

    for span in &block.spans {
        if let Some(size) = span.style.size {
            total_count += 1;
            if size > body_size * 1.2 {
                large_count += 1;
                max_size = max_size.max(size);
            }
        }
    }

    // Require 80% of spans to be large font
    if total_count > 0 && (large_count as f32 / total_count as f32) > 0.8 {
        // Calculate level based on size ratio
        let ratio = max_size / body_size;
        let level = if ratio >= 1.8 {
            2 // H2 for very large
        } else if ratio >= 1.5 {
            3 // H3 for large
        } else {
            4 // H4 for moderately large
        };

        // Additional validation: short text, not ending with period
        let text = block.text.trim();
        if text.len() < 100 && !text.ends_with('.') {
            return (true, level);
        }
    }

    (false, 0)
}
```

Then modify the main loop:

```rust
// In process() method, before the main loop:
let body_font_size = self.detect_body_font_size(&document);

// In the block loop, after existing checks:
else {
    // Try font-size based detection
    let (is_heading, level) = self.is_heading_by_font_size(block, body_font_size);
    if is_heading {
        block.block_type = BlockType::SectionHeader;
        block.level = Some(level);
        tracing::debug!("Detected heading by font size: '{}' -> level {}", text, level);
    }
}
```

### Expected Impact

**Quantitative:**

- Add 5-15% to style accuracy
- Detect headings currently missed by pattern matching
- Should help documents with low scores (ccn_2512.21804v1: 7% → 15-20%)

**Qualitative:**

- More robust heading detection
- Works across different document styles
- Doesn't depend on section numbering conventions

### Acceptance Checklist

- [ ] Code compiles without warnings
- [ ] All existing tests pass
- [ ] New unit test for `detect_body_font_size()`
- [ ] New unit test for `is_heading_by_font_size()`
- [ ] Validator shows improved style accuracy
- [ ] No regression in table accuracy
- [ ] Composite score increases by at least 2 points

### Risk Assessment

**Low Risk:**

- Only adds new detection path (fallback)
- Doesn't modify existing pattern-based detection
- Easy to rollback if issues found

**Potential Issues:**

- False positives: Large text that isn't a heading
  - Mitigation: Length check (< 100 chars), period check
- Performance: Extra pass through document
  - Mitigation: Single pass, O(n) complexity

## Next: ACT

Implement the patch, add tests, run validation.
