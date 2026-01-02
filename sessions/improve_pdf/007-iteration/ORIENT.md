# ORIENT.md - Iteration 007

**Directory:** `edgequake/crates/edgequake-pdf/src/processors`

**Timestamp:** 2026-01-02

## Root Cause: Hardcoded Spatial Thresholds

### Problem Analysis

The BlockMergeProcessor uses fixed pixel values that don't adapt to:

1. Document font sizes (9pt vs 14pt vs 24pt)
2. Page dimensions (Letter vs A4 vs presentation slides)
3. Layout styles (single column vs multi-column vs tables)

### First Principles Solution

**Spatial relationships are relative to font metrics, not absolute pixels.**

#### Fundamental Relationships (Typography)

1. **Line Spacing = Body Font Size × Leading Factor**

   - Typical leading: 1.2-1.5x font size
   - Academic papers: ~1.4x (10pt body → 14pt spacing)
   - Single-spaced: 1.2x
   - Double-spaced: 2.0x

2. **Paragraph Spacing = Line Spacing × Paragraph Factor**

   - Typical: 1.5-2.0x line spacing
   - Used to separate paragraphs

3. **Column Gap = Page Width × Column Factor**
   - Typical: 2-5% of page width
   - Or absolute: 20-50pt depending on page size

### Proposed Architecture

#### Stage 1: Document Statistics Collection

```rust
pub struct DocumentStats {
    pub body_font_size: f32,        // Median font size (most common)
    pub typical_line_spacing: f32,   // Median vertical gap between lines
    pub column_alignment_tolerance: f32, // Adaptive X-alignment tolerance
    pub page_width: f32,
    pub page_height: f32,
}

impl DocumentStats {
    pub fn from_document(doc: &Document) -> Self {
        // 1. Calculate body font size (median across all blocks)
        let mut sizes: Vec<f32> = doc.pages.iter()
            .flat_map(|p| &p.blocks)
            .filter_map(|b| b.spans.first())
            .filter_map(|s| s.style.size)
            .collect();
        sizes.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let body_font_size = percentile(&sizes, 0.5).unwrap_or(10.0);

        // 2. Calculate typical line spacing
        let mut gaps: Vec<f32> = doc.pages.iter()
            .flat_map(|p| consecutive_block_pairs(&p.blocks))
            .map(|(a, b)| (a.bbox.y1 - b.bbox.y2).abs())
            .filter(|&gap| gap < body_font_size * 3.0) // Filter outliers
            .collect();
        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let typical_line_spacing = percentile(&gaps, 0.5)
            .unwrap_or(body_font_size * 1.4);

        // 3. Calculate X-alignment tolerance from actual distributions
        let x_coords: Vec<f32> = doc.pages.iter()
            .flat_map(|p| &p.blocks)
            .map(|b| b.bbox.x1)
            .collect();
        let column_alignment_tolerance = calculate_alignment_tolerance(&x_coords);

        // 4. Get page dimensions (use most common page size)
        let (page_width, page_height) = most_common_page_size(doc);

        DocumentStats {
            body_font_size,
            typical_line_spacing,
            column_alignment_tolerance,
            page_width,
            page_height,
        }
    }
}
```

#### Stage 2: Adaptive BlockMergeProcessor

```rust
pub struct BlockMergeProcessor {
    // Remove fixed thresholds!
    // max_vertical_gap: f32,
    // max_margin_diff: f32,
}

impl BlockMergeProcessor {
    pub fn new() -> Self {
        Self {}  // No configuration needed!
    }

    fn should_merge(&self, a: &Block, b: &Block, stats: &DocumentStats) -> bool {
        // Derive thresholds from document statistics

        // 1. Vertical gap: Allow up to 2.5x typical line spacing
        //    (covers single-spaced to near-double-spaced)
        let max_vertical_gap = stats.typical_line_spacing * 2.5;

        // 2. For headers, allow more vertical space (multi-line headers)
        let vertical_threshold = if a.block_type == BlockType::SectionHeader {
            max_vertical_gap * 1.5
        } else {
            max_vertical_gap
        };

        let vertical_gap = (a.bbox.y1 - b.bbox.y2).abs();
        if vertical_gap > vertical_threshold {
            return false;
        }

        // 3. Horizontal alignment: Use adaptive tolerance
        let margin_diff = (a.bbox.x1 - b.bbox.x1).abs();
        if margin_diff > stats.column_alignment_tolerance {
            return false;
        }

        // 4. Column separation: Blocks in different columns shouldn't merge
        //    Use clustering results from LayoutAnalyzer
        let horizontal_zone_threshold = stats.page_width * 0.15; // 15% of page width
        if margin_diff > horizontal_zone_threshold {
            return false;
        }

        true
    }
}

impl Processor for BlockMergeProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        // Calculate stats once for entire document
        let stats = DocumentStats::from_document(&document);

        for page in &mut document.pages {
            let blocks = std::mem::take(&mut page.blocks);
            page.blocks = self.merge_page_blocks(blocks, &stats);
            page.update_stats();
        }

        document.update_stats();
        Ok(document)
    }
}
```

#### Stage 3: Helper Functions

```rust
fn percentile(sorted: &[f32], p: f32) -> Option<f32> {
    if sorted.is_empty() {
        return None;
    }
    let idx = ((sorted.len() - 1) as f32 * p) as usize;
    Some(sorted[idx])
}

fn consecutive_block_pairs(blocks: &[Block]) -> Vec<(&Block, &Block)> {
    blocks.windows(2)
        .map(|w| (&w[0], &w[1]))
        .collect()
}

fn calculate_alignment_tolerance(x_coords: &[f32]) -> f32 {
    // Use DBSCAN-style calculation: 10th percentile of nearest-neighbor distances
    let mut sorted = x_coords.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut nearest_neighbor_dists: Vec<f32> = Vec::new();
    for i in 1..sorted.len() {
        let dist = sorted[i] - sorted[i-1];
        if dist > 0.1 { // Filter zero distances
            nearest_neighbor_dists.push(dist);
        }
    }

    nearest_neighbor_dists.sort_by(|a, b| a.partial_cmp(b).unwrap());
    percentile(&nearest_neighbor_dists, 0.1).unwrap_or(20.0)
}

fn most_common_page_size(doc: &Document) -> (f32, f32) {
    use std::collections::HashMap;
    let mut size_counts: HashMap<(i32, i32), usize> = HashMap::new();

    for page in &doc.pages {
        let key = (page.width as i32, page.height as i32);
        *size_counts.entry(key).or_insert(0) += 1;
    }

    let (w, h) = size_counts.iter()
        .max_by_key(|(_, count)| *count)
        .map(|((w, h), _)| (*w as f32, *h as f32))
        .unwrap_or((612.0, 792.0)); // Default Letter size

    (w, h)
}
```

### Comparison: Before vs After

#### Before (Magic Numbers)

```rust
// Hardcoded - doesn't adapt
max_vertical_gap: 50.0
max_margin_diff: 20.0
horizontal_zone_threshold: 100.0
```

**Problems:**

- Works for 10-12pt documents
- Fails on 8pt (merges too aggressively)
- Fails on 18pt (doesn't merge enough)
- Fails on narrow columns (<100pt separation)

#### After (Statistical)

```rust
// Derived from document properties
max_vertical_gap: stats.typical_line_spacing * 2.5
max_margin_diff: stats.column_alignment_tolerance
horizontal_zone_threshold: stats.page_width * 0.15
```

**Advantages:**

- Adapts to any font size
- Adapts to any page layout
- Based on actual document statistics
- No magic numbers!

### Implementation Strategy

1. **Add DocumentStats struct** (new file: `src/processors/stats.rs`)
2. **Update BlockMergeProcessor** to use DocumentStats
3. **Update Processor trait** to pass stats context (breaking change)
4. **Update tests** to verify adaptive behavior

### Alternative: Minimal Breaking Change

Instead of changing Processor trait, calculate stats inside BlockMergeProcessor:

```rust
impl Processor for BlockMergeProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        let stats = DocumentStats::from_document(&document);
        // ... use stats
    }
}
```

This avoids breaking other processors but repeats calculation if multiple processors need stats.

### Risk Mitigation

1. **Backward Compatibility:**

   - Keep `with_params()` constructor for tests
   - Add `from_document()` constructor for production

2. **Performance:**

   - Stats calculation is O(n) where n = block count
   - Only done once per document
   - Negligible overhead

3. **Edge Cases:**
   - Documents with no text → use default values
   - Documents with highly variable fonts → use median
   - Empty pages → skip or use document-wide stats

### Expected Impact

- **Style Accuracy:** +5-8 points (better merge boundaries)
- **Robustness:** +10-15 points (works on any document type)
- **Maintainability:** +++ (no magic numbers to tune)
- **Correctness:** +++ (principled approach)
