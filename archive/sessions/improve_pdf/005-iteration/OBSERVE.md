# OBSERVE.md - Iteration 005

**Directory:** `edgequake/crates/edgequake-pdf/src/backend`

**Timestamp:** 2026-01-02

## Current State

### Baseline Metrics (from iteration 004)

- Table Accuracy: 2.4%
- Style Accuracy: 31.5%
- Composite Score: 32.5/100
- Robustness: 100%
- Performance: 90%

### Test Results

- All 111 tests passing
- 11 compiler warnings (mostly unused fields)

### Code Smells Identified

#### 1. Lattice Engine Completely Disabled (CRITICAL)

**Location:** `sota_backend.rs:2570-2575`

```rust
// Detect tables
let tables: Vec<Block> = Vec::new(); // DISABLED FOR NOW
                                     // let tables = self.lattice_engine.detect_tables(
                                     //     page_num,
                                     //     &lines,
                                     //     &mut text_elements,
                                     //     page_width,
                                     //     page_height,
                                     // );
```

**Issue:** The `lattice_engine` field is created but never used. Table detection is completely disabled, returning an empty vector.

**Impact:** Table Accuracy is only 2.4% because tables are not being detected at all.

**First Principles Violation:** Using empty vector instead of proper lattice-based table detection algorithm.

#### 2. Unused Field: lattice_engine

**Location:** `sota_backend.rs:1238`

```rust
pub struct SotaBackend {
    config: PdfConfig,
    lattice_engine: LatticeEngine,  // Never read
    // ...
}
```

**Issue:** The field is declared but never used, indicating incomplete implementation.

#### 3. Unused Fields in MergedLine

**Location:** `sota_backend.rs:1245`

```rust
struct MergedLine {
    text: String,
    bbox: BoundingBox,
    font_name: String,    // Never read
    is_bold: bool,        // Never read
    is_italic: bool,      // Never read
}
```

**Issue:** Style information is collected but not used in rendering pipeline.

**Impact:** Style Accuracy is only 31.5% because style information is being discarded.

## Lattice Engine Analysis

### What is Lattice Engine?

The `LatticeEngine` in `lattice.rs` implements a first-principles table detection algorithm:

1. **Graphical Line Detection:** Filters horizontal and vertical lines from PDF
2. **Connected Components:** Finds intersecting lines that form table grids
3. **Parallel Line Tables:** Detects tables without vertical lines (header/bottom only)
4. **Cell Extraction:** Extracts text from grid cells
5. **Markdown Formatting:** Formats tables as proper Markdown

### Algorithm Characteristics

- **First Principles:** Uses actual graphical lines from PDF (not heuristics)
- **No Magic Numbers:** Only `min_line_length: 10.0` and `line_tolerance: 2.0` (reasonable defaults)
- **Composable:** Can be integrated into extraction pipeline
- **Testable:** Pure functions with clear inputs/outputs

### Why Was It Disabled?

The comment says "DISABLED FOR NOW" but no reason is documented. Possible reasons:

1. Performance concerns (line intersection is O(n²))
2. False positives (detecting page borders as tables)
3. Incomplete testing

## Observations

### Table Detection is Completely Broken

- `tables` vector is always empty
- No tables are ever detected
- Table Accuracy of 2.4% is essentially random noise

### Lattice Engine is Production-Ready

- 514 lines of well-structured code
- Implements proper lattice-based table detection
- Has fallback for tables without vertical lines
- Formats output as Markdown tables

### Integration is Straightforward

The commented code shows the intended integration:

```rust
let tables = self.lattice_engine.detect_tables(
    page_num,
    &lines,           // pdf_lines from extract_page_elements
    &mut text_elements, // elements from extract_page_elements
    page_width,
    page_height,
);
```

## Next Steps

1. **ORIENT:** Analyze why lattice_engine was disabled and identify potential issues
2. **DECIDE:** Create minimal patch to enable lattice_engine with safeguards
3. **ACT:** Enable lattice_engine, add safety checks, measure Table Accuracy improvement

## Expected Impact

**Conservative Estimate:**

- Table Accuracy: 2.4% → 8-15%
- Composite Score: 32.5 → 35-38/100

**Reasoning:**

- Lattice engine uses actual graphical lines (first principles)
- Proper table detection algorithm (not heuristics)
- Current 2.4% is essentially random noise
- Even conservative improvement will be significant

## Risks

1. **False Positives:** Page borders might be detected as tables
2. **Performance:** Line intersection is O(n²) for n lines
3. **Regression:** Could break existing text extraction

**Mitigation:**

- Add minimum table size check
- Add maximum table size check (exclude page borders)
- Monitor performance metrics
- Keep existing text extraction as fallback
