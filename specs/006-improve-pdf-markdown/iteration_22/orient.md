# Orient – OODA-22: Implement Layout Analysis Confidence

## Current State

The `LayoutAnalyzer::analyze()` method has a hardcoded `confidence: 0.9`.

## What Confidence Should Measure

A good layout analysis confidence should reflect:
1. **Column detection clarity** - Clear column boundaries = high confidence
2. **Reading order consistency** - All blocks assigned = high confidence
3. **Region detection quality** - XY-cut produced meaningful regions

## Implementation Strategy

Calculate confidence as a weighted average of sub-scores:

```rust
fn calculate_confidence(
    blocks: &[Block],
    columns: &[BoundingBox],
    reading_order: &[usize],
    regions: &[LayoutRegion],
) -> f32 {
    // 1. Reading order coverage: all blocks should be in reading order
    let order_coverage = if blocks.is_empty() {
        1.0
    } else {
        reading_order.len() as f32 / blocks.len() as f32
    };
    
    // 2. Column detection confidence (from detector if available)
    let column_confidence = if columns.is_empty() || columns.len() == 1 {
        1.0 // Single column is always confident
    } else {
        0.95 // Multi-column has slight uncertainty
    };
    
    // 3. Region quality: XY-cut produced meaningful splits
    let region_confidence = if regions.is_empty() {
        0.8 // No regions detected
    } else if regions.len() > blocks.len() * 2 {
        0.7 // Over-fragmented
    } else {
        1.0 // Reasonable region count
    };
    
    // Weighted average (reading order is most important)
    (order_coverage * 0.5 + column_confidence * 0.3 + region_confidence * 0.2)
        .clamp(0.0, 1.0)
}
```
