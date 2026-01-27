# ORIENT.md - Iteration 009

**Directory:** `edgequake/crates/edgequake-pdf/src/processors`

**Timestamp:** 2026-01-02

## Strategy: Use DocumentStats Line Spacing

### Solution

**Reuse DocumentStats from Loop 007!**

The `typical_line_spacing` field already provides exactly what we need.

### Implementation

```rust
impl Processor for HyphenContinuationProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        // Calculate stats once (First Principles!)
        let stats = DocumentStats::from_document(&document);

        // Adaptive threshold: 2.5x typical line spacing
        let max_vertical_gap = stats.typical_line_spacing * 2.5;

        for page in &mut document.pages {
            let mut i = 0;
            while i < page.blocks.len() {
                if i + 1 < page.blocks.len() {
                    let current = &page.blocks[i];
                    let next = &page.blocks[i + 1];

                    if current.block_type == BlockType::Text && next.block_type == BlockType::Text {
                        let vertical_gap = (next.bbox.y1 - current.bbox.y2).abs();

                        let ends_hyph = self.ends_with_hyphen(&current.text);
                        let starts_cont = self.starts_with_continuation(&next.text);

                        // USE ADAPTIVE THRESHOLD!
                        if vertical_gap <= max_vertical_gap {
                            if ends_hyph.is_some() && starts_cont {
                                join_with = Some(i + 1);
                            }
                        }
                    }
                }
                // ... rest of logic
            }
        }
        Ok(document)
    }
}
```

### Why 2.5x?

- **Consistency:** BlockMergeProcessor uses 2.5x in Loop 007
- **Coverage:** Handles single-spaced (1.2x) to near double-spaced (2.0x)
- **Tolerance:** Allows some paragraph spacing
- **Proven:** Already working in BlockMergeProcessor

### Performance Impact

- **Overhead:** DocumentStats already calculated if BlockMergeProcessor runs first
- **Calculation:** O(n) but done once per document
- **Result:** Negligible (< 0.1ms)

### Simple and Clean

One import, one calculation, one variable, one line change. Perfect!
