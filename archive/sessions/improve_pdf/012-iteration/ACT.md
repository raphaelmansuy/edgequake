# ACT - Loop 012

## Timestamp

Fri Jan 2, 2026 20:00:00 HKT

## User Feedback - Refocus on Tables

**Critical insight from user**: "You must focus on table" - Tables have 2.4% accuracy with 40% weight (same as Style)

## Directory Scope

**Target: crates/edgequake-pdf/src/backend/lattice.rs**

## First Principles Analysis

### What We Discovered

1. **Tables ARE being detected** - Lattice engine finds bounding boxes correctly
2. **Column detection is broken** - `detect_columns_by_whitespace()` fails on real tables
3. **Cell content extraction fails** - Wrong column boundaries → text assigned to wrong cells

### Root Cause (Whitespace Detection Failure)

**Current approach (lines 317-400 in lattice.rs):**

```rust
// Uses 1-point resolution X-projection
let mut projection = vec![0; width + 1];
// Looks for gaps in projection to find columns
```

**Why it fails:**

1. **Multi-word cells**: "Claude3.7-Sonnet" has internal spaces but should be ONE cell
2. **Variable spacing**: Column gaps vary (5-20 pts) depending on content width
3. **Alignment issues**: Cells not perfectly aligned vertically
4. **Resolution too coarse**: 1-point bins don't capture subtle gaps

### Example from one_tool_2512.20957v2.pdf

**Gold table (10 columns):**

```markdown
| Agent Pipeline | Model | Recall | Precision | Sample-F1 | IoU | Recall | Precision | Sample-F1 | IoU |
| RepoSearcher | Claude3.7-Sonnet | 66.80 | 28.30 | 19.90 | 17.89 | 89.71 | 33.15 | 21.04 | 20.67 |
```

**Generated (mangled):**

```markdown
| One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents |
| -------------------------------------------------------------------------- |
```

The lattice engine detected the table bbox but failed to identify 10 columns, so it fell back to a 2-column default.

## Solution: Geometric Clustering for Columns

**Same first-principles approach that fixed Loop 004 column detection:**

### Algorithm

1. **Collect X-coordinates** of all text elements within table bbox
2. **Apply DBSCAN clustering** on X-coordinates:
   - Epsilon: 10th percentile of inter-element distances (adaptive)
   - MinPts: 1 (each column may have sparse elements)
3. **Extract column boundaries** from cluster centroids
4. **Sort columns** left-to-right
5. **Use as X-grid** for `create_table_block()`

### Why This Works

- **Adaptive**: Epsilon adjusts to actual column spacing
- **Robust**: Handles multi-word cells (clustered together)
- **No magic numbers**: Data-driven, not heuristic-based
- **Proven**: Same technique gave +14.6 points in Style Accuracy (Loop 004)

## Implementation Plan

### Change 1: Add DBSCAN to lattice.rs

```rust
// Import from geometric.rs (already exists from Loop 004)
use crate::layout::geometric::dbscan_1d;

fn detect_columns_by_clustering(
    &self,
    text_elements: &[TextElement],
    bbox: &BoundingBox,
) -> Vec<f32> {
    // Collect X coordinates
    let x_coords: Vec<f32> = text_elements
        .iter()
        .filter(|e| bbox_contains(bbox, e.x, e.y))
        .map(|e| e.x)
        .collect();

    if x_coords.len() < 2 {
        return Vec::new();
    }

    // Compute adaptive epsilon (10th percentile of distances)
    let mut sorted_x = x_coords.clone();
    sorted_x.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut distances: Vec<f32> = sorted_x
        .windows(2)
        .map(|w| w[1] - w[0])
        .filter(|&d| d > 0.5) // Ignore sub-point distances
        .collect();
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let epsilon = if distances.is_empty() {
        5.0 // fallback
    } else {
        let p10_idx = (distances.len() as f32 * 0.10).ceil() as usize;
        distances[p10_idx.min(distances.len() - 1)]
    };

    // Apply DBSCAN
    let clusters = dbscan_1d(&sorted_x, epsilon, 1);

    // Extract column boundaries (cluster centroids)
    let mut col_boundaries = Vec::new();
    for cluster in &clusters {
        if cluster.is_empty() {
            continue;
        }
        let sum: f32 = cluster.iter().sum();
        let centroid = sum / cluster.len() as f32;
        col_boundaries.push(centroid);
    }

    col_boundaries.sort_by(|a, b| a.partial_cmp(b).unwrap());
    col_boundaries
}
```

### Change 2: Replace whitespace detection call

```rust
// Line ~257 in lattice.rs
if unique_y.len() >= 2 && unique_x.len() < 2 {
    let detected_x = self.detect_columns_by_clustering(text_elements, &bbox);
    if detected_x.len() >= 2 {
        unique_x = detected_x;
    }
}
```

## Predicted Impact

### Metrics Improvement

- **Before**: Composite 32.5/100 (Table 2.4%, Style 31.5%)
- **Target**: Composite 40-45/100 (Table 15-20%, Style 31.5%)

### Reasoning

- Table Accuracy has 40% weight, currently at 2.4%
- Fixing column detection will correctly extract 10-column tables
- Even achieving 15% Table Accuracy → +5 composite points
- Achieving 20% Table Accuracy → +7 composite points

### Drift Reduction

- **table:mismatch**: 140 → ~50 (-65%)
- **content:mismatch**: 2067 → ~1800 (-13% indirect)

## Status

**PAUSED** - Refocusing on table extraction per user guidance

**Next Steps:**

1. Implement `detect_columns_by_clustering()` using geometric DBSCAN
2. Add unit tests for column detection with multi-column tables
3. Run validator and measure Table Accuracy improvement
4. Document approach in Loop 012 artifacts

## Lessons Learned

- Don't chase low-impact issues (headings) when high-impact ones exist (tables)
- Check gold files critically - they may not be perfect
- Focus on metrics with highest ROI (Table Accuracy: 2.4% vs Style: 31.5%)
- Reuse successful patterns (DBSCAN worked for column detection in Loop 004)
