# Iteration 07: ACT - N-Column Detection Algorithm

## Implementation Summary

Rewrote `detect_columns()` in `pymupdf_grouper.rs` to use a histogram-based approach
that finds ALL gutters, enabling support for 3+ column layouts.

## Changes Made

### 1. `src/layout/pymupdf_grouper.rs` (lines 530-660)

**Replaced old detect_columns with histogram algorithm:**

```rust
fn detect_columns(&self, lines: &[Line]) -> Vec<(f32, f32)> {
    // Build coverage histogram with 5pt resolution
    let bucket_width = 5.0;
    let num_buckets = ((page_width / bucket_width).ceil() as usize).max(1);
    let mut coverage = vec![0usize; num_buckets];

    // Count how many lines cover each bucket
    for line in lines {
        // Skip wide lines (headers, titles)
        if line.x1 - line.x0 > page_width * 0.8 {
            continue;
        }
        let start = ((line.x0 - page_left) / bucket_width) as usize;
        let end = ((line.x1 - page_left) / bucket_width) as usize;
        for i in start..=end.min(num_buckets - 1) {
            coverage[i] += 1;
        }
    }

    // Find gutters: runs of zero/low coverage
    // Gutter = region where no text exists
    // N gutters → N+1 columns
    ...
}
```

**Algorithm ASCII diagram:**

```
Lines:    ████████    ████████    ████████
X:        0   100  120  220  240   340
Coverage: [5,5,5,0,0,0,5,5,5,0,0,0,5,5,5]
           ↑         ↑         ↑
        col1      gutter1   gutter2

Gutters found at: [100-120], [220-240]
Columns output: [(0,110), (110,230), (230,340)]
```

### 2. `src/layout/pymupdf_structs.rs` (lines 399-410)

**Added test helper constructor:**

```rust
#[cfg(test)]
pub fn new_with_bbox(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
    Self { x0, y0, x1, y1, page_num: 0, spans: vec![] }
}
```

### 3. Added Unit Tests (lines 1430-1525)

Three new tests for column detection:

| Test | Purpose |
|------|---------|
| `test_detect_two_columns` | Classic 2-column layout with 100pt gutter |
| `test_detect_three_columns` | Validates N-column support (was BROKEN) |
| `test_detect_single_column` | Full-width text returns empty (no detection) |

## Test Results

```
$ cargo test --package edgequake-pdf --lib -- --test-threads=4

test result: ok. 515 passed; 0 failed; 0 ignored
```

**Test count: 512 → 515** (+3 column detection tests)

## Commit

```
commit 0dd8f5e7
Author: Claude OODA
Date:   [timestamp]

OODA-IT07: N-column detection with histogram algorithm

WHY: Previous detect_columns only found ONE gutter near center (50%),
breaking 3-column layouts where gutters are at 33% and 66%.

WHAT:
- Rewrote detect_columns with histogram-based gutter detection
- Build coverage histogram (5pt resolution) across full page width
- Find zero-coverage runs as gutters (min 10pt, max 1 line crossing)
- Convert N gutters to N+1 column boundaries
- Added Line::new_with_bbox test helper

TESTS:
- test_detect_two_columns: validates 2-column with clear gutter
- test_detect_three_columns: validates N-column support (was broken)
- test_detect_single_column: validates full-width returns empty

QUALITY: Multi-column score target 60→85 (critical priority from mission)
```

## Quality Impact

| Category | Before IT07 | After IT07 | Target |
|----------|-------------|------------|--------|
| Multi-column layouts | 60/100 | 70/100 (est.) | 85/100 |

**Rationale:** Algorithm now correctly detects N columns, but quality also depends
on reading order reconstruction (handled in `lines_to_blocks`). Further iterations
needed to improve block assignment after column detection.

## Next Steps (IT08)

1. Test with real 3-column PDF (e.g., `lighrag_2410.05779v3.pdf`)
2. Validate reading order is correct after column detection
3. Consider improvements to tables (50→80 target, critical priority)
