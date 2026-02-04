# OODA-05 Act: Font Style Detection & Header Levels

## Changes Implemented

### 1. Bold Font Detection (`pymupdf_structs.rs`)

Added "medi", "semi", "demi" patterns to `is_bold()` to detect Medium weight fonts as bold:

```rust
lower.contains("medi")  // Medium (NimbusRomNo9L-Medi)
|| lower.contains("semi")  // SemiBold
|| lower.contains("demi")  // DemiBold
```

### 2. Header Level Thresholds (`pymupdf_grouper.rs`)

Adjusted ratios for better H1-H4 mapping:
| Ratio | Before | After |
|-------|--------|-------|
| ≥1.8 | H1 | H1 |
| ≥1.4 | H3 | H2 |
| ≥1.3 | H4 | H3 |
| ≥1.25 | H5 | H4 |

## Evaluation Results

| Metric      | OODA-04 | OODA-05 | Delta  |
| ----------- | ------- | ------- | ------ |
| **Quality** | 0.675   | 0.702   | +4.0%  |
| ROUGE-L     | 0.702   | 0.701   | -0.1%  |
| Word F1     | 0.899   | 0.897   | -0.2%  |
| Structure   | 0.350   | 0.453   | +29.4% |
| Format      | 0.343   | 0.470   | +37.0% |

## Analysis

✅ **Structure score improved significantly** (+29.4%) - header level thresholds now better match academic paper font ratios
✅ **Format score improved** (+37.0%) - Medium weight fonts now detected as bold
⚠️ **ROUGE-L flat** - expected, no reading order changes

## Files Modified

- `layout/pymupdf_structs.rs` - `is_bold()` patterns
- `layout/pymupdf_grouper.rs` - header level thresholds

## Next Focus

- **Structure** still lowest at 0.453 - investigate heading detection algorithm
- **v2_2512.25072v1** at 0.558 quality - deep dive needed
