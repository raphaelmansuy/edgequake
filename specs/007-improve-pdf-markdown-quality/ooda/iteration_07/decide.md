# Iteration 07: DECIDE - Action Plan

## Decision

Rewrite `detect_columns()` in `pymupdf_grouper.rs` to support N columns using histogram-based gutter detection.

## Algorithm

```text
1. Collect all line right-edges (x1) and left-edges (x0)
2. Build histogram of "coverage" at each X position
3. Find regions with zero coverage (gutters)
4. Filter gutters: must have lines on both sides
5. Return columns as ranges between gutters

Example for 3-column layout:

  Lines:    ████████    ████████    ████████
  X:        0   100  120  220  240   340
  Gutters:        [100-120]   [220-240]
  Columns:  [0-110]  [110-230]  [230-340]
```

## Code Changes

### File: `src/layout/pymupdf_grouper.rs`

Replace `detect_columns()` (lines 532-607) with:

```rust
fn detect_columns(&self, lines: &[Line]) -> Vec<(f32, f32)> {
    if lines.len() < 4 { return vec![]; }

    // 1. Get page bounds
    let page_left = lines.iter().map(|l| l.x0).fold(f32::MAX, f32::min);
    let page_right = lines.iter().map(|l| l.x1).fold(f32::MIN, f32::max);
    let page_width = page_right - page_left;

    if page_width < 100.0 { return vec![]; }

    // 2. Build coverage histogram
    let bucket_width = 5.0; // 5pt resolution
    let num_buckets = ((page_width / bucket_width).ceil() as usize).max(1);
    let mut coverage = vec![0usize; num_buckets];

    for line in lines {
        if line.x1 - line.x0 > page_width * 0.8 { continue; } // Skip headers
        let start = ((line.x0 - page_left) / bucket_width) as usize;
        let end = ((line.x1 - page_left) / bucket_width) as usize;
        for i in start..=end.min(num_buckets - 1) {
            coverage[i] += 1;
        }
    }

    // 3. Find gutters (runs of zero coverage)
    let mut gutters = vec![];
    let min_gutter_buckets = 2; // ~10pt minimum
    let mut gutter_start = None;

    for (i, &count) in coverage.iter().enumerate() {
        if count == 0 {
            if gutter_start.is_none() { gutter_start = Some(i); }
        } else if let Some(start) = gutter_start {
            if i - start >= min_gutter_buckets {
                let gutter_x = page_left + ((start + i) as f32 / 2.0) * bucket_width;
                gutters.push(gutter_x);
            }
            gutter_start = None;
        }
    }

    // 4. Convert gutters to columns
    if gutters.is_empty() { return vec![]; }

    let mut columns = vec![];
    let mut prev = page_left;
    for gutter in gutters {
        columns.push((prev, gutter));
        prev = gutter;
    }
    columns.push((prev, page_right));

    columns
}
```

## Success Criteria

1. Existing 2-column tests still pass
2. New 3-column test passes:
   - Read left column first
   - Then middle column
   - Then right column
3. 512+ tests passing (no regression)
