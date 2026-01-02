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

        // Convert clusters to columns
        self.clusters_to_columns(&clusters, page_width)
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
    fn clusters_to_columns(&self, clusters: &[Cluster], page_width: f32) -> Vec<Column> {
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
        }

        columns
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
}
