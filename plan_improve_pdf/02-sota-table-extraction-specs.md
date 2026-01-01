# SOTA Table Extraction Specification (No ML)

## 1. Architecture Overview

The table extraction pipeline will run _before_ the main text layout analysis.

```ascii
[PDF Content Stream]
       |
       v
[Operator Parser] --> [TextElements]
       |          --> [GraphicalLines] (m, l, re)
       v
[Table Detector]
       |
       +--> [Lattice Strategy] (Explicit Lines)
       |         |
       |         v
       |    [Grid Detection]
       |
       +--> [Stream Strategy] (Implicit Alignment)
                 |
                 v
            [Alignment Analysis]
       |
       v
[Table Candidate Validation]
       |
       v
[Table Extractor] --> [TableBlock] (Markdown)
       |
       v
[Text Excluder] --> [Remaining TextElements]
       |
       v
[Layout Processor] (Columns, Blocks)
```

## 2. Data Structures

### 2.1 Graphical Line

```rust
struct PdfLine {
    p1: Point, // (x, y)
    p2: Point, // (x, y)
    width: f32,
    is_horizontal: bool,
    is_vertical: bool,
}
```

### 2.2 Table Cell

```rust
struct TableCell {
    bbox: BoundingBox,
    text: String,
    row_span: usize,
    col_span: usize,
    is_header: bool,
}
```

### 2.3 Table Structure

```rust
struct DetectedTable {
    bbox: BoundingBox,
    rows: Vec<Vec<TableCell>>,
    confidence: f32,
    method: TableMethod, // Lattice or Stream
}
```

## 3. Algorithms

### 3.1 Lattice Algorithm (Explicit Tables)

Used when the table has visible borders.

1.  **Line Extraction:** Parse `m`, `l`, `re` operators. Transform coordinates using the Current Transformation Matrix (CTM).
2.  **Line Merging:** Merge collinear and overlapping lines.
3.  **Intersection Finding:** Find points where horizontal and vertical lines cross.
4.  **Cell Formation:**
    ```ascii
    +-------+-------+
    | (x,y) |       |
    +-------+-------+
    |       |       |
    +-------+-------+
    ```
    Find the smallest rectangles formed by the grid of lines.
5.  **Content Assignment:** Assign `TextElement`s to cells based on `bbox` inclusion.

### 3.2 Stream Algorithm (Implicit Tables)

Used when the table has no borders (whitespace only).

1.  **Row Detection:** Group text elements by Y-overlap.
    ```ascii
    Row 1: [Word]   [Word]   [Word]
    Row 2: [Word]   [Word]   [Word]
    ```
2.  **Column Detection:**
    - Calculate vertical projection profile of text bounding boxes within the candidate region.
    - Find gaps (valleys) in the X-projection.
    ```ascii
    Text:  [AAA]   [BBB]   [CCC]
           [DDD]   [EEE]   [FFF]
    Proj:  #####   #####   #####
    Gaps:       ^^^     ^^^
    ```
3.  **Table Boundary Guessing:**
    - Look for regions with high density of aligned text.
    - Heuristic: > 2 columns, > 2 rows, consistent alignment.

## 4. Integration with SotaBackend

1.  **Extend `extract_text_elements`:** Rename to `extract_elements` and return both `TextElement`s and `PdfLine`s.
2.  **Implement `detect_tables`:**
    - Input: `Vec<TextElement>`, `Vec<PdfLine>`
    - Output: `Vec<DetectedTable>`
3.  **Filter Text:**
    - Create a spatial index (or simple loop) to remove `TextElement`s that fall inside any `DetectedTable` bbox.
4.  **Render Markdown:**
    - Convert `DetectedTable` to a Markdown string.
    - Insert as a `BlockType::Table`.

## 5. Edge Cases & Heuristics

- **Spanning Cells:** In Lattice, if a grid cell is empty and no line separates it from the left/top, merge.
- **Multi-line Cells:** In Stream, if rows are close (line-height) and columns align, merge into one row.
- **False Positives:** Avoid detecting multi-column text layouts as tables.
  - _Check:_ Tables usually have numbers or short text. Prose has long sentences.
  - _Check:_ Tables often have headers (bold, centered).

## 6. Cross-Check with Code

- `sota_backend.rs` already has `BoundingBox` and `TextElement`.
- Need to add `PdfLine` struct.
- Need to add `lopdf` operator parsing for graphics.
