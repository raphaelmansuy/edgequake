# DECIDE.md - Iteration 007

**Directory:** `edgequake/crates/edgequake-pdf/src/processors`

**Timestamp:** 2026-01-02

## Decision: Implement Statistical Threshold Derivation

### Implementation Plan

#### Phase 1: Create DocumentStats Module

**File:** `src/processors/stats.rs` (new)

**Content:**

```rust
//! Document statistics for adaptive threshold calculation.

use crate::schema::{Block, BlockType, Document};

/// Statistical properties of a PDF document used to derive adaptive thresholds.
#[derive(Debug, Clone)]
pub struct DocumentStats {
    /// Median font size across all blocks (most common body text size).
    pub body_font_size: f32,

    /// Median vertical gap between consecutive lines in the same block.
    pub typical_line_spacing: f32,

    /// Adaptive X-coordinate tolerance for column alignment.
    pub column_alignment_tolerance: f32,

    /// Most common page width in the document.
    pub page_width: f32,

    /// Most common page height in the document.
    pub page_height: f32,
}

impl DocumentStats {
    /// Calculate statistics from a document.
    pub fn from_document(doc: &Document) -> Self {
        let body_font_size = Self::calculate_body_font_size(doc);
        let typical_line_spacing = Self::calculate_line_spacing(doc, body_font_size);
        let column_alignment_tolerance = Self::calculate_alignment_tolerance(doc);
        let (page_width, page_height) = Self::most_common_page_size(doc);

        Self {
            body_font_size,
            typical_line_spacing,
            column_alignment_tolerance,
            page_width,
            page_height,
        }
    }

    fn calculate_body_font_size(doc: &Document) -> f32 {
        let mut sizes: Vec<f32> = doc.pages.iter()
            .flat_map(|p| &p.blocks)
            .filter_map(|b| b.spans.first())
            .filter_map(|s| s.style.size)
            .collect();

        if sizes.is_empty() {
            return 10.0; // Default
        }

        sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Self::percentile(&sizes, 0.5)
    }

    fn calculate_line_spacing(doc: &Document, body_font_size: f32) -> f32 {
        let mut gaps: Vec<f32> = Vec::new();

        for page in &doc.pages {
            let blocks: Vec<&Block> = page.blocks.iter().collect();
            for window in blocks.windows(2) {
                let gap = (window[0].bbox.y1 - window[1].bbox.y2).abs();
                // Filter outliers (gaps > 3x body size are likely paragraph breaks)
                if gap > 0.0 && gap < body_font_size * 3.0 {
                    gaps.push(gap);
                }
            }
        }

        if gaps.is_empty() {
            return body_font_size * 1.4; // Default leading
        }

        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Self::percentile(&gaps, 0.5)
    }

    fn calculate_alignment_tolerance(doc: &Document) -> f32 {
        let mut x_coords: Vec<f32> = doc.pages.iter()
            .flat_map(|p| &p.blocks)
            .map(|b| b.bbox.x1)
            .collect();

        if x_coords.len() < 2 {
            return 20.0; // Default
        }

        x_coords.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Calculate nearest-neighbor distances
        let mut nearest_dists: Vec<f32> = Vec::new();
        for i in 1..x_coords.len() {
            let dist = x_coords[i] - x_coords[i-1];
            if dist > 0.1 {
                nearest_dists.push(dist);
            }
        }

        if nearest_dists.is_empty() {
            return 20.0;
        }

        nearest_dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Self::percentile(&nearest_dists, 0.1)
    }

    fn most_common_page_size(doc: &Document) -> (f32, f32) {
        use std::collections::HashMap;
        let mut size_counts: HashMap<(i32, i32), usize> = HashMap::new();

        for page in &doc.pages {
            let key = (page.width as i32, page.height as i32);
            *size_counts.entry(key).or_insert(0) += 1;
        }

        size_counts.iter()
            .max_by_key(|(_, count)| *count)
            .map(|((w, h), _)| (*w as f32, *h as f32))
            .unwrap_or((612.0, 792.0)) // Letter size default
    }

    fn percentile(sorted: &[f32], p: f32) -> f32 {
        let idx = ((sorted.len() - 1) as f32 * p).max(0.0) as usize;
        sorted[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{BBox, Page, Span, Style};

    #[test]
    fn test_body_font_size_calculation() {
        let doc = create_test_document(vec![10.0, 10.0, 12.0, 10.0, 24.0]);
        let stats = DocumentStats::from_document(&doc);
        assert_eq!(stats.body_font_size, 10.0); // Median
    }

    #[test]
    fn test_line_spacing_calculation() {
        // Create document with blocks spaced at 15pt intervals
        let doc = create_spaced_document(vec![15.0, 15.0, 15.0]);
        let stats = DocumentStats::from_document(&doc);
        assert!((stats.typical_line_spacing - 15.0).abs() < 1.0);
    }

    fn create_test_document(sizes: Vec<f32>) -> Document {
        // Helper to create test documents
        // Implementation omitted for brevity
        todo!()
    }

    fn create_spaced_document(gaps: Vec<f32>) -> Document {
        todo!()
    }
}
```

#### Phase 2: Update BlockMergeProcessor

**File:** `src/processors/processor.rs` (lines ~1400-1500)

**Changes:**

1. Import stats module:

```rust
use super::stats::DocumentStats;
```

2. Remove magic numbers from BlockMergeProcessor struct:

```rust
pub struct BlockMergeProcessor {
    // DELETE these fields:
    // max_vertical_gap: f32,
    // max_margin_diff: f32,

    // Keep empty for now (add optional overrides later if needed)
}
```

3. Update constructor:

```rust
impl BlockMergeProcessor {
    pub fn new() -> Self {
        Self {}
    }

    // Keep for backward compatibility in tests
    #[cfg(test)]
    pub fn with_params(max_vertical_gap: f32, max_margin_diff: f32) -> Self {
        // Deprecated: now calculated automatically
        Self {}
    }
}
```

4. Update `should_merge` signature and logic:

```rust
fn should_merge(
    &self,
    a: &Block,
    b: &Block,
    stats: &DocumentStats
) -> bool {
    // 1. Calculate adaptive thresholds from stats
    let max_vertical_gap = stats.typical_line_spacing * 2.5;
    let vertical_threshold = if a.block_type == BlockType::SectionHeader {
        max_vertical_gap * 1.5
    } else {
        max_vertical_gap
    };

    // 2. Check vertical distance
    let vertical_gap = (a.bbox.y1 - b.bbox.y2).abs();
    if vertical_gap > vertical_threshold {
        return false;
    }

    // 3. Check horizontal alignment
    let margin_diff = (a.bbox.x1 - b.bbox.x1).abs();
    if margin_diff > stats.column_alignment_tolerance {
        return false;
    }

    // 4. Check for column separation
    let horizontal_zone_threshold = stats.page_width * 0.15;
    if margin_diff > horizontal_zone_threshold {
        return false;
    }

    // 5. Same block type check (existing logic)
    if a.block_type != b.block_type {
        return false;
    }

    true
}
```

5. Update `process` method:

```rust
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
```

6. Update `merge_page_blocks` signature:

```rust
fn merge_page_blocks(&self, blocks: Vec<Block>, stats: &DocumentStats) -> Vec<Block> {
    // ... existing logic, update should_merge calls to pass stats
    if self.should_merge(&blocks[i], &blocks[j], stats) {
        // merge logic
    }
}
```

#### Phase 3: Update Module Structure

**File:** `src/processors/mod.rs`

Add new module:

```rust
mod stats;
pub use stats::DocumentStats;
```

#### Phase 4: Update Tests

**File:** `src/processors/processor.rs` (test section)

1. Remove tests that rely on magic numbers
2. Add tests for adaptive behavior:

```rust
#[test]
fn test_block_merge_adapts_to_font_size() {
    // Small font (8pt) - should use smaller gap threshold
    let small_doc = create_document_with_font_size(8.0);
    let processor = BlockMergeProcessor::new();
    let result = processor.process(small_doc).unwrap();

    // Large font (18pt) - should use larger gap threshold
    let large_doc = create_document_with_font_size(18.0);
    let result2 = processor.process(large_doc).unwrap();

    // Verify different merging behavior
    assert!(result.pages[0].blocks.len() != result2.pages[0].blocks.len());
}

#[test]
fn test_block_merge_respects_columns() {
    let multi_column_doc = create_multi_column_document();
    let processor = BlockMergeProcessor::new();
    let result = processor.process(multi_column_doc).unwrap();

    // Verify blocks in different columns aren't merged
    assert_eq!(result.pages[0].blocks.len(), 2);
}
```

### Acceptance Criteria

✅ **All magic numbers removed from BlockMergeProcessor:**

- ~~`max_vertical_gap: 50.0`~~
- ~~`max_margin_diff: 20.0`~~
- ~~`horizontal_zone_threshold: 100.0`~~ (derived from page width)

✅ **All 111 tests pass**

✅ **Adaptive behavior verified:**

- Small fonts use smaller thresholds
- Large fonts use larger thresholds
- Multi-column layouts preserved

✅ **No breaking changes to other processors**

### Implementation Order

1. Create `stats.rs` module (new file)
2. Update `mod.rs` to export DocumentStats
3. Update BlockMergeProcessor implementation
4. Update tests
5. Run `cargo test` to verify
6. Run validator to measure improvement

### Expected Validation Results

**Before Loop 007:**

- Composite Score: ~32-35/100
- Robustness: Low (fails on non-standard documents)

**After Loop 007:**

- Composite Score: ~40-45/100 (estimated +8-10 points)
- Robustness: High (adapts to any document)
- Style Accuracy: +3-5 points (better merge boundaries)

### Rollback Plan

If tests fail:

1. Keep `with_params()` constructor functional
2. Add feature flag: `cfg(feature = "adaptive-thresholds")`
3. Fall back to fixed thresholds by default
4. Debug statistical calculation edge cases

### Migration Notes

**For future processors:**

- Pass `DocumentStats` as context parameter
- Avoid hardcoding pixel thresholds
- Use relative thresholds based on font metrics
