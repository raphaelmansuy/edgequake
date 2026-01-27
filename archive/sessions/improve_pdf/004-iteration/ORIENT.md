# ORIENT.md - Iteration 004: First Principles Architecture Design

**Directory:** `edgequake/crates/edgequake-pdf/src`

## First Principles from PDF Specification

### 1. PDF Text Positioning (from ISO 32000)

**Operators:**

- `Tj` (show text): Display string at current text position
- `TJ` (show text with positioning): Display string with individual glyph positioning
- `Tm` (set text matrix): Set text line matrix and current text matrix
- `Td` (move text position): Move to start of next line
- `T*` (move to next line): Move to start of next line (offset by leading)

**Key Insight:** Every character has **exact (x, y) coordinates** from the Current Transformation Matrix (CTM).  
We should NOT use histogram binning - use actual coordinates!

### 2. Font Information (from PDF Font Dictionaries)

**Available in PDF:**

- `/BaseFont`: Font name (e.g., `ABCDEE+TimesNewRomanPSMT`)
- `/Subtype`: Type1, TrueType, Type3, CIDFont
- `/FontDescriptor`:
  - `/FontWeight`: Numeric weight (400=normal, 700=bold)
  - `/ItalicAngle`: Non-zero for italic
  - `/Flags`: Bitfield encoding properties
  - `/FontBBox`: Bounding box
  - `/CapHeight`, `/XHeight`, `/Ascent`, `/Descent`

**Key Insight:** Headers are identified by **font size + weight**, not keyword matching!

### 3. Graphics State and Layout

**From PDF Content Streams:**

- Line drawing operators: `m` (move), `l` (line), `re` (rectangle)
- Table borders are actual graphics primitives
- Clipping paths define text regions

**Key Insight:** Tables have **actual borders** in PDF - detect from graphics, not content patterns!

### 4. Structural Information

**PDF Logical Structure (Tagged PDF):**

- Outline/Bookmark tree for sections
- Tagged elements (H1, H2, P, Table, etc.)
- Reading order hints

**Key Insight:** Use structure when available, but must work without it (most PDFs aren't tagged).

## Proposed Modular Architecture

### Module 1: GeometricClustering

**Purpose:** Group text elements by spatial proximity  
**Algorithm:** DBSCAN (Density-Based Spatial Clustering)

```rust
pub struct GeometricClustering {
    eps: f32,              // Maximum distance between neighbors
    min_samples: usize,    // Minimum points to form cluster
}

impl GeometricClustering {
    /// Cluster text spans by (x, y) coordinates
    /// Returns: Vec<Cluster> where each cluster is spatially coherent
    pub fn cluster_spans(&self, spans: &[TextSpan]) -> Vec<Cluster> {
        // Use DBSCAN on (x, y) positions
        // eps = median_font_size * 1.5 (adaptive, not hardcoded!)
        // min_samples = 3
    }

    /// Detect columns from x-coordinate distribution
    /// Returns: Column boundaries
    pub fn detect_columns(&self, spans: &[TextSpan]) -> Vec<Column> {
        // 1. Extract x-coordinates of all spans
        // 2. Apply k-means clustering (k=1,2,3)
        // 3. Choose k with best silhouette score
        // 4. Return column regions
    }
}
```

**Why Better:**

- No histogram binning - uses actual coordinates
- Adaptive threshold (eps = median_font_size \* 1.5)
- Works for any layout/scale

### Module 2: FontMetricsAnalyzer

**Purpose:** Extract and classify font properties  
**Algorithm:** Statistical analysis of font attributes

```rust
pub struct FontMetricsAnalyzer {
    font_catalog: HashMap<String, FontMetrics>,
}

pub struct FontMetrics {
    size: f32,
    weight: u16,        // From FontDescriptor
    is_italic: bool,    // From ItalicAngle
    is_mono: bool,      // From Flags
}

impl FontMetricsAnalyzer {
    /// Analyze all fonts in document
    pub fn analyze_document(&mut self, doc: &Document) {
        // Extract font metrics from PDF dictionaries
        // Build catalog of font characteristics
    }

    /// Classify text level (heading vs body)
    pub fn classify_level(&self, span: &TextSpan) -> TextLevel {
        let metrics = &self.font_catalog[&span.font];
        let doc_stats = &self.document_stats;

        // Calculate z-score for font size
        let size_zscore = (metrics.size - doc_stats.median_size) / doc_stats.std_dev_size;

        // Heading detection:
        // - size_zscore > 1.5  → H1
        // - size_zscore > 1.0  → H2
        // - size_zscore > 0.5  → H3
        // - weight >= 700      → Bold (emphasis)
        // - is_italic          → Italic (emphasis)

        // No hardcoded thresholds! All relative to document statistics.
    }
}
```

**Why Better:**

- Uses actual PDF font data
- Statistical thresholds (z-scores), not hardcoded values
- Language-agnostic (no keyword lists)

### Module 3: TableExtractor

**Purpose:** Detect and extract table structure  
**Algorithm:** Border detection + cell alignment

```rust
pub struct TableExtractor {
    border_detector: BorderDetector,
    cell_aligner: CellAligner,
}

impl TableExtractor {
    /// Extract tables from page
    pub fn extract_tables(&self, page: &Page) -> Vec<Table> {
        // 1. Detect borders from line graphics
        let borders = self.border_detector.detect_lines(page);

        // 2. Find grid intersections
        let grid = self.find_grid_structure(&borders);

        // 3. If no borders, use text alignment
        if grid.is_empty() {
            return self.detect_alignment_table(page);
        }

        // 4. Extract cell contents within grid
        self.extract_cell_contents(&grid, page)
    }

    /// Detect tables from text alignment (no borders)
    fn detect_alignment_table(&self, page: &Page) -> Vec<Table> {
        // 1. Group spans by row (y-coordinate clustering)
        let rows = self.cluster_by_row(&page.spans);

        // 2. For each row, detect column positions (x-clustering)
        // 3. Check if columns are consistent across rows
        // 4. If >80% rows have same column structure → table

        // Uses Jaccard similarity for column consistency:
        // J(A,B) = |A ∩ B| / |A ∪ B|
        // If J > 0.8 for adjacent rows → same table
    }
}
```

**Why Better:**

- Uses PDF graphics primitives (borders)
- Geometric clustering for borderless tables
- No content pattern matching
- No hardcoded "table-like" scores

### Module 4: TextNormalizer

**Purpose:** Clean and normalize text  
**Algorithm:** Unicode normalization + statistical analysis

```rust
pub struct TextNormalizer {
    unicode_normalizer: UnicodeNormalizer,
}

impl TextNormalizer {
    /// Normalize text using Unicode standards
    pub fn normalize(&self, text: &str) -> String {
        // 1. Unicode NFKC normalization (canonical + compatibility)
        //    Handles: soft hyphens, zero-width spaces, ligatures
        let normalized = self.unicode_normalizer.nfkc(text);

        // 2. Whitespace normalization
        //    - Multiple spaces → single space
        //    - Preserve intentional spacing (code blocks, tables)

        // 3. Word break detection from TJ operator
        //    - Use actual glyph positions, not heuristics
        //    - Gap > 0.5 * space_width → word break

        normalized
    }

    /// Detect and fix hyphenation
    pub fn fix_hyphenation(&self, text: &str, next_text: &str) -> Option<String> {
        // Check if current line ends with hyphen
        // AND next line starts with lowercase
        // AND combined word exists in frequency dictionary

        if text.ends_with('-') && next_text.chars().next()?.is_lowercase() {
            let combined = format!("{}{}", &text[..text.len()-1], next_text);
            // No hardcoded word lists! Use corpus frequency or ignore.
            return Some(combined);
        }
        None
    }
}
```

**Why Better:**

- Unicode standard normalization (not custom byte patterns)
- Uses TJ operator positioning data
- Statistical validation, not hardcoded word lists

### Module 5: ReadingOrderAnalyzer

**Purpose:** Determine correct reading sequence  
**Algorithm:** Geometric layout analysis

```rust
pub struct ReadingOrderAnalyzer {
    column_detector: GeometricClustering,
}

impl ReadingOrderAnalyzer {
    /// Sort blocks by reading order
    pub fn sort_blocks(&self, blocks: &mut [Block], columns: &[Column]) {
        if columns.is_empty() {
            // Single column: sort by Y then X
            blocks.sort_by(|a, b| {
                let y_cmp = a.bbox.y1.partial_cmp(&b.bbox.y1);
                y_cmp.unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.bbox.x1.partial_cmp(&b.bbox.x1).unwrap_or(std::cmp::Ordering::Equal))
            });
        } else {
            // Multi-column: group by column, then sort each column
            for col in columns {
                let mut col_blocks: Vec<_> = blocks.iter_mut()
                    .filter(|b| col.contains(b.bbox.x1))
                    .collect();
                col_blocks.sort_by(|a, b| a.bbox.y1.partial_cmp(&b.bbox.y1).unwrap_or(std::cmp::Ordering::Equal));
            }
        }
    }
}
```

**Why Better:**

- Geometric sorting, no heuristics
- Respects column structure
- Language/layout agnostic

## Comparison: Old vs New Approach

| Aspect                 | Old (Heuristic)                           | New (First Principles)                     |
| ---------------------- | ----------------------------------------- | ------------------------------------------ |
| **Column Detection**   | Histogram bins (5pt), threshold 0.35\*max | DBSCAN on actual coordinates, adaptive eps |
| **Header Detection**   | Keyword matching (60+ words)              | Font size z-score + weight from PDF        |
| **Table Detection**    | Content patterns (pipes, digits)          | Border graphics + alignment clustering     |
| **Text Normalization** | Hardcoded control chars (\x02, \x1F)      | Unicode NFKC standard                      |
| **Thresholds**         | Magic numbers (25.0, 50.0, 100.0)         | Document statistics (median, std dev)      |
| **Adaptability**       | Breaks on new documents                   | Works on any PDF                           |

## Implementation Plan

### Phase 1: Core Geometric Module (Iteration 004)

**Target:** Replace column detection histogram with coordinate clustering

**Files to modify:**

- `src/layout/column_detector.rs` - Replace histogram with DBSCAN
- Add `src/layout/geometric.rs` - New clustering module
- Add tests in `tests/geometric_test.rs`

**Expected Improvement:** Table Accuracy 3.5% → 8%+

### Phase 2: Font Analysis Module (Iteration 005)

**Target:** Replace keyword-based header detection

**Files to modify:**

- Add `src/analysis/font_metrics.rs`
- Modify `src/processors/processor.rs` - Remove SECTION_KEYWORDS
- Update SectionPatternProcessor to use font metrics

**Expected Improvement:** Style Accuracy 16.9% → 30%+

### Phase 3: Table Extraction Module (Iteration 006)

**Target:** Border-based + alignment table detection

**Files to modify:**

- Add `src/extraction/table_extractor.rs`
- Modify `src/processors/processor.rs` - Remove table_like_score(), parse_agent_pipeline
- Add border detection from graphics stream

**Expected Improvement:** Table Accuracy 8% → 20%+

### Phase 4: Text Normalization Module (Iteration 007)

**Target:** Unicode standard normalization

**Files to modify:**

- Add `src/normalization/text_normalizer.rs`
- Modify PostProcessor to use new normalizer
- Remove hardcoded control character handling

**Expected Improvement:** Style Accuracy + robustness

## Success Criteria

1. **No hardcoded constants** (except mathematical constants like π)
2. **All thresholds adaptive** (derived from document statistics)
3. **No keyword/pattern lists** (language-agnostic)
4. **Single Responsibility** (each module does ONE thing)
5. **Composable** (modules work independently)
6. **Testable** (unit tests for each module)
7. **Metrics improve** (composite score 27.2 → 50+)

## Next Step: DECIDE Phase

Select ONE module to implement first. Recommended: **GeometricClustering** (Phase 1)  
Reason: Biggest impact, foundational for other modules, clear win.
