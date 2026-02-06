# OODA Iteration 20 - Decide

## Decision: Add table-vs-column discriminator to zone-based detection

### Location

`src/backend/column_detection.rs`, in the zone-based detection block (line ~349)

### Algorithm

After the zone-based detection determines `left_starts >= 3 && right_starts >= 3 && balance > 0.15`:

1. Filter out elements whose width spans > 50% of page (titles, descriptions)
2. Separate remaining elements by the detected boundary
3. Compute avg text length for both left and right
4. Count Y-aligned pairs (tolerance: 5pt)
5. If avg text length < 15 chars AND Y-alignment ratio > 0.6 → return None (table, not columns)

### Function Signature

```rust
fn looks_like_table_not_columns(
    elements: &[TextElement],
    boundary: f32,
    page_width: f32,
) -> bool
```

### Tests to Add

1. `test_table_not_detected_as_columns` - Short cells in grid pattern → true
2. `test_columns_not_detected_as_table` - Long text in columns → false
3. `test_mixed_content_not_table` - Title + short items → false (not enough alignment)

### Risk Assessment

- **Low risk**: Only affects the zone-based fallback, which fires AFTER peak detection fails
- **Conservative**: Requires BOTH short text AND high Y-alignment
- **No regression**: Two-column papers won't trigger (long text > 15 chars avg)
