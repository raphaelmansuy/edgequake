//! Geometry types for spatial layout analysis.

use serde::{Deserialize, Serialize};

/// A point in 2D space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    /// Create a new point.
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Calculate the distance to another point.
    pub fn distance(&self, other: &Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Calculate the Manhattan distance to another point.
    pub fn manhattan_distance(&self, other: &Point) -> f32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

impl Default for Point {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// A polygon represented as a series of points.
pub type Polygon = Vec<Point>;

/// Axis-aligned bounding box representation.
///
/// Coordinates are in page units (typically points, 1/72 inch).
/// Origin is at top-left, Y increases downward.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    /// Left edge X coordinate
    pub x1: f32,
    /// Top edge Y coordinate
    pub y1: f32,
    /// Right edge X coordinate
    pub x2: f32,
    /// Bottom edge Y coordinate
    pub y2: f32,
}

impl BoundingBox {
    /// Create a new bounding box from coordinates.
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// Create a bounding box from top-left corner and dimensions.
    pub fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x1: x,
            y1: y,
            x2: x + width,
            y2: y + height,
        }
    }

    /// Create a bounding box from center point and dimensions.
    pub fn from_center(center: Point, width: f32, height: f32) -> Self {
        let half_w = width / 2.0;
        let half_h = height / 2.0;
        Self {
            x1: center.x - half_w,
            y1: center.y - half_h,
            x2: center.x + half_w,
            y2: center.y + half_h,
        }
    }

    /// Create a bounding box that encompasses all given boxes.
    pub fn union_all(boxes: &[BoundingBox]) -> Option<Self> {
        if boxes.is_empty() {
            return None;
        }

        let mut result = boxes[0];
        for bbox in &boxes[1..] {
            result = result.union(bbox);
        }
        Some(result)
    }

    /// Get the width of the bounding box.
    pub fn width(&self) -> f32 {
        self.x2 - self.x1
    }

    /// Get the height of the bounding box.
    pub fn height(&self) -> f32 {
        self.y2 - self.y1
    }

    /// Get the area of the bounding box.
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    /// Get the center point of the bounding box.
    pub fn center(&self) -> Point {
        Point {
            x: (self.x1 + self.x2) / 2.0,
            y: (self.y1 + self.y2) / 2.0,
        }
    }

    /// Get the top-left corner.
    pub fn top_left(&self) -> Point {
        Point {
            x: self.x1,
            y: self.y1,
        }
    }

    /// Get the top-right corner.
    pub fn top_right(&self) -> Point {
        Point {
            x: self.x2,
            y: self.y1,
        }
    }

    /// Get the bottom-left corner.
    pub fn bottom_left(&self) -> Point {
        Point {
            x: self.x1,
            y: self.y2,
        }
    }

    /// Get the bottom-right corner.
    pub fn bottom_right(&self) -> Point {
        Point {
            x: self.x2,
            y: self.y2,
        }
    }

    /// Check if this box contains a point.
    pub fn contains_point(&self, point: &Point) -> bool {
        point.x >= self.x1 && point.x <= self.x2 && point.y >= self.y1 && point.y <= self.y2
    }

    /// Check if this box fully contains another box.
    pub fn contains(&self, other: &BoundingBox) -> bool {
        self.x1 <= other.x1 && self.y1 <= other.y1 && self.x2 >= other.x2 && self.y2 >= other.y2
    }

    /// Check if this box intersects with another box.
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.x1 < other.x2 && self.x2 > other.x1 && self.y1 < other.y2 && self.y2 > other.y1
    }

    /// Calculate the intersection area with another box.
    pub fn intersection_area(&self, other: &BoundingBox) -> f32 {
        let x_overlap = (self.x2.min(other.x2) - self.x1.max(other.x1)).max(0.0);
        let y_overlap = (self.y2.min(other.y2) - self.y1.max(other.y1)).max(0.0);
        x_overlap * y_overlap
    }

    /// Calculate the intersection box with another box.
    pub fn intersection(&self, other: &BoundingBox) -> Option<BoundingBox> {
        if !self.intersects(other) {
            return None;
        }

        Some(BoundingBox {
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
            x2: self.x2.min(other.x2),
            y2: self.y2.min(other.y2),
        })
    }

    /// Calculate the union box with another box.
    pub fn union(&self, other: &BoundingBox) -> BoundingBox {
        BoundingBox {
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
            x2: self.x2.max(other.x2),
            y2: self.y2.max(other.y2),
        }
    }

    /// Calculate Intersection over Union (IoU) with another box.
    ///
    /// IoU is a common metric for measuring overlap between bounding boxes.
    /// Returns a value between 0.0 (no overlap) and 1.0 (identical boxes).
    pub fn iou(&self, other: &BoundingBox) -> f32 {
        let intersection = self.intersection_area(other);
        let union = self.area() + other.area() - intersection;
        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }

    /// Expand the bounding box by a given margin on all sides.
    pub fn expand(&self, margin: f32) -> BoundingBox {
        BoundingBox {
            x1: self.x1 - margin,
            y1: self.y1 - margin,
            x2: self.x2 + margin,
            y2: self.y2 + margin,
        }
    }

    /// Shrink the bounding box by a given margin on all sides.
    pub fn shrink(&self, margin: f32) -> BoundingBox {
        let new_width = (self.width() - 2.0 * margin).max(0.0);
        let new_height = (self.height() - 2.0 * margin).max(0.0);
        let center = self.center();
        BoundingBox::from_center(center, new_width, new_height)
    }

    /// Calculate the vertical gap to another box (positive if other is below).
    pub fn vertical_gap(&self, other: &BoundingBox) -> f32 {
        if other.y1 >= self.y2 {
            other.y1 - self.y2
        } else if self.y1 >= other.y2 {
            -(self.y1 - other.y2)
        } else {
            0.0 // overlapping
        }
    }

    /// Calculate the horizontal gap to another box (positive if other is to the right).
    pub fn horizontal_gap(&self, other: &BoundingBox) -> f32 {
        if other.x1 >= self.x2 {
            other.x1 - self.x2
        } else if self.x1 >= other.x2 {
            -(self.x1 - other.x2)
        } else {
            0.0 // overlapping
        }
    }

    /// Check if this box is roughly aligned horizontally with another.
    pub fn is_horizontally_aligned(&self, other: &BoundingBox, tolerance: f32) -> bool {
        let y_overlap = self.y2.min(other.y2) - self.y1.max(other.y1);
        let min_height = self.height().min(other.height());
        y_overlap >= min_height * (1.0 - tolerance)
    }

    /// Check if this box is roughly aligned vertically with another.
    pub fn is_vertically_aligned(&self, other: &BoundingBox, tolerance: f32) -> bool {
        let x_overlap = self.x2.min(other.x2) - self.x1.max(other.x1);
        let min_width = self.width().min(other.width());
        x_overlap >= min_width * (1.0 - tolerance)
    }

    /// Convert to a polygon (4 corners, clockwise from top-left).
    pub fn to_polygon(&self) -> Polygon {
        vec![
            self.top_left(),
            self.top_right(),
            self.bottom_right(),
            self.bottom_left(),
        ]
    }

    /// Get left half of the box (for XY-cut algorithm).
    pub fn left_half(&self, cut_x: f32) -> BoundingBox {
        BoundingBox {
            x1: self.x1,
            y1: self.y1,
            x2: cut_x,
            y2: self.y2,
        }
    }

    /// Get right half of the box (for XY-cut algorithm).
    pub fn right_half(&self, cut_x: f32) -> BoundingBox {
        BoundingBox {
            x1: cut_x,
            y1: self.y1,
            x2: self.x2,
            y2: self.y2,
        }
    }

    /// Get top half of the box (for XY-cut algorithm).
    pub fn top_half(&self, cut_y: f32) -> BoundingBox {
        BoundingBox {
            x1: self.x1,
            y1: self.y1,
            x2: self.x2,
            y2: cut_y,
        }
    }

    /// Get bottom half of the box (for XY-cut algorithm).
    pub fn bottom_half(&self, cut_y: f32) -> BoundingBox {
        BoundingBox {
            x1: self.x1,
            y1: cut_y,
            x2: self.x2,
            y2: self.y2,
        }
    }
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self {
            x1: 0.0,
            y1: 0.0,
            x2: 0.0,
            y2: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert!((p1.distance(&p2) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_bbox_dimensions() {
        let bbox = BoundingBox::new(10.0, 20.0, 50.0, 80.0);
        assert_eq!(bbox.width(), 40.0);
        assert_eq!(bbox.height(), 60.0);
        assert_eq!(bbox.area(), 2400.0);
    }

    #[test]
    fn test_bbox_center() {
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let center = bbox.center();
        assert_eq!(center.x, 50.0);
        assert_eq!(center.y, 50.0);
    }

    #[test]
    fn test_bbox_intersects() {
        let box1 = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let box2 = BoundingBox::new(25.0, 25.0, 75.0, 75.0);
        let box3 = BoundingBox::new(100.0, 100.0, 150.0, 150.0);

        assert!(box1.intersects(&box2));
        assert!(box2.intersects(&box1));
        assert!(!box1.intersects(&box3));
    }

    #[test]
    fn test_bbox_iou() {
        let box1 = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let box2 = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        assert!((box1.iou(&box2) - 1.0).abs() < 0.001);

        let box3 = BoundingBox::new(50.0, 50.0, 150.0, 150.0);
        // Intersection: 50x50 = 2500
        // Union: 10000 + 10000 - 2500 = 17500
        // IoU: 2500/17500 ≈ 0.143
        let iou = box1.iou(&box3);
        assert!(iou > 0.14 && iou < 0.15);
    }

    #[test]
    fn test_bbox_contains() {
        let outer = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let inner = BoundingBox::new(25.0, 25.0, 75.0, 75.0);
        let outside = BoundingBox::new(50.0, 50.0, 150.0, 150.0);

        assert!(outer.contains(&inner));
        assert!(!outer.contains(&outside));
        assert!(!inner.contains(&outer));
    }

    #[test]
    fn test_bbox_union() {
        let box1 = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let box2 = BoundingBox::new(25.0, 25.0, 100.0, 100.0);
        let union = box1.union(&box2);

        assert_eq!(union.x1, 0.0);
        assert_eq!(union.y1, 0.0);
        assert_eq!(union.x2, 100.0);
        assert_eq!(union.y2, 100.0);
    }

    #[test]
    fn test_bbox_expand_shrink() {
        let bbox = BoundingBox::new(10.0, 10.0, 90.0, 90.0);

        let expanded = bbox.expand(5.0);
        assert_eq!(expanded.x1, 5.0);
        assert_eq!(expanded.y1, 5.0);
        assert_eq!(expanded.x2, 95.0);
        assert_eq!(expanded.y2, 95.0);

        let shrunk = bbox.shrink(5.0);
        assert_eq!(shrunk.width(), 70.0);
        assert_eq!(shrunk.height(), 70.0);
    }

    #[test]
    fn test_bbox_gaps() {
        let box1 = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let box2 = BoundingBox::new(0.0, 70.0, 50.0, 120.0);
        let box3 = BoundingBox::new(70.0, 0.0, 120.0, 50.0);

        assert_eq!(box1.vertical_gap(&box2), 20.0);
        assert_eq!(box1.horizontal_gap(&box3), 20.0);
    }

    #[test]
    fn test_bbox_halves() {
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        let left = bbox.left_half(50.0);
        assert_eq!(left.width(), 50.0);
        assert_eq!(left.x1, 0.0);
        assert_eq!(left.x2, 50.0);

        let right = bbox.right_half(50.0);
        assert_eq!(right.width(), 50.0);
        assert_eq!(right.x1, 50.0);
        assert_eq!(right.x2, 100.0);
    }

    #[test]
    fn test_bbox_serialization() {
        let bbox = BoundingBox::new(10.0, 20.0, 30.0, 40.0);
        let json = serde_json::to_string(&bbox).unwrap();
        let parsed: BoundingBox = serde_json::from_str(&json).unwrap();
        assert_eq!(bbox, parsed);
    }

    // Additional geometry tests for Phase 4.1

    #[test]
    fn test_point_manhattan_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert_eq!(p1.manhattan_distance(&p2), 7.0);
    }

    #[test]
    fn test_point_default() {
        let p = Point::default();
        assert_eq!(p.x, 0.0);
        assert_eq!(p.y, 0.0);
    }

    #[test]
    fn test_bbox_from_xywh() {
        let bbox = BoundingBox::from_xywh(10.0, 20.0, 50.0, 30.0);
        assert_eq!(bbox.x1, 10.0);
        assert_eq!(bbox.y1, 20.0);
        assert_eq!(bbox.x2, 60.0);
        assert_eq!(bbox.y2, 50.0);
        assert_eq!(bbox.width(), 50.0);
        assert_eq!(bbox.height(), 30.0);
    }

    #[test]
    fn test_bbox_from_center() {
        let center = Point::new(50.0, 50.0);
        let bbox = BoundingBox::from_center(center, 40.0, 20.0);
        assert_eq!(bbox.x1, 30.0);
        assert_eq!(bbox.y1, 40.0);
        assert_eq!(bbox.x2, 70.0);
        assert_eq!(bbox.y2, 60.0);
    }

    #[test]
    fn test_bbox_union_all() {
        let boxes = vec![
            BoundingBox::new(0.0, 0.0, 10.0, 10.0),
            BoundingBox::new(20.0, 20.0, 30.0, 30.0),
            BoundingBox::new(50.0, 50.0, 60.0, 60.0),
        ];
        let union = BoundingBox::union_all(&boxes).unwrap();
        assert_eq!(union.x1, 0.0);
        assert_eq!(union.y1, 0.0);
        assert_eq!(union.x2, 60.0);
        assert_eq!(union.y2, 60.0);
    }

    #[test]
    fn test_bbox_union_all_empty() {
        let boxes: Vec<BoundingBox> = vec![];
        assert!(BoundingBox::union_all(&boxes).is_none());
    }

    #[test]
    fn test_bbox_corners() {
        let bbox = BoundingBox::new(10.0, 20.0, 30.0, 40.0);

        let tl = bbox.top_left();
        assert_eq!(tl.x, 10.0);
        assert_eq!(tl.y, 20.0);

        let tr = bbox.top_right();
        assert_eq!(tr.x, 30.0);
        assert_eq!(tr.y, 20.0);

        let bl = bbox.bottom_left();
        assert_eq!(bl.x, 10.0);
        assert_eq!(bl.y, 40.0);

        let br = bbox.bottom_right();
        assert_eq!(br.x, 30.0);
        assert_eq!(br.y, 40.0);
    }

    #[test]
    fn test_bbox_contains_point() {
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        assert!(bbox.contains_point(&Point::new(50.0, 50.0))); // Inside
        assert!(bbox.contains_point(&Point::new(0.0, 0.0))); // Corner
        assert!(bbox.contains_point(&Point::new(100.0, 100.0))); // Corner
        assert!(!bbox.contains_point(&Point::new(-1.0, 50.0))); // Outside left
        assert!(!bbox.contains_point(&Point::new(101.0, 50.0))); // Outside right
    }

    #[test]
    fn test_bbox_intersection_box() {
        let box1 = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let box2 = BoundingBox::new(50.0, 50.0, 150.0, 150.0);

        let intersection = box1.intersection(&box2).unwrap();
        assert_eq!(intersection.x1, 50.0);
        assert_eq!(intersection.y1, 50.0);
        assert_eq!(intersection.x2, 100.0);
        assert_eq!(intersection.y2, 100.0);
    }

    #[test]
    fn test_bbox_intersection_none() {
        let box1 = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let box2 = BoundingBox::new(100.0, 100.0, 150.0, 150.0);

        assert!(box1.intersection(&box2).is_none());
    }

    #[test]
    fn test_bbox_intersection_area() {
        let box1 = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let box2 = BoundingBox::new(50.0, 50.0, 150.0, 150.0);

        let area = box1.intersection_area(&box2);
        assert_eq!(area, 2500.0); // 50 x 50
    }

    #[test]
    fn test_bbox_is_horizontally_aligned() {
        let box1 = BoundingBox::new(0.0, 0.0, 50.0, 20.0);
        let box2 = BoundingBox::new(60.0, 0.0, 110.0, 20.0); // Same Y
        let box3 = BoundingBox::new(60.0, 100.0, 110.0, 120.0); // Different Y

        assert!(box1.is_horizontally_aligned(&box2, 0.1));
        assert!(!box1.is_horizontally_aligned(&box3, 0.1));
    }

    #[test]
    fn test_bbox_is_vertically_aligned() {
        let box1 = BoundingBox::new(0.0, 0.0, 20.0, 50.0);
        let box2 = BoundingBox::new(0.0, 60.0, 20.0, 110.0); // Same X
        let box3 = BoundingBox::new(100.0, 60.0, 120.0, 110.0); // Different X

        assert!(box1.is_vertically_aligned(&box2, 0.1));
        assert!(!box1.is_vertically_aligned(&box3, 0.1));
    }

    #[test]
    fn test_bbox_to_polygon() {
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let polygon = bbox.to_polygon();

        assert_eq!(polygon.len(), 4);
        assert_eq!(polygon[0], Point::new(0.0, 0.0)); // top-left
        assert_eq!(polygon[1], Point::new(100.0, 0.0)); // top-right
        assert_eq!(polygon[2], Point::new(100.0, 100.0)); // bottom-right
        assert_eq!(polygon[3], Point::new(0.0, 100.0)); // bottom-left
    }

    #[test]
    fn test_bbox_top_bottom_halves() {
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);

        let top = bbox.top_half(50.0);
        assert_eq!(top.y1, 0.0);
        assert_eq!(top.y2, 50.0);
        assert_eq!(top.height(), 50.0);

        let bottom = bbox.bottom_half(50.0);
        assert_eq!(bottom.y1, 50.0);
        assert_eq!(bottom.y2, 100.0);
        assert_eq!(bottom.height(), 50.0);
    }

    #[test]
    fn test_bbox_default() {
        let bbox = BoundingBox::default();
        assert_eq!(bbox.x1, 0.0);
        assert_eq!(bbox.y1, 0.0);
        assert_eq!(bbox.x2, 0.0);
        assert_eq!(bbox.y2, 0.0);
        assert_eq!(bbox.area(), 0.0);
    }

    #[test]
    fn test_bbox_iou_no_overlap() {
        let box1 = BoundingBox::new(0.0, 0.0, 10.0, 10.0);
        let box2 = BoundingBox::new(100.0, 100.0, 110.0, 110.0);
        assert_eq!(box1.iou(&box2), 0.0);
    }

    #[test]
    fn test_point_serialization() {
        let point = Point::new(10.5, 20.5);
        let json = serde_json::to_string(&point).unwrap();
        let parsed: Point = serde_json::from_str(&json).unwrap();
        assert_eq!(point, parsed);
    }
}
