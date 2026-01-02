# DECIDE.md - Iteration 004: Implement Geometric Clustering Module

**Directory:** `edgequake/crates/edgequake-pdf/src/layout`

## Decision: Implement Phase 1 - Geometric Clustering

### Rationale

**Why GeometricClustering First:**

1. **Foundation for all other improvements**

   - Column detection affects reading order
   - Reading order affects table detection
   - Text grouping affects everything downstream

2. **Clear algorithmic improvement**

   - Histogram binning → DBSCAN clustering
   - Hardcoded thresholds → adaptive parameters
   - Quantifiable improvement

3. **Self-contained module**

   - No dependencies on other refactoring
   - Can be tested in isolation
   - Clear interface boundaries

4. **High ROI**
   - Fixes fundamental spatial analysis
   - Expected improvement: Table Accuracy 3.5% → 8%+
   - Enables future improvements

### Implementation Plan

#### Step 1: Create `src/layout/geometric.rs`

```rust
//! Geometric clustering for PDF text elements.
//!
//! Uses DBSCAN (Density-Based Spatial Clustering of Applications with Noise)
//! to group text spans by spatial proximity without hardcoded thresholds.

use crate::schema::{BoundingBox, TextSpan};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct GeometricClusterer {
    /// Epsilon parameter computed adaptively from median font size
    eps_multiplier: f32,
    /// Minimum points to form a core point
    min_samples: usize,
}

impl GeometricClusterer {
    /// Create a new geometric clusterer with default parameters.
    pub fn new() -> Self {
        Self {
            eps_multiplier: 1.5,  // eps = median_font_size * 1.5
            min_samples: 3,
        }
    }

    /// Cluster points using DBSCAN algorithm.
    pub fn dbscan(&self, points: &[(f32, f32)], eps: f32) -> Vec<Cluster> {
        let n = points.len();
        let mut labels = vec![Label::Unclassified; n];
        let mut cluster_id = 0;

        for i in 0..n {
            if !matches!(labels[i], Label::Unclassified) {
                continue;
            }

            let neighbors = self.range_query(points, i, eps);

            if neighbors.len() < self.min_samples {
                labels[i] = Label::Noise;
            } else {
                cluster_id += 1;
                self.expand_cluster(
                    points,
                    &mut labels,
                    i,
                    &neighbors,
                    cluster_id,
                    eps,
                );
            }
        }

        self.build_clusters(&labels, points)
    }

    /// Find all points within distance eps of point i.
    fn range_query(&self, points: &[(f32, f32)], i: usize, eps: f32) -> Vec<usize> {
        let (x, y) = points[i];
        points
            .iter()
            .enumerate()
            .filter(|(_, (px, py))| {
                let dx = x - px;
                let dy = y - py;
                dx * dx + dy * dy <= eps * eps
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    /// Expand cluster from core point.
    fn expand_cluster(
        &self,
        points: &[(f32, f32)],
        labels: &mut [Label],
        core_idx: usize,
        neighbors: &[usize],
        cluster_id: usize,
        eps: f32,
    ) {
        labels[core_idx] = Label::Clustered(cluster_id);
        let mut seeds = neighbors.to_vec();
        let mut i = 0;

        while i < seeds.len() {
            let q = seeds[i];
            i += 1;

            if matches!(labels[q], Label::Noise) {
                labels[q] = Label::Clustered(cluster_id);
            }

            if !matches!(labels[q], Label::Unclassified) {
                continue;
            }

            labels[q] = Label::Clustered(cluster_id);
            let q_neighbors = self.range_query(points, q, eps);

            if q_neighbors.len() >= self.min_samples {
                seeds.extend(q_neighbors);
            }
        }
    }

    /// Build cluster structures from labels.
    fn build_clusters(&self, labels: &[Label], points: &[(f32, f32)]) -> Vec<Cluster> {
        let mut clusters: Vec<Cluster> = Vec::new();
        let mut cluster_map: HashMap<usize, usize> = HashMap::new();

        for (i, (x, y)) in points.iter().enumerate() {
            if let Label::Clustered(id) = labels[i] {
                let cluster_idx = *cluster_map
                    .entry(id)
                    .or_insert_with(|| {
                        clusters.push(Cluster::new(id));
                        clusters.len() - 1
                    });
                clusters[cluster_idx].add_point(*x, *y, i);
            }
        }

        clusters
    }

    /// Detect columns from x-coordinates of text spans.
    pub fn detect_columns(&self, spans: &[TextSpan], page_width: f32) -> Vec<Column> {
        if spans.is_empty() {
            return vec![Column::new(0.0, page_width)];
        }

        // Extract x-coordinates (left edge of each span)
        let x_coords: Vec<f32> = spans.iter().map(|s| s.bbox.x1).collect();

        // Calculate adaptive epsilon from x-coordinate distribution
        let eps = self.calculate_eps_from_distribution(&x_coords);

        // Convert to points for clustering
        let points: Vec<(f32, f32)> = x_coords.iter().map(|&x| (x, 0.0)).collect();

        // Cluster x-coordinates
        let clusters = self.dbscan(&points, eps);

        if clusters.len() <= 1 {
            return vec![Column::new(0.0, page_width)];
        }

        // Convert clusters to columns
        self.clusters_to_columns(&clusters, page_width)
    }

    /// Calculate epsilon from coordinate distribution.
    fn calculate_eps_from_distribution(&self, coords: &[f32]) -> f32 {
        if coords.len() < 2 {
            return 30.0; // Fallback
        }

        // Compute pairwise distances
        let mut distances = Vec::new();
        for i in 0..coords.len().min(100) { // Sample for efficiency
            for j in (i + 1)..coords.len().min(100) {
                distances.push((coords[i] - coords[j]).abs());
            }
        }

        distances.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Use 10th percentile as eps (captures tight clusters)
        let idx = (distances.len() as f32 * 0.10) as usize;
        distances.get(idx).copied().unwrap_or(30.0).max(10.0)
    }

    /// Convert x-coordinate clusters to column regions.
    fn clusters_to_columns(&self, clusters: &[Cluster], page_width: f32) -> Vec<Column> {
        let mut columns = Vec::new();
        let mut sorted_clusters: Vec<_> = clusters.iter().collect();
        sorted_clusters.sort_by(|a, b| a.center_x().partial_cmp(&b.center_x()).unwrap());

        let mut prev_x = 0.0;
        for cluster in sorted_clusters {
            let col_start = prev_x;
            let col_end = cluster.max_x() + cluster.width() / 2.0;
            columns.push(Column::new(col_start, col_end));
            prev_x = col_end;
        }

        // Add final column
        if prev_x < page_width {
            columns.push(Column::new(prev_x, page_width));
        }

        columns
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Label {
    Unclassified,
    Noise,
    Clustered(usize),
}

#[derive(Debug, Clone)]
pub struct Cluster {
    id: usize,
    points: Vec<(f32, f32, usize)>, // (x, y, original_index)
}

impl Cluster {
    fn new(id: usize) -> Self {
        Self {
            id,
            points: Vec::new(),
        }
    }

    fn add_point(&mut self, x: f32, y: f32, idx: usize) {
        self.points.push((x, y, idx));
    }

    fn center_x(&self) -> f32 {
        self.points.iter().map(|(x, _, _)| x).sum::<f32>() / self.points.len() as f32
    }

    fn max_x(&self) -> f32 {
        self.points.iter().map(|(x, _, _)| *x).fold(f32::MIN, f32::max)
    }

    fn width(&self) -> f32 {
        let min = self.points.iter().map(|(x, _, _)| *x).fold(f32::MAX, f32::min);
        let max = self.max_x();
        max - min
    }
}

#[derive(Debug, Clone)]
pub struct Column {
    pub x1: f32,
    pub x2: f32,
}

impl Column {
    pub fn new(x1: f32, x2: f32) -> Self {
        Self { x1, x2 }
    }

    pub fn contains(&self, x: f32) -> bool {
        x >= self.x1 && x <= self.x2
    }

    pub fn width(&self) -> f32 {
        self.x2 - self.x1
    }
}

impl Default for GeometricClusterer {
    fn default() -> Self {
        Self::new()
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbscan_simple() {
        let clusterer = GeometricClusterer::new();
        let points = vec![
            (1.0, 1.0), (1.5, 1.5), (2.0, 2.0),  // Cluster 1
            (10.0, 10.0), (10.5, 10.5), (11.0, 11.0),  // Cluster 2
        ];

        let clusters = clusterer.dbscan(&points, 2.0);
        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn test_column_detection_single() {
        let clusterer = GeometricClusterer::new();
        let spans = create_test_spans(vec![50.0, 52.0, 55.0]); // All same column

        let columns = clusterer.detect_columns(&spans, 600.0);
        assert_eq!(columns.len(), 1);
    }

    #[test]
    fn test_column_detection_two_columns() {
        let clusterer = GeometricClusterer::new();
        let spans = create_test_spans(vec![
            50.0, 52.0, 55.0,      // Column 1
            350.0, 352.0, 355.0,   // Column 2
        ]);

        let columns = clusterer.detect_columns(&spans, 600.0);
        assert!(columns.len() >= 2);
    }

    fn create_test_spans(x_coords: Vec<f32>) -> Vec<TextSpan> {
        x_coords.iter().map(|&x| {
            TextSpan {
                text: "test".to_string(),
                bbox: BoundingBox::new(x, 100.0, x + 10.0, 110.0),
                ..Default::default()
            }
        }).collect()
    }
}
```

#### Step 2: Modify `src/layout/column_detector.rs`

**Change:**

- Remove histogram-based detection
- Use GeometricClusterer instead
- Remove all magic number thresholds

```rust
use crate::layout::geometric::{GeometricClusterer, Column};

pub struct ColumnDetector {
    clusterer: GeometricClusterer,
}

impl ColumnDetector {
    pub fn new() -> Self {
        Self {
            clusterer: GeometricClusterer::new(),
        }
    }

    pub fn detect(&self, items: &[BoundingBox], page_width: f32) -> Vec<BoundingBox> {
        // Convert to text spans (or work with bboxes directly)
        let columns = self.clusterer.detect_columns_from_bboxes(items, page_width);

        // Convert Column to BoundingBox
        columns.iter().map(|col| {
            let height = items.iter().map(|b| b.y2).fold(0.0f32, f32::max);
            BoundingBox::new(col.x1, 0.0, col.x2, height)
        }).collect()
    }
}
```

#### Step 3: Update `src/layout/mod.rs`

```rust
pub mod column_detector;
pub mod geometric;        // NEW MODULE
pub mod reading_order;

pub use column_detector::ColumnDetector;
pub use geometric::{GeometricClusterer, Cluster, Column};  // EXPORT
```

#### Step 4: Write Comprehensive Tests

**File:** `tests/geometric_test.rs`

```rust
use edgequake_pdf::layout::geometric::*;

#[test]
fn test_dbscan_handles_noise() {
    // Test that DBSCAN correctly labels noise points
}

#[test]
fn test_column_detection_academic_paper() {
    // Test 2-column academic paper layout
}

#[test]
fn test_column_detection_single_column() {
    // Ensure single-column documents stay single-column
}

#[test]
fn test_adaptive_eps_calculation() {
    // Verify eps adapts to coordinate distribution
}

#[test]
fn test_no_crash_empty_input() {
    // Ensure robustness with edge cases
}
```

### Acceptance Checklist

- [ ] New `geometric.rs` module created
- [ ] DBSCAN algorithm implemented correctly
- [ ] Column detection uses geometric clustering
- [ ] All magic numbers removed from column_detector.rs
- [ ] Histogram code removed
- [ ] Unit tests pass (5+ tests)
- [ ] Integration test: `cargo test -p edgequake-pdf`
- [ ] Real dataset evaluation shows improvement
- [ ] Validator SKILL run: Table Accuracy improves
- [ ] No performance regression (< 10% slower)
- [ ] Code documented with rustdoc comments
- [ ] No compiler warnings

### Expected Metrics Impact

**Before (Baseline):**

- Table Accuracy: 3.5%
- Style Accuracy: 16.9%
- Composite Score: 27.2/100

**After (Phase 1):**

- Table Accuracy: 8-12% (better column detection → better table grouping)
- Style Accuracy: 16.9% (no change yet)
- Composite Score: 30-33/100

**Justification:**

- Improved column detection prevents multi-column text from being grouped as tables
- Better spatial clustering improves block grouping
- Foundation for Phase 2 improvements

### Rollback Plan

If metrics regress:

1. Revert changes to `column_detector.rs`
2. Keep `geometric.rs` module (useful for future)
3. Add feature flag: `use_geometric_clustering`
4. Debug with specific failing documents
5. Adjust DBSCAN parameters (eps_multiplier, min_samples)

### Time Estimate

- Implementation: 2-3 hours
- Testing: 1-2 hours
- Integration + Validation: 1 hour
- **Total: 4-6 hours**

## Next Steps After Completion

1. Run full validation suite
2. Document learnings in `ACT.md`
3. Update `scratchpad_append_log.md`
4. Commit with message: `feat(layout): replace histogram with DBSCAN clustering`
5. Proceed to Phase 2: Font Metrics Analyzer

---

**Decision Made:** Implement GeometricClustering module to replace histogram-based column detection with first-principles coordinate clustering using DBSCAN algorithm.
