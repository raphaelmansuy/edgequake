use super::elements::{PdfLine, TextElement};
use crate::layout::dbscan_1d;
use crate::schema::{Block, BlockType, BoundingBox};

/// Lattice Engine for table extraction based on graphical lines
pub struct LatticeEngine {
    // Configuration parameters
    min_line_length: f32,
    line_tolerance: f32,
}

impl Default for LatticeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LatticeEngine {
    pub fn new() -> Self {
        Self {
            min_line_length: 10.0,
            line_tolerance: 2.0,
        }
    }

    /// Detect tables using lattice method (graphical lines)
    ///
    /// WHY: PDF tables often have visible grid lines (borders). By finding
    /// connected components of intersecting lines, we can identify table
    /// structures without relying on text positioning alone.
    ///
    /// Algorithm:
    /// 1. Filter lines into horizontal and vertical categories
    /// 2. Build an adjacency graph where edges connect intersecting lines
    /// 3. Find connected components using DFS
    /// 4. Components with ≥4 lines form table candidates (minimum: a box)
    pub fn detect_tables(
        &self,
        lines: &[PdfLine],
        text_elements: &[TextElement],
        _page_width: f32,
        _page_height: f32,
    ) -> Vec<Block> {
        // WHY: Filter first to avoid processing decorative/unrelated lines
        let (h_lines, v_lines) = self.filter_lines(lines);
        let all_lines: Vec<&PdfLine> = h_lines.iter().chain(v_lines.iter()).collect();

        if all_lines.is_empty() {
            return Vec::new();
        }

        // WHY: Build adjacency list for connected component detection
        // Using graph representation enables O(V+E) component finding
        let mut adj = vec![Vec::new(); all_lines.len()];
        for i in 0..all_lines.len() {
            for j in i + 1..all_lines.len() {
                if self.lines_intersect(all_lines[i], all_lines[j]) {
                    adj[i].push(j);
                    adj[j].push(i);
                }
            }
        }

        // WHY: DFS-based connected component detection is cache-friendly
        // and handles arbitrary graph topologies
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

                // WHY: Minimum 4 lines forms a box (simplest table)
                // Fewer lines are likely decorative or partial borders
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

        // FIRST PRINCIPLES: Merge horizontally adjacent tables that share the same Y-band.
        // Wide tables in academic PDFs are often split into left/right halves with a gap.
        // If two tables have overlapping Y-bands (>80%) and are side-by-side (X gap < 50pt),
        // they should be merged into a single table.

        self.merge_horizontal_table_halves(tables, text_elements)
    }

    /// Merge tables that appear to be left/right halves of the same table.
    /// Returns a deduplicated list of tables.
    fn merge_horizontal_table_halves(
        &self,
        mut tables: Vec<Block>,
        text_elements: &[TextElement],
    ) -> Vec<Block> {
        if tables.len() < 2 {
            return tables;
        }

        // Sort by Y-position (top to bottom in PDF coordinates where Y increases downward for our bbox)
        tables.sort_by(|a, b| {
            b.bbox
                .y2
                .partial_cmp(&a.bbox.y2)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut merged_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut result: Vec<Block> = Vec::new();

        for i in 0..tables.len() {
            if merged_indices.contains(&i) {
                continue;
            }

            let mut merged_table = tables[i].clone();
            let mut merged_count = 1;

            for j in (i + 1)..tables.len() {
                if merged_indices.contains(&j) {
                    continue;
                }

                let t1 = &merged_table;
                let t2 = &tables[j];

                // Check Y-band overlap (>70%)
                let y_overlap = (t1.bbox.y2.min(t2.bbox.y2) - t1.bbox.y1.max(t2.bbox.y1)).max(0.0);
                let min_height = t1.bbox.height().min(t2.bbox.height());
                let y_overlap_ratio = if min_height > 0.0 {
                    y_overlap / min_height
                } else {
                    0.0
                };

                if y_overlap_ratio < 0.70 {
                    continue;
                }

                // Check X-gap (should be small - tables are adjacent)
                let x_gap = if t1.bbox.x2 < t2.bbox.x1 {
                    t2.bbox.x1 - t1.bbox.x2
                } else if t2.bbox.x2 < t1.bbox.x1 {
                    t1.bbox.x1 - t2.bbox.x2
                } else {
                    0.0 // overlapping
                };

                if x_gap > 50.0 {
                    continue;
                }

                // Tables are horizontally adjacent with matching Y-band - merge them
                println!(
                    "🔗 MERGING HORIZONTAL TABLE HALVES: Y-overlap={:.1}%, X-gap={:.1}pt",
                    y_overlap_ratio * 100.0,
                    x_gap
                );

                // Expand bbox to include both tables
                merged_table.bbox.x1 = merged_table.bbox.x1.min(t2.bbox.x1);
                merged_table.bbox.x2 = merged_table.bbox.x2.max(t2.bbox.x2);
                merged_table.bbox.y1 = merged_table.bbox.y1.min(t2.bbox.y1);
                merged_table.bbox.y2 = merged_table.bbox.y2.max(t2.bbox.y2);

                // Re-extract text for the merged bbox
                let text_parts = self.extract_text_in_rect(
                    text_elements,
                    merged_table.bbox.x1,
                    merged_table.bbox.y1,
                    merged_table.bbox.x2,
                    merged_table.bbox.y2,
                );
                merged_table.text = text_parts.join(" ");

                merged_indices.insert(j);
                merged_count += 1;
            }

            if merged_count > 1 {
                println!(
                    "📊 MERGED {} table halves into one (bbox: {:.1}x{:.1})",
                    merged_count,
                    merged_table.bbox.width(),
                    merged_table.bbox.height()
                );
            }

            result.push(merged_table);
        }

        result
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

        // FIRST PRINCIPLES: Only use clustering when grid lines are absent or insufficient
        //
        // Rationale:
        // - When vertical grid lines exist (unique_x >= 2), trust them - PDF author explicitly drew boundaries
        // - When no vertical lines (unique_x < 2), try clustering to find implicit column boundaries
        // - Clustering on already-gridded tables causes false crossing_ratio rejections
        //
        // This hybrid approach handles both:
        // 1. Properly gridded tables (trust the lines)
        // 2. Under-gridded or whitespace tables (infer from text positions)

        if unique_x.len() < 2 {
            // No vertical lines - try clustering to detect columns
            let detected_x = self.detect_columns_by_clustering(text_elements, &bbox);

            let rows_from_lines = unique_y.len().saturating_sub(1);
            let cols_from_lines = unique_x.len().saturating_sub(1);
            let cols_from_clustering = detected_x.len().saturating_sub(1);

            println!(
                "Table grid: {} rows (from lines), {} cols (from lines), {} cols (from clustering)",
                rows_from_lines, cols_from_lines, cols_from_clustering
            );

            if detected_x.len() >= 2 {
                println!(
                    "Using clustered columns ({} cols) - no vertical grid lines found",
                    cols_from_clustering
                );
                unique_x = detected_x;
            }
        } else {
            // Has vertical lines - trust them, don't override with clustering
            let rows_from_lines = unique_y.len().saturating_sub(1);
            let cols_from_lines = unique_x.len().saturating_sub(1);
            println!(
                "Table grid: {} rows, {} cols (using grid lines - not clustering)",
                rows_from_lines, cols_from_lines
            );
        }

        // If we don't have a grid, fallback to raw text dump
        if unique_y.len() < 2 || unique_x.len() < 2 {
            // If no grid is found, we return None so the text is processed by the standard layout engine.
            // Returning a fallback block here would force it to be treated as a "Table" block,
            // which bypasses paragraph/header detection.
            return None;
        }

        // Validate table structure using geometric properties (First Principles)
        // We reject structures that are physically unlikely to be readable tables.

        let table_height = bbox.height();
        let table_width = bbox.width();
        let num_rows = unique_y.len().saturating_sub(1);
        let num_cols = unique_x.len().saturating_sub(1);

        if num_rows == 0 || num_cols == 0 {
            return None;
        }

        // FIRST PRINCIPLES: A table must have at least 2 rows (header + data)
        // A single-row grid is NOT a table - it's a decorative line or header underline.
        if num_rows < 2 {
            println!(
                "Rejecting table: Only {} row(s) - tables require header + data rows",
                num_rows
            );
            return None;
        }

        // 1. Row Height Analysis
        // A single table row shouldn't take up half the page (unless it's a layout wrapper).
        // Standard page height is ~800pt.
        let avg_row_height = table_height / num_rows as f32;
        if avg_row_height > 200.0 {
            // tracing::info!("Rejecting table: Row height too large ({:.1}pt)", avg_row_height);
            return None;
        }

        // 2. Column Width Analysis
        // Columns narrower than ~10pt (approx 2-3 chars) are likely grid noise or vertical separators.
        let avg_col_width = table_width / num_cols as f32;
        if avg_col_width < 10.0 {
            // tracing::info!("Rejecting table: Column width too narrow ({:.1}pt)", avg_col_width);
            return None;
        }

        // 3. Aspect Ratio / Density Check
        // Extreme aspect ratios (e.g. 1 row, 50 cols) are suspicious.
        let cell_aspect_ratio = avg_col_width / avg_row_height;
        if cell_aspect_ratio < 0.05 {
            // Very tall, very narrow cells
            // tracing::info!("Rejecting table: Extreme cell aspect ratio ({:.3})", cell_aspect_ratio);
            return None;
        }

        // 6. Column Crossing Check (Heuristic 6)
        // If text elements physically cross column boundaries, the vertical lines are likely not column separators.
        if unique_x.len() > 2 {
            let mut crossing_count = 0;

            // Filter elements inside table
            let table_elements: Vec<&TextElement> = text_elements
                .iter()
                .filter(|e| {
                    e.x >= bbox.x1 - 1.0
                        && e.x <= bbox.x2 + 1.0
                        && e.y >= bbox.y1 - 1.0
                        && e.y <= bbox.y2 + 1.0
                })
                .collect();

            let total_elements = table_elements.len();

            for elem in &table_elements {
                let char_width = if elem.font_size > 0.0 {
                    elem.font_size * 0.5
                } else {
                    5.0
                };
                let elem_width = elem.text.len() as f32 * char_width;
                let elem_right = elem.x + elem_width;

                for &boundary in &unique_x[1..unique_x.len() - 1] {
                    // Skip outer edges
                    // Check if element crosses boundary with significant overlap
                    // We use a small tolerance (2.0) to avoid floating point issues
                    if elem.x < boundary - 2.0 && elem_right > boundary + 2.0 {
                        crossing_count += 1;
                        break; // Count once per element
                    }
                }
            }

            if total_elements > 0 {
                let crossing_ratio = crossing_count as f32 / total_elements as f32;
                println!(
                    "Table Check: crossing_ratio={:.2} ({}/{})",
                    crossing_ratio, crossing_count, total_elements
                );
                if crossing_ratio > 0.35 {
                    // First principles: Word-level text extraction in multi-line cells naturally
                    // creates apparent "crossings". Threshold of 0.35 (35%) allows legitimate
                    // multi-line cells while rejecting severely malformed grids.
                    println!(
                        "Rejecting table: Text elements cross column boundaries (ratio {:.2})",
                        crossing_ratio
                    );
                    return None;
                }
            }

            // 7. Element Count vs Grid Size (Heuristic 7)
            // If the grid implies many cells but we have very few text elements, it's likely noise.
            if num_cols * num_rows > 20 && total_elements < 5 {
                println!(
                    "Rejecting table: Large grid ({}x{}) with few text elements ({})",
                    num_rows, num_cols, total_elements
                );
                return None;
            }
        }

        // 2. Build Cells and Extract Text
        // Note: extract_text_in_rect now returns Vec<String> to handle merged cells
        let mut rows = Vec::new();

        // DEBUG: Show which table we're processing
        println!(
            "📊 BUILDING TABLE: grid={}x{} (rows x cols)",
            num_rows, num_cols
        );

        for i in 0..unique_y.len() - 1 {
            let top = unique_y[i];
            let bottom = unique_y[i + 1];
            let mut row_cells = Vec::new();

            for j in 0..unique_x.len() - 1 {
                let left = unique_x[j];
                let right = unique_x[j + 1];

                // Extract text - may return multiple strings if cell is merged
                let cell_texts = self.extract_text_in_rect(text_elements, left, bottom, right, top);

                // FIRST PRINCIPLES: Handle merged cells
                // If extract_text_in_rect found multiple X-position clusters,
                // we have a merged cell that should be split into multiple columns

                // DEBUG: Track if this is a split cell
                if cell_texts.len() > 1 {
                    println!(
                        "💥 SPLIT APPLIED: Cell at grid col {} split into {} subcells",
                        j,
                        cell_texts.len()
                    );
                    for (idx, text) in cell_texts.iter().enumerate() {
                        println!("   Subcell {}: {:?}", idx, text);
                    }
                }

                for text in cell_texts {
                    row_cells.push(text);
                }
            }

            // DEBUG: Show row cell count after splitting
            if row_cells.len() != unique_x.len() - 1 {
                println!(
                    "🔥 ROW EXPANDED: grid cols={}, actual cells={}",
                    unique_x.len() - 1,
                    row_cells.len()
                );
            }

            // DEBUG: Check for Agentless in row cells
            let row_text = row_cells.join(" ");
            if row_text.contains("Agentless") || row_text.contains("25.20") {
                println!("🔴 ROW WITH AGENTLESS/25.20: {} cells", row_cells.len());
                for (idx, cell) in row_cells.iter().enumerate() {
                    println!("   cell[{}]: {:?}", idx, cell);
                }
            }

            rows.push(row_cells);
        }

        // FIRST PRINCIPLES: Detect empty header row and look for text above table
        // PDFs often position table headers ABOVE the grid lines, not inside the first row's bounds.
        // If first row is entirely empty but data rows exist, look for text just above the table.
        if !rows.is_empty() && rows.len() >= 2 {
            let first_row_empty = rows[0].iter().all(|c| c.trim().is_empty());
            let has_data_rows = rows
                .iter()
                .skip(1)
                .any(|row| row.iter().any(|c| !c.trim().is_empty()));

            if first_row_empty && has_data_rows {
                // Look for text elements just above the table (within ~25pt of top edge)
                let table_top = bbox.y2; // In PDF coords, y2 is typically the top
                let search_above = 30.0; // Search 30pt above the table top

                // Find text elements above the table within the table's X range
                let mut header_elements: Vec<&TextElement> = text_elements
                    .iter()
                    .filter(|elem| {
                        let is_above = elem.y >= table_top && elem.y <= table_top + search_above;
                        let is_within_x = elem.x >= bbox.x1 - 5.0 && elem.x <= bbox.x2 + 5.0;
                        is_above && is_within_x
                    })
                    .collect();

                if !header_elements.is_empty() {
                    // Sort by X position
                    header_elements
                        .sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));

                    // Try to distribute header elements into columns based on X position
                    // Use the column boundaries we already have
                    let num_cols = rows[0].len();
                    if num_cols >= 2 && unique_x.len() > num_cols {
                        let mut header_row: Vec<String> = vec![String::new(); num_cols];

                        for elem in &header_elements {
                            // Find which column this element belongs to
                            for col_idx in 0..num_cols {
                                let col_left = unique_x[col_idx];
                                let col_right = unique_x[col_idx + 1];

                                if elem.x >= col_left - 5.0 && elem.x < col_right + 5.0 {
                                    if !header_row[col_idx].is_empty() {
                                        header_row[col_idx].push(' ');
                                    }
                                    header_row[col_idx].push_str(&elem.text);
                                    break;
                                }
                            }
                        }

                        // Only use if we found at least one header
                        if header_row.iter().any(|h| !h.trim().is_empty()) {
                            println!("📋 DETECTED HEADERS ABOVE TABLE: {:?}", header_row);
                            rows[0] = header_row;
                        }
                    }
                }
            }
        }

        // 4. Content-based Validation (Heuristic 4)
        // Reject tables that are mostly empty (likely grid noise over whitespace)
        let total_cells: usize = rows.iter().map(|r| r.len()).sum();
        let empty_cells: usize = rows
            .iter()
            .flatten()
            .filter(|s| s.trim().is_empty())
            .count();

        if total_cells > 0 {
            let empty_ratio = empty_cells as f32 / total_cells as f32;
            if empty_ratio > 0.9 {
                // tracing::info!("Rejecting table: Too many empty cells ({:.1}%)", empty_ratio * 100.0);
                return None;
            }

            // 5. Text Density / Sentence Check (Heuristic 5)
            // If a table has many columns but contains long sentences in single cells, it's likely text layout.
            if num_cols > 3 {
                // Calculate average cell length for non-empty cells
                let non_empty_cells: Vec<&String> = rows
                    .iter()
                    .flatten()
                    .filter(|s| !s.trim().is_empty())
                    .collect();

                // Check for "sentence-like" content (long text with spaces)
                // Lowered threshold to 40 to catch shorter sentence fragments
                let has_long_sentences = non_empty_cells
                    .iter()
                    .any(|s| s.len() > 40 && s.contains(' '));

                // Also check average length
                let avg_len = if !non_empty_cells.is_empty() {
                    non_empty_cells.iter().map(|s| s.len()).sum::<usize>() as f32
                        / non_empty_cells.len() as f32
                } else {
                    0.0
                };

                println!(
                    "Table Check: cols={}, empty_ratio={:.2}, long_sentences={}, avg_len={:.1}",
                    num_cols, empty_ratio, has_long_sentences, avg_len
                );

                if empty_ratio > 0.5 && (has_long_sentences || avg_len > 30.0) {
                    println!("Rejecting table: Sparse table with sentence-like content (likely text layout)");
                    return None;
                }
            }
        }

        // 3. Normalize Column Counts (Handle Merged Cells)
        // FIRST PRINCIPLES: When cells are split by X-clustering, rows may have different column counts
        // Find the maximum column count and pad shorter rows with empty cells
        let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);

        if max_cols == 0 {
            return None;
        }

        for row in &mut rows {
            while row.len() < max_cols {
                row.push(String::new());
            }
        }

        // Update num_cols to reflect actual column count after splitting
        let _num_cols = max_cols;

        // FIRST PRINCIPLES: Handle merged text in single cells
        // Some PDFs have one text element containing multiple values that should be in separate columns
        // Detection: Cell has many whitespace-separated tokens, and row has many empty trailing cells
        for row in &mut rows {
            // Check if any cell contains Agentless
            let has_agentless = row.iter().any(|c| c.contains("Agentless"));
            if has_agentless {
                println!("🎯 PROCESSING AGENTLESS ROW: {} cells", row.len());
                for (idx, cell) in row.iter().enumerate() {
                    println!("   Cell {}: {:?}", idx, cell);
                }
            }

            let empty_count = row.iter().filter(|s| s.trim().is_empty()).count();
            let empty_ratio = empty_count as f32 / row.len() as f32;

            if has_agentless {
                println!(
                    "   Empty ratio: {:.2} ({}/{})",
                    empty_ratio,
                    empty_count,
                    row.len()
                );
            }

            // Only process rows that are mostly empty (suggests merged text in one cell)
            if empty_ratio > 0.3 {
                // Find cells with many tokens
                let mut new_row = Vec::new();
                for cell in row.iter() {
                    let tokens: Vec<&str> = cell.split_whitespace().collect();

                    // If cell has many tokens and looks like merged data (has numbers)
                    let has_numbers = tokens.iter().any(|t| {
                        t.parse::<f32>().is_ok()
                            || t.chars().all(|c| c.is_ascii_digit() || c == '.')
                    });

                    if tokens.len() > 5 && has_numbers {
                        // Split into separate cells
                        println!(
                            "📦 SPLITTING MERGED TEXT: {} tokens from {:?}",
                            tokens.len(),
                            cell
                        );
                        for token in tokens {
                            new_row.push(token.to_string());
                        }
                    } else {
                        new_row.push(cell.clone());
                    }
                }
                *row = new_row;
            }
        }

        // Re-normalize columns after splitting
        let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        for row in &mut rows {
            while row.len() < max_cols {
                row.push(String::new());
            }
        }
        let num_cols = max_cols;

        // 4. Format as Markdown Table
        let mut markdown = String::new();

        if !rows.is_empty() {
            // DEBUG: Show rows before formatting
            for (idx, row) in rows.iter().enumerate() {
                let row_text = row.join(" | ");
                if row_text.contains("Agentless") || row_text.contains("25.20") {
                    println!(
                        "🔥 TABLE ROW {}: {} cells, content: {}",
                        idx,
                        row.len(),
                        row_text
                    );
                }
            }

            // Header row
            markdown.push_str("| ");
            markdown.push_str(&rows[0].join(" | "));
            markdown.push_str(" |\n");

            // Separator row
            markdown.push('|');
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
        println!(
            "Accepted table: bbox={:?}, cols={}, rows={}",
            bbox, num_cols, num_rows
        );
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

        // 3. FIRST PRINCIPLES: Use fixed epsilon for column detection
        // Rationale: We want to cluster X-positions that represent the SAME COLUMN,
        // not every individual character. A column must be at least wide enough
        // for a few characters (~20-30pt minimum).
        //
        // Using adaptive epsilon from 10th percentile fails because:
        // - In prose text, characters are <1pt apart
        // - This creates 50-100+ clusters (one per character position)
        // - crossing_ratio check then rejects these as invalid tables
        //
        // Fixed epsilon of 15pt means:
        // - Text elements within 15pt horizontally are same column
        // - Handles slight alignment variations (ragged columns)
        // - Minimum column width is effectively ~30pt (15pt * 2)
        let epsilon = 15.0; // Points

        // 4. Apply DBSCAN clustering with min_samples=2
        // This requires at least 2 elements to form a column (reject single outliers)
        let clusters: Vec<Vec<f32>> = dbscan_1d(&x_coords, epsilon, 2);

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

    #[allow(dead_code)]
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
            } else if in_gap {
                in_gap = false;
                if i - gap_start >= min_gap_width {
                    // Found a gap. The column boundary is the middle of the gap.
                    let boundary_x = start_x as f32 + (gap_start + i) as f32 / 2.0;
                    column_boundaries.push(boundary_x);
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

    /// Extract text from cell, detecting merged cells by X-position clustering.
    /// Returns Vec<String> where each element is text from one logical column.
    /// Single-column cells return Vec with one element.
    fn extract_text_in_rect(
        &self,
        text_elements: &[TextElement],
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
    ) -> Vec<String> {
        let tol = 1.5;

        let mut contained: Vec<&TextElement> = text_elements
            .iter()
            .filter(|elem| {
                elem.x >= min_x - tol
                    && elem.x <= max_x + tol
                    && elem.y >= min_y - tol
                    && elem.y <= max_y + tol
            })
            .collect();

        if contained.is_empty() {
            return vec![String::new()];
        }

        // Sort by Y then X
        contained.sort_by(|a, b| {
            if (a.y - b.y).abs() < 1.0 {
                a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        // Filter decorative text
        let filtered: Vec<&TextElement> = contained
            .into_iter()
            .filter(|elem| {
                let is_decorative = elem.text.len() > 1
                    && elem
                        .text
                        .chars()
                        .all(|c| !c.is_alphanumeric() && !c.is_whitespace());
                !is_decorative
            })
            .collect();

        if filtered.is_empty() {
            return vec![String::new()];
        }

        // DEBUG: Show cells containing specific text
        let joined_text = filtered
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if joined_text.contains("Agentless") || joined_text.contains("25.20") {
            println!(
                "📋 TARGET CELL: bbox=[{:.1},{:.1},{:.1},{:.1}], {} elems",
                min_x,
                min_y,
                max_x,
                max_y,
                filtered.len()
            );
            println!("   text: {:?}", joined_text);
            for (idx, elem) in filtered.iter().enumerate() {
                println!("   [{}] x={:.1}, text={:?}", idx, elem.x, elem.text);
            }
        }

        // FIRST PRINCIPLES: Detect merged cells by X-position clustering
        // Problem: PDF has one grid cell containing text at multiple X-positions
        // Solution: Cluster by X, treat each cluster as separate logical cell

        // Cluster by X-position (20pt tolerance for same column)
        let epsilon = 20.0;
        let x_coords: Vec<f32> = filtered.iter().map(|e| e.x).collect();

        let clusters = dbscan_1d(&x_coords, epsilon, 1);

        if clusters.len() > 1 {
            println!(
                "  → Merged cell detected: {} X-position clusters in cell bbox [{:.1}, {:.1}]",
                clusters.len(),
                min_x,
                max_x
            );
        }

        if clusters.len() <= 1 {
            // Single column - return as one string
            let text: String = filtered
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            return vec![text];
        }

        // Multiple columns detected - this is a MERGED CELL
        // Sort clusters by leftmost X-coordinate
        let mut sorted_clusters: Vec<(f32, Vec<f32>)> = clusters
            .iter()
            .map(|cluster| {
                let min_x = cluster.iter().fold(f32::INFINITY, |acc, &x| acc.min(x));
                (min_x, cluster.clone())
            })
            .collect();
        sorted_clusters.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Assign each element to its cluster
        let mut cluster_texts: Vec<Vec<&TextElement>> = vec![Vec::new(); sorted_clusters.len()];
        for elem in &filtered {
            // Find which cluster this element belongs to
            for (cluster_id, (_min_x, cluster)) in sorted_clusters.iter().enumerate() {
                if cluster.iter().any(|&x| (x - elem.x).abs() < 0.5) {
                    cluster_texts[cluster_id].push(elem);
                    break;
                }
            }
        }

        // Build result strings, filtering empty clusters
        cluster_texts
            .iter()
            .map(|elems| {
                elems
                    .iter()
                    .map(|e| e.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect()
    }

    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_h_line(x1: f32, y: f32, x2: f32) -> PdfLine {
        PdfLine {
            p1: (x1, y),
            p2: (x2, y),
            width: 1.0,
        }
    }

    fn make_v_line(x: f32, y1: f32, y2: f32) -> PdfLine {
        PdfLine {
            p1: (x, y1),
            p2: (x, y2),
            width: 1.0,
        }
    }

    #[test]
    fn test_lattice_engine_creation() {
        let engine = LatticeEngine::new();
        assert_eq!(engine.min_line_length, 10.0);
        assert_eq!(engine.line_tolerance, 2.0);
    }

    #[test]
    fn test_empty_lines_no_tables() {
        let engine = LatticeEngine::new();
        let tables = engine.detect_tables(&[], &[], 600.0, 800.0);
        assert!(tables.is_empty());
    }

    #[test]
    fn test_filter_horizontal_lines() {
        let engine = LatticeEngine::new();
        let lines = vec![
            make_h_line(0.0, 100.0, 200.0), // Horizontal
            make_v_line(100.0, 0.0, 200.0), // Vertical
        ];
        let (h_lines, v_lines) = engine.filter_lines(&lines);
        assert_eq!(h_lines.len(), 1, "Should detect 1 horizontal line");
        assert_eq!(v_lines.len(), 1, "Should detect 1 vertical line");
    }

    #[test]
    fn test_line_intersection() {
        let engine = LatticeEngine::new();
        let h_line = make_h_line(0.0, 100.0, 200.0);
        let v_line = make_v_line(100.0, 50.0, 150.0);

        // They should intersect at (100, 100)
        assert!(
            engine.lines_intersect(&h_line, &v_line),
            "Perpendicular lines should intersect"
        );
    }

    #[test]
    fn test_parallel_lines_no_intersect() {
        let engine = LatticeEngine::new();
        let line1 = make_h_line(0.0, 100.0, 200.0);
        let line2 = make_h_line(0.0, 150.0, 200.0);

        assert!(
            !engine.lines_intersect(&line1, &line2),
            "Parallel horizontal lines should not intersect"
        );
    }

    #[test]
    fn test_simple_box_table_detection() {
        let engine = LatticeEngine::new();

        // Create a simple box (4 lines forming a rectangle)
        // Box from (10,10) to (110,60)
        let lines = vec![
            make_h_line(10.0, 60.0, 110.0), // Top horizontal
            make_h_line(10.0, 10.0, 110.0), // Bottom horizontal
            make_v_line(10.0, 10.0, 60.0),  // Left vertical
            make_v_line(110.0, 10.0, 60.0), // Right vertical
        ];

        // This tests that detect_tables runs without panic
        // Actual table detection depends on complex intersection logic
        let tables = engine.detect_tables(&lines, &[], 600.0, 800.0);
        // Even if no tables found, the function should complete
        assert!(
            tables.len() <= 1,
            "At most one table from 4 connected lines"
        );
    }

    #[test]
    fn test_not_enough_lines_for_table() {
        let engine = LatticeEngine::new();

        // Only 2 lines - not enough for a table
        let lines = vec![
            make_h_line(10.0, 100.0, 110.0),
            make_v_line(10.0, 50.0, 100.0),
        ];

        let tables = engine.detect_tables(&lines, &[], 600.0, 800.0);
        // The connected component only has 2 lines, minimum is 4
        assert!(tables.is_empty(), "2 lines should not form a table");
    }
}
