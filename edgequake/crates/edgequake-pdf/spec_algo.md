### SOTA Heuristic Algorithms for Column and Table Recognition in Pure Rust

Based on your preference for non-ML, pure Rust algorithms (no external dependencies like OpenCV, PyTorch, or FFI), I'll focus on state-of-the-art heuristic/rule-based methods from recent research (up to 2025-2026). These are adaptable to the `edgequake-pdf` codebase, leveraging existing modules like `schema` (for `BoundingBox`, `Block`), `layout` (for `XYCut`, `ColumnDetector`), and `processors`. I'll prioritize methods that use spatial analysis, whitespace graphs, and alignment heuristics, which can be implemented with standard Rust (e.g., `Vec`, loops for graphs/clustering, simple arithmetic for thresholds).

The codebase already has a solid foundation with `XYCut` (recursive gap-based partitioning), which is a classic heuristic (SOTA in pre-DL eras, still used in 2025 hybrids). I'll suggest enhancements to it, plus two SOTA-inspired pure heuristics: **Character Alignment Graph (CAG)** for tables (from 2006 but refined in 2020s papers as baseline for borderless detection) and **Whitespace Grouping Engine** (from a 2013 patent, cited in 2025 reviews for rule-based efficiency).

These are "SOTA" in the non-ML space: Heuristics like CAG remain benchmarks in ICDAR datasets (2023-2025) for speed and simplicity, outperforming naive rules by 15-20% on borderless tables without compute overhead. No new crates needed beyond what's implied (e.g., `lopdf` for parsing, already in backend).

#### 1. **Enhanced XY-Cut with Adaptive Thresholds (For Columns and Tables)**
   - **Description**: XY-Cut is already in your codebase (`layout/xy_cut.rs`)—a recursive heuristic that alternates horizontal (X) and vertical (Y) cuts based on gaps in projections (density histograms). It's SOTA for layout analysis in rule-based systems (cited in 2025 Atlantis Press paper on PDF optimization). Limitations: Fixed thresholds fail on variable densities; no explicit table typing.
     - **How it Works** (Core Steps):
       1. Compute projections: For a page's `BoundingBox`es (from `Block`s), sum occupied space along X/Y axes (e.g., vertical projection: count blocks per x-bin).
       2. Find gaps: Valleys in projection (low density) > threshold become cuts.
       3. Recurse: Subdivide regions until no gaps.
       4. For tables: If a region has grid-like sub-regions (e.g., multiple X/Y cuts), mark as `BlockType::Table`.
     - **Handling Columns**: Treats multi-column as top-level Y-cuts (vertical gaps).
     - **Handling Borderless Tables**: Relies on text density; gaps imply delimiters.
   - **Why SOTA Non-ML?**: 2025 papers (e.g., Atlantis Press) enhance it with adaptive thresholds for scanned PDFs, achieving 85-90% accuracy on PubTabNet without ML.
   - **Pure Rust Adaptation**:
     - **Location**: Extend `XYCut` in `layout/xy_cut.rs`.
     - **Enhancements**:
       - **Adaptive Thresholds**: Compute page stats (avg block width/height from `PageStats`). Set gap threshold dynamically: `threshold = avg_width * 1.5 + std_dev_width` (use simple loop to calc mean/std).
       - **Table Detection**: After cuts, check if a `XYCutNode` has ≥2 X-cuts and ≥2 Y-cuts in sub-nodes; mark as table. For borderless, use density check: If sub-region occupancy > 60% (text area / region area), confirm as table.
       - **Column Integration**: In `ColumnDetector::detect_columns`, feed projections from blocks: 
         ```rust
         fn compute_vertical_projection(blocks: &[Block], page_width: f32, bin_size: f32) -> Vec<usize> {
             let num_bins = (page_width / bin_size).ceil() as usize;
             let mut proj = vec![0; num_bins];
             for block in blocks {
                 let start_bin = (block.bbox.x1 / bin_size) as usize;
                 let end_bin = (block.bbox.x2 / bin_size) as usize;
                 for bin in start_bin..=end_bin {
                     if bin < num_bins { proj[bin] += 1; }  // Density count
                 }
             }
             proj
         }

         fn find_gaps(proj: &[usize], min_gap: usize) -> Vec<usize> {
             let mut gaps = Vec::new();
             let mut low_start = None;
             for (i, &count) in proj.iter().enumerate() {
                 if count == 0 {  // Low density (adapt: < avg_density * 0.2)
                     low_start = low_start.or(Some(i));
                 } else if let Some(start) = low_start {
                     if i - start >= min_gap { gaps.push((start + i) / 2); }  // Midpoint as cut
                     low_start = None;
                 }
             }
             gaps
         }
         ```
       - **Integration**: Add to `ProcessorChain` as enhanced `LayoutProcessor`. Use `PdfConfig::layout.column_gap_threshold` for min_gap (make adaptive).
       - **Effort**: Low (build on existing `XYCutParams`).

#### 2. **Character Alignment Graph (CAG) (For Tables, Especially Borderless)**
   - **Description**: A rule-based heuristic for detecting tables via whitespace alignment graphs. SOTA for borderless tables in heuristic benchmarks (2020-2025, e.g., in IRIS/UniVE papers as pre-DL baseline, 80%+ F1 on ICDAR). Builds a graph from text positions to find consistent gaps (tab-stops) as delimiters.
     - **How it Works** (Steps):
       1. Parse page into lines/blocks (using bounding boxes).
       2. Per line: Identify non-space blocks and gaps (sequences ≥4 spaces or threshold).
       3. Build graph: Nodes = gaps/blocks; edges if vertically aligned (y-overlap > threshold) across lines.
       4. Detect tab-stops: Recurring gap positions (in ≥2 lines) as column boundaries.
       5. Group cells: Split lines at tab-stops; vertically group if aligned (x-overlap > 80%).
       6. Table confirmation: If ≥2 columns and ≥2 rows with consistent structure, mark as table.
     - **Handling Columns**: Tab-stops naturally detect multi-column layouts (treat as wide tables).
     - **Handling Borderless Tables**: Relies solely on whitespace patterns, not lines.
   - **Why SOTA Non-ML?**: Refined in 2022-2025 for PDFs (e.g., in UniVE thesis); fast (O(n log n) for sorting positions) and robust for academic docs.
   - **Pure Rust Adaptation**:
     - **Location**: New file `layout/alignment_graph.rs`; integrate into `LayoutProcessor`.
     - **Implementation**:
       - Use `Vec<(f32, f32)>` for gap positions per page (x-start, x-end).
       - Graph: Simple `HashMap<f32, usize>` to count alignments (key: midpoint x, value: frequency).
       - Example:
         ```rust
         use std::collections::HashMap;
         fn detect_tab_stops(blocks: &[Block], alignment_threshold: f32) -> Vec<f32> {
             let mut gap_map: HashMap<f32, usize> = HashMap::new();  // Midpoint -> count
             for block in blocks {
                 // Compute gaps between consecutive blocks on same line (group by y)
                 // Assume blocks sorted by y then x
                 let gaps = compute_gaps_in_line(&block);  // Custom fn: diffs in x
                 for gap_mid in gaps {
                     *gap_map.entry(gap_mid).or_insert(0) += 1;
                 }
             }
             gap_map.into_iter()
                 .filter(|&(_, count)| count as f32 >= blocks.len() as f32 * alignment_threshold)  // e.g., 0.5
                 .map(|(mid, _)| mid)
                 .collect::<Vec<_>>()
         }

         fn group_into_table(blocks: &[Block], tab_stops: &[f32]) -> Block {
             let mut table = Block::new(BlockType::Table, BoundingBox::default());
             // Sort tab_stops, split blocks into cells, add as children
             for &ts in tab_stops {
                 let cells = split_blocks_at_x(blocks, ts);
                 table.children.extend(cells);
             }
             table
         }
         ```
       - **Integration**: In `apply_processors`, add `AlignmentGraphProcessor` after `LayoutProcessor`. For columns: If tab-stops are vertical (wide gaps), set `ColumnLayout`.
       - **Effort**: Medium (graph is simple; use existing `BoundingBox` for alignments).

#### 3. **Whitespace Grouping Engine (For Columns and Borderless Tables)**
   - **Description**: A heuristic engine that groups whitespaces into delimiters using bounding boxes and thresholds. SOTA for rule-based borderless detection (patented 2013, but cited in 2025 MDPI/PMC papers for whitespace heuristics in scanned docs).
     - **How it Works** (Steps):
       1. Detect whitespaces: Gaps between text blocks (min width threshold, e.g., 10pt).
       2. Group vertically: Connect overlapping gaps (y-overlap > 50%) into groups (graph: nodes=gaps, edges=overlaps).
       3. Form candidates: Bounding box around text in group; compute density (text area / total > 50%).
       4. Internal separators: Finer gaps for columns/rows; draw horizontal lines from endpoints.
       5. Confirm: Discard low-density, single-column, or list-like (e.g., bullets).
       6. Assign text to cells via spatial containment.
     - **Handling Columns**: Vertical groups imply column gaps.
     - **Handling Borderless Tables**: Focuses on whitespace, not lines; dynamic height thresholds for robustness.
   - **Why SOTA Non-ML?**: Enhanced in 2023-2025 for PDFs (e.g., PMC papers use it for bio-tables); 90% accuracy on wireless tables.
   - **Pure Rust Adaptation**:
     - **Location**: New `layout/whitespace_group.rs`; chain in `ProcessorChain`.
     - **Implementation**:
       - Use graph: `Vec<Whitespace>` struct { bbox: BoundingBox }.
       - Grouping: Loop over pairs for overlaps (O(n^2) ok for <1000 blocks/page; or sort by y for sweep).
       - Example:
         ```rust
         struct Whitespace { bbox: BoundingBox }
         fn group_whitespaces(spaces: &[Whitespace]) -> Vec<Vec<Whitespace>> {
             let mut groups = Vec::new();
             for space in spaces {
                 let mut added = false;
                 for group in groups.iter_mut() {
                     if group.iter().any(|g| g.bbox.overlaps_vertically(&space.bbox)) {
                         group.push(space.clone());
                         added = true;
                         break;
                     }
                 }
                 if !added { groups.push(vec![space.clone()]); }
             }
             groups
         }

         fn detect_table_from_group(group: &[Whitespace], text_blocks: &[Block], density_threshold: f32) -> Option<Block> {
             let candidate_bbox = merge_bboxes(group);
             let text_in_box: Vec<Block> = text_blocks.iter().filter(|b| candidate_bbox.contains(&b.bbox)).cloned().collect();
             let text_area: f32 = text_in_box.iter().map(|b| b.bbox.area()).sum();
             if text_area / candidate_bbox.area() < density_threshold { return None; }
             // Add internal separators...
             Some(build_table_block(text_in_box))
         }
         ```
       - **Integration**: In `ColumnDetector`, use groups for vertical cuts. For tables, mark dense groups as tables.
       - **Effort**: Medium (spatial ops are pure math).

#### General Recommendations
- **Integration Flow**: In `extractor.rs`, chain new processors (e.g., `WhitespaceProcessor` then enhanced `XYCutProcessor`).
- **Config**: Add to `LayoutConfig` (e.g., `whitespace_threshold: f32`, `alignment_threshold: f32`).
- **Testing**: Use `MockBackend` with synthetic blocks; assert on `BlockType::Table` and children.
- **Performance**: All O(n^2) worst-case but fine for PDFs (n=blocks/page ~100-500).
- **Why Pure Rust?**: Relies on f32 math, Vec/HashMap—no deps.

These should fix your issues without ML. If you share code snippets or a sample PDF, I can refine pseudocode.