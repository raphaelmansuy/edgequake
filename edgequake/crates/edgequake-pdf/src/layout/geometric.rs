//! Geometric clustering for PDF text elements.
//!
//! Uses DBSCAN (Density-Based Spatial Clustering of Applications with Noise)
//! to group text spans by spatial proximity without hardcoded thresholds.
//!
//! ## First Principles Approach
//!
//! Instead of using histogram binning with magic number thresholds, this module:
//! - Uses actual (x, y) coordinates from PDF text positioning
//! - Applies DBSCAN clustering algorithm for density-based grouping
//! - Calculates adaptive epsilon from coordinate distribution
//! - Works for any layout, scale, or language

use crate::schema::BoundingBox;
use std::collections::HashMap;

/// Geometric clusterer using DBSCAN algorithm.
///
/// **WHY DBSCAN for PDF column detection?**
///
/// Compared to histogram binning with magic thresholds:
/// - No need to specify number of columns a priori
/// - Handles variable-width columns (narrow/wide mix)
/// - Robust to noise (scattered text elements)
/// - Adapts to any document scale or language
///
/// **Key Parameters:**
/// - `min_samples=3`: Prevents single outlier text spans from forming clusters
/// - Epsilon (eps): Calculated adaptively from coordinate distribution
#[derive(Debug, Clone)]
pub struct GeometricClusterer {
    /// Minimum points to form a core point in DBSCAN
    min_samples: usize,
}

impl GeometricClusterer {
    /// Create a new geometric clusterer with default parameters.
    pub fn new() -> Self {
        Self { min_samples: 3 }
    }

    /// Cluster points using DBSCAN algorithm.
    ///
    /// # Arguments
    /// * `points` - Array of (x, y) coordinates
    /// * `eps` - Maximum distance between two points to be considered neighbors
    ///
    /// # Returns
    /// Vector of clusters, each containing point indices
    pub fn dbscan(&self, points: &[(f32, f32)], eps: f32) -> Vec<Cluster> {
        let n = points.len();
        if n == 0 {
            return Vec::new();
        }

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
                self.expand_cluster(points, &mut labels, i, &neighbors, cluster_id, eps);
            }
        }

        self.build_clusters(&labels, points)
    }

    /// Find all points within distance eps of point i.
    fn range_query(&self, points: &[(f32, f32)], i: usize, eps: f32) -> Vec<usize> {
        let (x, y) = points[i];
        let eps_sq = eps * eps;

        points
            .iter()
            .enumerate()
            .filter(|(_, (px, py))| {
                let dx = x - px;
                let dy = y - py;
                dx * dx + dy * dy <= eps_sq
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

        for (i, &(x, y)) in points.iter().enumerate() {
            if let Label::Clustered(id) = labels[i] {
                let cluster_idx = *cluster_map.entry(id).or_insert_with(|| {
                    clusters.push(Cluster::new(id));
                    clusters.len() - 1
                });
                clusters[cluster_idx].add_point(x, y, i);
            }
        }

        clusters
    }

    /// Detect columns from bounding boxes using geometric clustering.
    ///
    /// This is the first-principles approach to column detection:
    /// 1. Extract x-coordinates (actual positions from PDF)
    /// 2. Calculate adaptive epsilon from distribution
    /// 3. Cluster x-coordinates using DBSCAN
    /// 4. Convert clusters to column regions
    ///
    /// No histogram binning, no magic numbers!
    pub fn detect_columns(&self, bboxes: &[BoundingBox], page_width: f32) -> Vec<Column> {
        if bboxes.is_empty() {
            return vec![Column::new(0.0, page_width)];
        }

        // Extract x-coordinates (left edge of each bbox)
        let x_coords: Vec<f32> = bboxes.iter().map(|b| b.x1).collect();

        // Calculate adaptive epsilon from x-coordinate distribution
        let eps = self.calculate_eps_from_distribution(&x_coords);

        // Convert to points for clustering (y=0 since we only care about x-axis)
        let points: Vec<(f32, f32)> = x_coords.iter().map(|&x| (x, 0.0)).collect();

        // Cluster x-coordinates
        let clusters = self.dbscan(&points, eps);

        if clusters.len() <= 1 {
            // Single column or no clear clustering
            return vec![Column::new(0.0, page_width)];
        }

        // ──────────────────────────────────────────────────────────────
        // WHY: Inter-cluster gap validation (OODA-29)
        //
        // DBSCAN can split a single-column document into multiple clusters
        // when text has different indentation levels (e.g., headings at x=72
        // and bullet items at x=94). The 22pt gap is enough for DBSCAN to
        // separate them, but it's NOT a real multi-column boundary.
        //
        // Real multi-column layouts have columns separated by a substantial
        // gutter — typically 30-80pt on a 612pt page. The x-coordinate
        // clusters in a real 2-column document would be ~250pt apart.
        //
        // Guard: If adjacent cluster centers are closer than 15% of page
        // width (~92pt for US Letter), collapse to single column.
        //
        //   Single-column with indent:    Real 2-column layout:
        //   ┌──────────────────────┐      ┌──────────────────────┐
        //   │████ heading (x=72)   │      │████ col1  │ ████ col2│
        //   │  ████ bullet (x=94)  │      │████       │ ████     │
        //   │████ heading (x=72)   │      │████       │ ████     │
        //   │  ████ bullet (x=94)  │      │████       │ ████     │
        //   └──────────────────────┘      └──────────────────────┘
        //   cluster gap: 22pt (FAIL)      cluster gap: ~250pt (PASS)
        // ──────────────────────────────────────────────────────────────
        let min_cluster_separation = page_width * 0.15;
        let mut sorted_clusters: Vec<&Cluster> = clusters.iter().collect();
        sorted_clusters.sort_by(|a, b| {
            a.center_x()
                .partial_cmp(&b.center_x())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let all_well_separated = sorted_clusters
            .windows(2)
            .all(|pair| (pair[1].center_x() - pair[0].center_x()) > min_cluster_separation);

        if !all_well_separated {
            tracing::debug!(
                "COLUMN-DETECT: clusters too close (min_sep={:.1}pt), collapsing to single column",
                min_cluster_separation
            );
            return vec![Column::new(0.0, page_width)];
        }

        // Convert clusters to columns
        let columns = self.clusters_to_columns(&clusters, page_width);

        // ──────────────────────────────────────────────────────────────
        // WHY: Column balance validation (OODA-29)
        //
        // Even after cluster separation check, the column construction
        // algorithm can produce unbalanced columns. In a real multi-column
        // layout, columns have roughly similar widths (±50% variation).
        //
        // If the widest column is >3x the narrowest, this is NOT a real
        // multi-column layout — it's a single column with margin variation.
        //
        //   Balanced (real 2-col):     Unbalanced (false positive):
        //   ┌──────┬──────┐           ┌──┬────────────────┐
        //   │ 280pt│ 280pt│           │94│     518pt      │
        //   │      │      │           │  │                │
        //   └──────┴──────┘           └──┴────────────────┘
        //   ratio: 1.0 (PASS)         ratio: 5.5 (FAIL)
        // ──────────────────────────────────────────────────────────────
        if columns.len() > 1 {
            let min_width = columns
                .iter()
                .map(|c| c.width())
                .fold(f32::INFINITY, f32::min);
            let max_width = columns.iter().map(|c| c.width()).fold(0.0f32, f32::max);
            if min_width > 0.0 && max_width > min_width * 3.0 {
                tracing::debug!(
                    "COLUMN-DETECT: unbalanced columns (max={:.1} / min={:.1} = {:.1}x), collapsing to single column",
                    max_width,
                    min_width,
                    max_width / min_width
                );
                return vec![Column::new(0.0, page_width)];
            }
        }

        columns
    }

    /// Calculate epsilon from coordinate distribution using statistical approach.
    ///
    /// Uses 10th percentile of pairwise distances to capture tight clusters
    /// while avoiding outliers. This adapts to the document's layout.
    fn calculate_eps_from_distribution(&self, coords: &[f32]) -> f32 {
        if coords.len() < 2 {
            return 30.0; // Fallback for degenerate case
        }

        // Sample pairwise distances (up to 100 points for efficiency)
        let sample_size = coords.len().min(100);
        let mut distances = Vec::with_capacity(sample_size * (sample_size - 1) / 2);

        for i in 0..sample_size {
            for j in (i + 1)..sample_size {
                distances.push((coords[i] - coords[j]).abs());
            }
        }

        if distances.is_empty() {
            return 30.0;
        }

        distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Use 10th percentile as eps (captures tight clusters)
        let idx = (distances.len() as f32 * 0.10) as usize;
        distances
            .get(idx)
            .copied()
            .unwrap_or(30.0)
            .max(10.0) // Minimum epsilon
            .min(100.0) // Maximum epsilon (sanity check)
    }

    /// Convert x-coordinate clusters to column regions.
    ///
    /// # OODA-20: Added minimum column width filter
    ///
    /// Columns narrower than MIN_COLUMN_WIDTH are merged with adjacent columns.
    /// This prevents indentation patterns (like bullet points at x=322 vs headers at x=300)
    /// from being misdetected as separate columns.
    fn clusters_to_columns(&self, clusters: &[Cluster], page_width: f32) -> Vec<Column> {
        // Minimum column width in points. Columns narrower than this are likely
        // indentation patterns, not real multi-column layouts.
        const MIN_COLUMN_WIDTH: f32 = 80.0;

        let mut columns = Vec::new();

        // Sort clusters by x-position (left to right)
        let mut sorted_clusters: Vec<_> = clusters.iter().collect();
        sorted_clusters.sort_by(|a, b| {
            a.center_x()
                .partial_cmp(&b.center_x())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut prev_x = 0.0;

        for cluster in sorted_clusters {
            let col_start = prev_x;
            // Column extends to midpoint beyond cluster
            let col_end = (cluster.max_x() + cluster.width() * 0.5).min(page_width);

            if col_end > col_start {
                columns.push(Column::new(col_start, col_end));
            }

            prev_x = col_end;
        }

        // Add final column if there's space
        if prev_x < page_width - 10.0 {
            // 10pt margin
            columns.push(Column::new(prev_x, page_width));
        }

        // If no valid columns, return single column
        if columns.is_empty() {
            columns.push(Column::new(0.0, page_width));
            return columns;
        }

        // OODA-20: Merge narrow columns with adjacent columns
        // This prevents indentation patterns from creating spurious columns
        let mut merged_columns: Vec<Column> = Vec::new();
        for col in columns {
            let width = col.width();
            if width < MIN_COLUMN_WIDTH {
                // Narrow column - merge with previous or next
                if let Some(last) = merged_columns.last_mut() {
                    // Extend the previous column to include this narrow one
                    last.x2 = col.x2;
                } else {
                    // No previous column, just add it (will merge with next)
                    merged_columns.push(col);
                }
            } else if let Some(last) = merged_columns.last_mut() {
                // Check if previous column was narrow (width < MIN_COLUMN_WIDTH)
                if last.width() < MIN_COLUMN_WIDTH {
                    // Extend narrow previous column to this one
                    last.x2 = col.x2;
                } else {
                    merged_columns.push(col);
                }
            } else {
                merged_columns.push(col);
            }
        }

        // Final pass: if we have multiple columns but some are still too narrow, merge them
        let mut final_columns: Vec<Column> = Vec::new();
        for col in merged_columns {
            if col.width() < MIN_COLUMN_WIDTH && !final_columns.is_empty() {
                // Merge narrow column with previous
                let last = final_columns.last_mut().unwrap();
                last.x2 = col.x2;
            } else {
                final_columns.push(col);
            }
        }

        if final_columns.is_empty() {
            final_columns.push(Column::new(0.0, page_width));
        }

        final_columns
    }
}

impl Default for GeometricClusterer {
    fn default() -> Self {
        Self::new()
    }
}

/// Label for DBSCAN algorithm.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Label {
    Unclassified,
    Noise,
    Clustered(usize),
}

/// A cluster of points from DBSCAN.
#[derive(Debug, Clone)]
pub struct Cluster {
    #[allow(dead_code)]
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
        if self.points.is_empty() {
            return 0.0;
        }
        self.points.iter().map(|(x, _, _)| x).sum::<f32>() / self.points.len() as f32
    }

    fn max_x(&self) -> f32 {
        self.points
            .iter()
            .map(|(x, _, _)| *x)
            .fold(f32::MIN, f32::max)
    }

    fn min_x(&self) -> f32 {
        self.points
            .iter()
            .map(|(x, _, _)| *x)
            .fold(f32::MAX, f32::min)
    }

    fn width(&self) -> f32 {
        if self.points.is_empty() {
            return 0.0;
        }
        self.max_x() - self.min_x()
    }

    /// Get the indices of points in this cluster.
    pub fn indices(&self) -> Vec<usize> {
        self.points.iter().map(|(_, _, idx)| *idx).collect()
    }
}

/// A column region in a document.
#[derive(Debug, Clone)]
pub struct Column {
    pub x1: f32,
    pub x2: f32,
}

impl Column {
    pub fn new(x1: f32, x2: f32) -> Self {
        Self { x1, x2 }
    }

    /// Check if an x-coordinate is within this column.
    pub fn contains(&self, x: f32) -> bool {
        x >= self.x1 && x <= self.x2
    }

    /// Get the width of this column.
    pub fn width(&self) -> f32 {
        self.x2 - self.x1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbscan_simple_clusters() {
        let clusterer = GeometricClusterer::new();
        let points = vec![
            (1.0, 1.0),
            (1.5, 1.5),
            (2.0, 2.0), // Cluster 1
            (10.0, 10.0),
            (10.5, 10.5),
            (11.0, 11.0), // Cluster 2
        ];

        let clusters = clusterer.dbscan(&points, 2.0);
        assert_eq!(clusters.len(), 2, "Should detect 2 clusters");
    }

    #[test]
    fn test_dbscan_handles_noise() {
        let clusterer = GeometricClusterer::new();
        let points = vec![
            (1.0, 1.0),
            (1.5, 1.5),
            (2.0, 2.0),     // Cluster
            (100.0, 100.0), // Noise point
        ];

        let clusters = clusterer.dbscan(&points, 2.0);
        assert_eq!(clusters.len(), 1, "Should detect 1 cluster (noise ignored)");
    }

    #[test]
    fn test_column_detection_single() {
        let clusterer = GeometricClusterer::new();
        let bboxes = vec![
            BoundingBox::new(50.0, 100.0, 150.0, 110.0),
            BoundingBox::new(52.0, 120.0, 152.0, 130.0),
            BoundingBox::new(55.0, 140.0, 155.0, 150.0),
        ];

        let columns = clusterer.detect_columns(&bboxes, 600.0);
        assert_eq!(columns.len(), 1, "Should detect single column");
    }

    #[test]
    fn test_column_detection_two_columns() {
        let clusterer = GeometricClusterer::new();
        let bboxes = vec![
            // Left column
            BoundingBox::new(50.0, 100.0, 150.0, 110.0),
            BoundingBox::new(52.0, 120.0, 152.0, 130.0),
            BoundingBox::new(55.0, 140.0, 155.0, 150.0),
            // Right column (far enough to be separate)
            BoundingBox::new(350.0, 100.0, 450.0, 110.0),
            BoundingBox::new(352.0, 120.0, 452.0, 130.0),
            BoundingBox::new(355.0, 140.0, 455.0, 150.0),
        ];

        let columns = clusterer.detect_columns(&bboxes, 600.0);
        assert!(
            columns.len() >= 2,
            "Should detect at least 2 columns, got {}",
            columns.len()
        );
    }

    #[test]
    fn test_no_crash_empty_input() {
        let clusterer = GeometricClusterer::new();
        let bboxes: Vec<BoundingBox> = Vec::new();

        let columns = clusterer.detect_columns(&bboxes, 600.0);
        assert_eq!(columns.len(), 1, "Empty input should return single column");
    }

    // ==========================================================================
    // OODA-29: Test that indentation does NOT create false multi-column
    // ==========================================================================

    #[test]
    fn test_indented_single_column_not_split() {
        let clusterer = GeometricClusterer::new();

        // Simulates a single-column document with headings at x=72
        // and indented bullet items at x=94. The 22pt gap is just
        // indentation, NOT a column boundary.
        let bboxes = vec![
            // Headings at left margin
            BoundingBox::new(72.0, 50.0, 500.0, 70.0),
            BoundingBox::new(72.0, 120.0, 500.0, 140.0),
            BoundingBox::new(72.0, 250.0, 500.0, 270.0),
            BoundingBox::new(72.0, 380.0, 500.0, 400.0),
            BoundingBox::new(72.0, 510.0, 500.0, 530.0),
            BoundingBox::new(72.0, 640.0, 500.0, 660.0),
            BoundingBox::new(72.0, 710.0, 500.0, 730.0),
            // Indented bullet items
            BoundingBox::new(94.0, 80.0, 500.0, 100.0),
            BoundingBox::new(94.0, 150.0, 500.0, 170.0),
            BoundingBox::new(94.0, 280.0, 500.0, 300.0),
            BoundingBox::new(94.0, 410.0, 500.0, 430.0),
            BoundingBox::new(94.0, 540.0, 500.0, 560.0),
            BoundingBox::new(94.0, 670.0, 500.0, 690.0),
            BoundingBox::new(94.0, 740.0, 500.0, 760.0),
        ];

        let columns = clusterer.detect_columns(&bboxes, 612.0);
        assert_eq!(
            columns.len(),
            1,
            "Single-column with indentation should NOT be split into {} columns",
            columns.len()
        );
    }

    #[test]
    fn test_adaptive_eps_calculation() {
        let clusterer = GeometricClusterer::new();

        // Tight distribution
        let coords1 = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let eps1 = clusterer.calculate_eps_from_distribution(&coords1);

        // Loose distribution
        let coords2 = vec![10.0, 50.0, 100.0, 150.0, 200.0];
        let eps2 = clusterer.calculate_eps_from_distribution(&coords2);

        assert!(
            eps1 < eps2,
            "Tight distribution should have smaller eps than loose distribution"
        );
    }

    #[test]
    fn test_cluster_center_calculation() {
        let mut cluster = Cluster::new(1);
        cluster.add_point(0.0, 0.0, 0);
        cluster.add_point(10.0, 0.0, 1);
        cluster.add_point(20.0, 0.0, 2);

        assert_eq!(cluster.center_x(), 10.0, "Center should be 10.0");
        assert_eq!(cluster.min_x(), 0.0, "Min should be 0.0");
        assert_eq!(cluster.max_x(), 20.0, "Max should be 20.0");
        assert_eq!(cluster.width(), 20.0, "Width should be 20.0");
    }

    // ==========================================================================
    // OODA-28: Tests for dbscan_1d()
    // ==========================================================================

    #[test]
    fn test_dbscan_1d_two_clusters() {
        // Two clear clusters separated by large gap
        let values = vec![1.0, 1.5, 2.0, 10.0, 10.5, 11.0];
        let clusters = dbscan_1d(&values, 2.0, 2);
        assert_eq!(clusters.len(), 2, "Should detect 2 clusters");
    }

    #[test]
    fn test_dbscan_1d_single_cluster() {
        // All values very close together - single cluster
        // With eps=3.0 and min_samples=2, all points connect
        let values = vec![1.0, 2.0, 2.5, 3.0];
        let clusters = dbscan_1d(&values, 3.0, 2);
        assert_eq!(clusters.len(), 1, "Should detect 1 cluster");
    }

    #[test]
    fn test_dbscan_1d_empty() {
        // Edge case: empty input
        let values: Vec<f32> = vec![];
        let clusters = dbscan_1d(&values, 2.0, 2);
        assert!(clusters.is_empty(), "Empty input should return no clusters");
    }
}

/// 1D DBSCAN for column/row detection in tables.
///
/// Clusters values in a single dimension using density-based approach.
///
/// # Arguments
/// * `values` - Sorted array of 1D coordinates
/// * `eps` - Maximum distance between two values to be neighbors
/// * `min_samples` - Minimum points to form a cluster
///
/// # Returns
/// Vector of clusters, each containing the coordinate values
pub fn dbscan_1d(values: &[f32], eps: f32, min_samples: usize) -> Vec<Vec<f32>> {
    let n = values.len();
    if n == 0 {
        return Vec::new();
    }

    let mut labels = vec![-1; n]; // -1 = noise, >= 0 = cluster ID
    let mut cluster_id = 0;

    for i in 0..n {
        if labels[i] != -1 {
            continue; // Already processed
        }

        // Find neighbors within eps
        let mut neighbors: Vec<usize> = Vec::new();
        for j in 0..n {
            if (values[j] - values[i]).abs() <= eps {
                neighbors.push(j);
            }
        }

        // Not enough neighbors - mark as noise
        if neighbors.len() < min_samples {
            labels[i] = -1;
            continue;
        }

        // Start new cluster
        labels[i] = cluster_id;
        let mut seed_set = neighbors.clone();
        let mut k = 0;

        while k < seed_set.len() {
            let q = seed_set[k];
            k += 1;

            // If noise, add to cluster
            if labels[q] == -1 {
                labels[q] = cluster_id;
            }

            // Already in cluster
            if labels[q] != -1 {
                continue;
            }

            labels[q] = cluster_id;

            // Find neighbors of q
            let mut q_neighbors: Vec<usize> = Vec::new();
            for j in 0..n {
                if (values[j] - values[q]).abs() <= eps {
                    q_neighbors.push(j);
                }
            }

            // If q is core point, add its neighbors to seed set
            if q_neighbors.len() >= min_samples {
                for &neighbor in &q_neighbors {
                    if labels[neighbor] == -1 || !seed_set.contains(&neighbor) {
                        seed_set.push(neighbor);
                    }
                }
            }
        }

        cluster_id += 1;
    }

    // Group values by cluster ID
    let mut clusters: HashMap<i32, Vec<f32>> = HashMap::new();
    for (i, &label) in labels.iter().enumerate() {
        if label >= 0 {
            clusters.entry(label).or_default().push(values[i]);
        }
    }

    // Convert to vector of clusters
    let mut result: Vec<Vec<f32>> = clusters.into_values().collect();
    result.sort_by(|a, b| {
        let a_min = a.iter().fold(f32::INFINITY, |acc, &x| acc.min(x));
        let b_min = b.iter().fold(f32::INFINITY, |acc, &x| acc.min(x));
        a_min.partial_cmp(&b_min).unwrap()
    });

    result
}
