# OODA Loop 4 - ORIENT (Root Cause Analysis)

**Date:** 2026-01-03  
**Directory Scope:** `crates/edgequake-pdf/src/backend/lattice.rs`  
**Problem:** Cell text splitting failure - multi-column text dumped into single cells

## First Principles Analysis

### What is a PDF Table?

**Physical representation (PDF):**

- Lines (PdfLine elements) forming a grid
- Text elements (TextElement) positioned at specific (x, y) coordinates
- Text positioning MAY or MAY NOT align with line grid

**Logical representation (Markdown):**

```markdown
| Col1 | Col2 | Col3 |
| ---- | ---- | ---- |
| A    | B    | C    |
```

Each cell contains ONE piece of data.

### Current Algorithm (Lattice Detector)

```
1. Find all PdfLine elements
2. Extract unique X and Y coordinates from line endpoints
3. Create grid: cells[i][j] = rectangle(unique_x[j], unique_y[i], unique_x[j+1], unique_y[i+1])
4. For each cell: extract_text_in_rect() → get ALL text in bbox → concatenate with spaces
5. Build markdown table from cells
```

**Problem Location:** Step 4

### The Fundamental Flaw

**Assumption:** One lattice cell = One logical cell

**Reality:** The PDF grid lines don't always align with logical column boundaries!

**Example from one_tool:**

**PDF physical structure:**

```
+----------------------------------------------------------+
| Agentless Training Free 25.20 14.30 16.14 12.28 ...     |
+----------------------------------------------------------+
```

One big lattice cell containing text at multiple X-positions.

**Logical structure (what SHOULD be):**

```
+----------+----------+------+------+------+------+
| Agentless| Training | 25.20| 14.30| 16.14| 12.28|
|          | Free     |      |      |      |      |
+----------+----------+------+------+------+------+
```

Six logical cells with different data.

### Why This Happens

**PDF authoring tools** often create tables in two ways:

1. **Properly gridded:** Line for every column/row boundary

   - unique_x[] has entries for every column
   - Works correctly with current code

2. **Under-gridded:** Fewer lines than logical boundaries
   - Large cells with multiple text elements inside
   - Text positioned at different X-coordinates within same cell
   - Current code fails: dumps all text into one cell

### Evidence from Direct Comparison

**Gold markdown (correct):**

```markdown
| Agent Pipeline | Model | Function-level Recall | Funct Precision | ...
```

10 distinct columns.

**Generated markdown (broken):**

```markdown
| CoSIL | Training | Free | 48.61 | 13.40 | 19.81 | 12.12 | 78.35 |
```

8 columns, mid-table start, wrong data.

### First Principles Solution

**We cannot rely solely on PdfLine grid.**

**Must use:** Text element X-positions to infer column boundaries.

**Algorithm (First Principles):**

```
For each lattice cell with bbox [left, bottom, right, top]:
  1. Get all text elements in bbox
  2. Group text by X-coordinate (cluster within tolerance)
  3. Sort clusters by X-position (left to right)
  4. Each cluster = ONE LOGICAL CELL
  5. Return Vec<String> instead of String
```

**Then:** Match these logical cells to markdown columns.

## Research: Similar Problems in PDF Table Extraction

### Tabula (Java library)

Tabula uses "lattice" and "stream" modes:

- **Lattice mode:** Uses ruling lines (like our current code)
- **Stream mode:** Uses text positions ONLY (no lines)

**Key insight:** Even in lattice mode, Tabula SUBDIVIDES cells by text position!

Source: https://github.com/tabulapdf/tabula-java/blob/master/src/main/java/technology/tabula/extractors/BasicExtractionAlgorithm.java

### Camelot (Python library)

Camelot's lattice detection:

1. Finds grid lines
2. Gets candidate cells
3. **Analyzes text position distribution within each cell**
4. **Splits cells if text forms distinct columns**

Source: https://camelot-py.readthedocs.io/en/master/user/advanced.html

### Academic Papers

"Table Structure Recognition in Document Images" (2014)

- Problem: "Grid lines do not always correspond to logical structure"
- Solution: "Use text alignment as secondary signal"

### Pattern in Real-World PDFs

From dataset analysis:

- **2900_Goyal:** Fully gridded, works
- **one_tool:** Under-gridded, broken
- **agent_2510:** No grid lines at all (whitespace), completely broken

**Conclusion:** Need HYBRID approach using both lines AND text positions.

## Decision Criteria

### Option 1: Text Position Clustering Within Cells

**Approach:** Modify `extract_text_in_rect()` to return `Vec<String>` instead of `String`.

**Algorithm:**

```rust
fn extract_text_in_rect(...) -> Vec<String> {
    let elements = get_elements_in_bbox(...);
    let clusters = cluster_by_x_position(elements, tolerance=5.0);
    clusters.iter().map(|c| concat_texts(c)).collect()
}
```

**Pros:**

- Fixes the one_tool problem directly
- Maintains first principles: uses actual text positions
- Relatively small code change

**Cons:**

- Changes function signature (affects callers)
- Need to handle variable number of subcells per lattice cell
- Might over-split if tolerance is wrong

### Option 2: Pre-process Text to Infer Column Boundaries

**Approach:** Before building lattice, analyze text X-positions across all rows to find column boundaries.

**Algorithm:**

```rust
fn infer_column_boundaries(text_elements: &[TextElement]) -> Vec<f32> {
    // Group by Y (rows)
    // For each row, collect X positions
    // Find X-positions that appear consistently across rows
    // These are column boundaries
}
```

**Pros:**

- More robust: uses statistical patterns across entire table
- Doesn't depend on lattice cell boundaries

**Cons:**

- Complex implementation
- Requires identifying which text belongs to which table
- May not work if table structure varies by row

### Option 3: Hybrid - Use Both Lines and Text

**Approach:**

1. Use lattice lines as primary structure
2. Use text positions as secondary refinement
3. When lattice cell has text at multiple X-clusters, split it

**This is Option 1 but described differently.**

## Decision

**Choose Option 1: Text Position Clustering Within Cells**

**Rationale:**

- Direct fix for observed problem
- Aligns with first principles (use actual text positions)
- Similar to Tabula/Camelot approaches (proven in production)
- Minimal code change footprint

**Implementation plan:**

1. Create `cluster_by_x_position(elements: &[TextElement]) -> Vec<Vec<&TextElement>>`
2. Modify `extract_text_in_rect()` to return `Vec<String>`
3. Update lattice table building to handle variable-width cells
4. Add tests for clustering logic

**Expected impact:**

- one_tool: 11% → 40-50% (fixing cell splitting)
- May help other documents with similar issues
- Won't fix agent_2510 (still need whitespace detection)

**Risk mitigation:**

- Test clustering tolerance (5pt seems reasonable)
- Verify doesn't break 2900_Goyal (currently working)
- Add unit tests for edge cases (single element, empty cell, etc.)
