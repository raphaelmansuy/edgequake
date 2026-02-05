# Act – OODA-17: Add Column Layout ASCII Diagram

## What Changed

Added comprehensive ASCII diagram and documentation to `text_grouping.rs` module docs:

1. **Two-Column Layout Detection section**:
   - 4-step algorithm overview
   - ASCII diagram showing page zones (header, title/authors, columns, footer)
   - Y-normalization explanation
2. **Reading Order section**:
   - Explains column-by-column reading (not Y-interleaved)
3. **Key Thresholds section**:
   - References OODA-09 for threshold documentation
   - Lists column_boundary, margin, header/footer thresholds

## Code Location

- `edgequake/crates/edgequake-pdf/src/backend/text_grouping.rs` lines 1-50

## Verification

```
cargo test --lib
# Result: 454 passed; 0 failed; 0 ignored
```

## Value Added

- Developers can now understand the two-column algorithm at a glance
- ASCII diagram visualizes page zone detection
- Links to OODA-09 for detailed threshold documentation
- Reading order explanation prevents common misunderstanding

## Next Iteration

OODA-18: Continue documentation or add tests for edge cases
