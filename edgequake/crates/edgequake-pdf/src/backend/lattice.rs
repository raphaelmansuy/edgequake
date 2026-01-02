use super::elements::{PdfLine, TextElement};
use crate::layout::dbscan_1d;
use crate::schema::{Block, BlockType, BoundingBox};

/// Lattice Engine for table extraction based on graphical lines
pub struct LatticeEngine {
    // Configuration parameters
    min_line_length: f32,
    line_tolerance: f32,
}

impl LatticeEngine {
    pub fn new() -> Self {
        Self {
            min_line_length: 10.0,
            line_tolerance: 2.0,
        }
    }

    /// Detect tables using lattice method (graphical lines)
    pub fn detect_tables(
        &self,
        lines: &[PdfLine],
        text_elements: &[TextElement],
        _page_width: f32,
        _page_height: f32,
    ) -> Vec<Block> {
        // 1. Filter relevant lines (horizontal and vertical)
        let (h_lines, v_lines) = self.filter_lines(lines);
        let all_lines: Vec<&PdfLine> = h_lines.iter().chain(v_lines.iter()).collect();

        if all_lines.is_empty() {
            return Vec::new();
        }

        // 2. Build adjacency list (graph)
        let mut adj = vec![Vec::new(); all_lines.len()];
        for i in 0..all_lines.len() {
            for j in i + 1..all_lines.len() {
                if self.lines_intersect(all_lines[i], all_lines[j]) {
                    adj[i].push(j);
                    adj[j].push(i);
                }
            }
        }

        // 3. Find connected components
        let mut visited = vec![false; all_lines.len()];
        let mut tables = Vec::new();

        for i in 0..all_lines.len() {
            if !visited[i] {
                let mut component = Vec::new();
                let mut stack = vec![i];
                visited[i] = true;

                while let Some(curr) = stack.pop() {
                    component.push(all_lines[curr]);
                    for &neighbor in &adj[curr] {
                        if !visited[neighbor] {
                            visited[neighbor] = true;
                            stack.push(neighbor);
                        }
                    }
                }

                // 4. Create table block from component
                // Minimum lines for a connected component table: 4 (a box)
                if component.len() >= 4 {
                    if let Some(table_block) = self.create_table_block(component, text_elements) {
                        tables.push(table_block);
                    }
                }
            }
        }

        // 5. Detect Parallel Line Tables (for tables without vertical lines)
        // Collect lines that were NOT part of any large connected component
        // We check if a horizontal line is inside an already detected table bbox.

        let mut unused_h_lines = Vec::new();
        for line in &h_lines {
            let mut used = false;
            for table in &tables {
                // Check if line is inside table bbox
                // Use center point
                let cx = (line.p1.0 + line.p2.0) / 2.0;
                let cy = (line.p1.1 + line.p2.1) / 2.0;
                if table
                    .bbox
                    .contains_point(&crate::schema::Point::new(cx, cy))
                {
                    used = true;
                    break;
                }
            }
            if !used {
                unused_h_lines.push(line);
            }
        }

        // Group parallel lines
        let groups = self.group_parallel_lines(&unused_h_lines);

        for group in groups {
            // Minimum 2 parallel lines to form a table (top and bottom, or header and bottom)
            if group.len() >= 2 {
                if let Some(block) = self.create_table_block(group, text_elements) {
                    tables.push(block);
                }
            }
        }

        tables
    }

    fn group_parallel_lines<'a>(&self, lines: &[&'a PdfLine]) -> Vec<Vec<&'a PdfLine>> {
        if lines.is_empty() {
            return Vec::new();
        }

        // Sort by Y (descending)
        let mut sorted_lines = lines.to_vec();
        sorted_lines.sort_by(|a, b| b.p1.1.partial_cmp(&a.p1.1).unwrap());

        let mut groups = Vec::new();
        let mut current_group: Vec<&PdfLine> = Vec::new();

        for line in sorted_lines {
            if current_group.is_empty() {
                current_group.push(line);
            } else {
                let last = current_group.last().unwrap();

                // Check horizontal overlap
                let l1_min_x = line.p1.0.min(line.p2.0);
                let l1_max_x = line.p1.0.max(line.p2.0);
                let l2_min_x = last.p1.0.min(last.p2.0);
                let l2_max_x = last.p1.0.max(last.p2.0);

                let overlap_min = l1_min_x.max(l2_min_x);
                let overlap_max = l1_max_x.min(l2_max_x);

                let overlap_len = (overlap_max - overlap_min).max(0.0);
                let l1_len = l1_max_x - l1_min_x;
                let l2_len = l2_max_x - l2_min_x;

                // If overlap is significant (> 50% of the shorter line)
                let min_len = l1_len.min(l2_len);
                if min_len > 0.0 && overlap_len / min_len > 0.5 {
                    // Also check vertical distance?
                    // If lines are too far apart (e.g. > 500 points), maybe not same table?
                    // But a table can be long.
                    // Let's assume if they overlap horizontally and are sequential in Y, they might be related.
                    current_group.push(line);
                } else {
                    // Start new group
                    groups.push(current_group);
                    current_group = vec![line];
                }
            }
        }
        if !current_group.is_empty() {
            groups.push(current_group);
        }

        groups
    }

    fn filter_lines(&self, lines: &[PdfLine]) -> (Vec<PdfLine>, Vec<PdfLine>) {
        let mut h_lines = Vec::new();
        let mut v_lines = Vec::new();

        for line in lines {
            let dx = (line.p2.0 - line.p1.0).abs();
            let dy = (line.p2.1 - line.p1.1).abs();

            if dx > self.min_line_length && dy < self.line_tolerance {
                h_lines.push(line.clone());
            } else if dy > self.min_line_length && dx < self.line_tolerance {
                v_lines.push(line.clone());
            }
        }

        (h_lines, v_lines)
    }

    fn lines_intersect(&self, l1: &PdfLine, l2: &PdfLine) -> bool {
        // Simple AABB check first
        let l1_min_x = l1.p1.0.min(l1.p2.0) - self.line_tolerance;
        let l1_max_x = l1.p1.0.max(l1.p2.0) + self.line_tolerance;
        let l1_min_y = l1.p1.1.min(l1.p2.1) - self.line_tolerance;
        let l1_max_y = l1.p1.1.max(l1.p2.1) + self.line_tolerance;

        let l2_min_x = l2.p1.0.min(l2.p2.0) - self.line_tolerance;
        let l2_max_x = l2.p1.0.max(l2.p2.0) + self.line_tolerance;
        let l2_min_y = l2.p1.1.min(l2.p2.1) - self.line_tolerance;
        let l2_max_y = l2.p1.1.max(l2.p2.1) + self.line_tolerance;

        if l1_max_x < l2_min_x || l1_min_x > l2_max_x || l1_max_y < l2_min_y || l1_min_y > l2_max_y
        {
            return false;
        }

        true
    }

    fn create_table_block(
        &self,
        lines: Vec<&PdfLine>,
        text_elements: &[TextElement],
    ) -> Option<Block> {
        // Calculate bounding box
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for line in lines.iter() {
            min_x = min_x.min(line.p1.0).min(line.p2.0);
            max_x = max_x.max(line.p1.0).max(line.p2.0);
            min_y = min_y.min(line.p1.1).min(line.p2.1);
            max_y = max_y.max(line.p1.1).max(line.p2.1);
        }

        if min_x.is_infinite() {
            return None;
        }

        let bbox = BoundingBox::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y);

        // 1. Identify Grid Lines
        let mut h_lines = Vec::new();
        let mut v_lines = Vec::new();

        for line in &lines {
            let dx = (line.p2.0 - line.p1.0).abs();
            let dy = (line.p2.1 - line.p1.1).abs();

            if dx > self.min_line_length && dy < self.line_tolerance {
                h_lines.push(*line);
            } else if dy > self.min_line_length && dx < self.line_tolerance {
                v_lines.push(*line);
            }
        }

        // Collect Y coordinates from H lines (average of p1.y and p2.y)
        let y_coords: Vec<f32> = h_lines.iter().map(|l| (l.p1.1 + l.p2.1) / 2.0).collect();
        // Collect X coordinates from V lines (average of p1.x and p2.x)
        let x_coords: Vec<f32> = v_lines.iter().map(|l| (l.p1.0 + l.p2.0) / 2.0).collect();

        // Sort and deduplicate
        let unique_y = self.get_sorted_unique(&y_coords, true); // Descending (Top to Bottom)
        let mut unique_x = self.get_sorted_unique(&x_coords, false); // Ascending (Left to Right)

        // If we have rows (unique_y >= 2) but no columns (unique_x < 2), use geometric clustering
        // This handles tables without vertical lines but with clear column structure
        if unique_y.len() >= 2 && unique_x.len() < 2 {
            let detected_x = self.detect_columns_by_clustering(text_elements, &bbox);
            if detected_x.len() >= 2 {
                unique_x = detected_x;
                tracing::info!("Detected {} columns in table bbox {:?}", unique_x.len(), bbox);
            }
        }

        // If we don't have a grid, fallback to raw text dump
        if unique_y.len() < 2 || unique_x.len() < 2 {
            return self.create_fallback_table_block(bbox, text_elements);
        }

        // 2. Build Cells and Extract Text
        let mut rows = Vec::new();
        for i in 0..unique_y.len() - 1 {
            let top = unique_y[i];
            let bottom = unique_y[i + 1];
            let mut row_cells = Vec::new();

            for j in 0..unique_x.len() - 1 {
                let left = unique_x[j];
                let right = unique_x[j + 1];

                // Find text in this cell
                // Note: PDF Y increases upwards. So 'top' has higher Y than 'bottom'.
                // Rect is [left, bottom, right, top]
                let cell_text = self.extract_text_in_rect(text_elements, left, bottom, right, top);
                row_cells.push(cell_text);
            }
            rows.push(row_cells);
        }

        // 3. Format as Markdown Table
        let mut markdown = String::new();

        if !rows.is_empty() {
            // Header row
            markdown.push_str("| ");
            markdown.push_str(&rows[0].join(" | "));
            markdown.push_str(" |\n");

            // Separator row
            markdown.push_str("|");
            for _ in 0..rows[0].len() {
                markdown.push_str("---|");
            }
            markdown.push('\n');

            // Data rows
            for row in rows.iter().skip(1) {
                markdown.push_str("| ");
                markdown.push_str(&row.join(" | "));
                markdown.push_str(" |\n");
            }
        }

        let mut block = Block::new(BlockType::Table, bbox);
        block.text = markdown;
        Some(block)
    }

    /// Detect table columns using geometric clustering (DBSCAN).
    ///
    /// First principles approach: cluster text element X-coordinates to find
    /// natural column boundaries instead of using whitespace heuristics.
    ///
    /// This handles multi-word cells, variable spacing, and alignment issues.
    fn detect_columns_by_clustering(
        &self,
        text_elements: &[TextElement],
        bbox: &BoundingBox,
    ) -> Vec<f32> {
        // 1. Collect X coordinates of elements within table bbox
        let mut x_coords: Vec<f32> = text_elements
            .iter()
            .filter(|elem| {
                let cx = elem.x;
                let cy = elem.y;
                let tolerance = 2.0;
                cx >= bbox.x1 - tolerance
                    && cx <= bbox.x2 + tolerance
                    && cy >= bbox.y1 - tolerance
                    && cy <= bbox.y2 + tolerance
            })
            .map(|elem| elem.x)
            .collect();

        if x_coords.len() < 2 {
            return Vec::new();
        }

        // 2. Sort coordinates
        x_coords.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // 3. Compute adaptive epsilon (10th percentile of inter-element distances)
        let mut distances: Vec<f32> = x_coords
            .windows(2)
            .map(|w| w[1] - w[0])
            .filter(|&d| d > 0.5) // Ignore sub-point distances
            .collect();

        if distances.is_empty() {
            return Vec::new();
        }

        distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p10_idx = ((distances.len() as f32 * 0.10).ceil() as usize).min(distances.len() - 1);
        let epsilon = distances[p10_idx];

        // 4. Apply DBSCAN clustering
        let clusters: Vec<Vec<f32>> = dbscan_1d(&x_coords, epsilon, 1);

        // 5. Extract column boundaries (cluster centroids)
        let mut col_boundaries: Vec<f32> = Vec::new();
        for cluster in &clusters {
            if cluster.is_empty() {
                continue;
            }
            let sum: f32 = cluster.iter().sum();
            let centroid = sum / cluster.len() as f32;
            col_boundaries.push(centroid);
        }

        // 6. Sort left to right
        col_boundaries.sort_by(|a, b| a.partial_cmp(b).unwrap());

        col_boundaries
    }

    fn detect_columns_by_whitespace(
        &self,
        text_elements: &[TextElement],
        bbox: &BoundingBox,
    ) -> Vec<f32> {
        // Filter elements inside bbox
        let elements: Vec<&TextElement> = text_elements
            .iter()
            .filter(|elem| {
                let elem_cx = elem.x;
                let elem_cy = elem.y;
                let tolerance = 2.0;
                elem_cx >= bbox.x1 - tolerance
                    && elem_cx <= bbox.x1 + bbox.width() + tolerance
                    && elem_cy >= bbox.y1 - tolerance
                    && elem_cy <= bbox.y1 + bbox.height() + tolerance
            })
            .collect();

        if elements.is_empty() {
            return Vec::new();
        }

        // 1. Create X-projection
        // Use a resolution of 1.0 point
        let width = bbox.width().ceil() as usize;
        if width == 0 {
            return Vec::new();
        }

        let start_x = bbox.x1.floor() as usize;
        let mut projection = vec![0; width + 1];

        for elem in &elements {
            // Estimate element width: text length * font_size * 0.5 (approx char width)
            let elem_w = elem.text.len() as f32 * elem.font_size * 0.5;
            let elem_x = elem.x;

            // Assume x is left coordinate
            let start = (elem_x - start_x as f32).max(0.0) as usize;
            let end = (elem_x + elem_w - start_x as f32).min(width as f32) as usize;

            for i in start..end {
                if i < projection.len() {
                    projection[i] = 1;
                }
            }
        }

        // 2. Find gaps
        let mut column_boundaries = Vec::new();
        column_boundaries.push(bbox.x1); // Left edge

        let mut in_gap = false;
        let mut gap_start = 0;
        let min_gap_width = 5; // Reduced from 10 to 5 points

        for i in 0..width {
            if projection[i] == 0 {
                if !in_gap {
                    in_gap = true;
                    gap_start = i;
                }
            } else {
                if in_gap {
                    in_gap = false;
                    if i - gap_start >= min_gap_width {
                        // Found a gap. The column boundary is the middle of the gap.
                        let boundary_x = start_x as f32 + (gap_start + i) as f32 / 2.0;
                        column_boundaries.push(boundary_x);
                    }
                }
            }
        }

        // Check last gap
        if in_gap && width - gap_start >= min_gap_width {
            // Ignore trailing gap as it's just the right margin
        }

        column_boundaries.push(bbox.x1 + bbox.width()); // Right edge

        // Debug print
        if column_boundaries.len() >= 3 {
            println!(
                "Detected {} columns in table bbox {:?}",
                column_boundaries.len() - 1,
                bbox
            );
        } else {
        }

        column_boundaries
    }

    fn get_sorted_unique(&self, values: &[f32], descending: bool) -> Vec<f32> {
        let mut coords = values.to_vec();
        coords.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut unique = Vec::new();
        if coords.is_empty() {
            return unique;
        }

        unique.push(coords[0]);
        for &val in &coords[1..] {
            if (val - unique.last().unwrap()).abs() > self.line_tolerance {
                unique.push(val);
            }
        }

        if descending {
            unique.reverse();
        }
        unique
    }

    fn extract_text_in_rect(
        &self,
        text_elements: &[TextElement],
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
    ) -> String {
        let mut contained: Vec<&TextElement> = text_elements
            .iter()
            .filter(|elem| {
                let cx = elem.x;
                let cy = elem.y;
                // Use a small tolerance
                let tol = 2.0;
                cx >= min_x - tol && cx <= max_x + tol && cy >= min_y - tol && cy <= max_y + tol
            })
            .collect();

        // Sort by Y (descending) then X (ascending)
        contained.sort_by(|a, b| {
            let row_a = (a.y / 5.0).round() as i32;
            let row_b = (b.y / 5.0).round() as i32;
            if row_a != row_b {
                row_b.cmp(&row_a)
            } else {
                a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        let mut text = String::new();
        for (i, elem) in contained.iter().enumerate() {
            if i > 0 {
                text.push(' ');
            }
            text.push_str(&elem.text);
        }
        text
    }

    fn create_fallback_table_block(
        &self,
        bbox: BoundingBox,
        text_elements: &[TextElement],
    ) -> Option<Block> {
        let mut table_text = String::new();

        let mut contained_elements: Vec<&TextElement> = text_elements
            .iter()
            .filter(|elem| {
                let elem_cx = elem.x;
                let elem_cy = elem.y;
                let tolerance = 5.0;
                elem_cx >= bbox.x1 - tolerance
                    && elem_cx <= bbox.x1 + bbox.width() + tolerance
                    && elem_cy >= bbox.y1 - tolerance
                    && elem_cy <= bbox.y1 + bbox.height() + tolerance
            })
            .collect();

        contained_elements.sort_by(|a, b| {
            let row_a = (a.y / 5.0).round() as i32;
            let row_b = (b.y / 5.0).round() as i32;
            if row_a != row_b {
                row_b.cmp(&row_a)
            } else {
                a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        for elem in contained_elements {
            table_text.push_str(&elem.text);
            table_text.push(' ');
        }

        let mut block = Block::new(BlockType::Table, bbox);
        block.text = table_text.trim().to_string();
        Some(block)
    }
}
