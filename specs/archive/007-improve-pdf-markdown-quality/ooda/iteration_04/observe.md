# Iteration 04: OBSERVE - Table Detection & Rendering

**Date:** 2025-01-XX
**Focus:** Table Detection and Markdown Rendering (Priority: CRITICAL, Target: 50 → 80)

## Observation: Current Table Handling Architecture

### 1. Table Detection (`processors/table_detection.rs`)

**Two Processors:**

- `TableDetectionProcessor` - Spatial detection from block arrangement
- `TextTableReconstructionProcessor` - Text pattern-based detection

**Algorithm (TableDetectionProcessor):**

```
┌─────────────────────────────────────────────────────────────┐
│              TABLE DETECTION ALGORITHM                       │
├─────────────────────────────────────────────────────────────┤
│  1. Group blocks by Y-coordinate (rows)                     │
│  2. Sort each row by X-coordinate (left to right)           │
│  3. Find regions with multiple blocks per row               │
│  4. Apply heuristics:                                       │
│     - Skip multi-column pages (OODA-34)                     │
│     - Paragraph detection (OODA-21)                         │
│     - Author block rejection (OODA-32)                      │
│  5. Create Table block with TableCell children              │
└─────────────────────────────────────────────────────────────┘
```

**Key Heuristics:**

- `is_paragraph()`: Block > 55% page width AND > 60 chars = NOT table cell
- `is_likely_table()`: 3+ rows with multi-col = table candidate
- Y-tolerance: 10pt normal, 2pt strict mode
- Gap threshold: 150pt max between cells

### 2. Table Rendering (`layout/pymupdf_renderer.rs`)

**CRITICAL ISSUE:**

```rust
fn render_table(&self, block: &Block) -> String {
    // KNOWN LIMITATION: Proper table rendering not implemented
    // WHY: Requires cell boundary detection which is complex
    // WORKAROUND: Tables are rendered as paragraphs
    self.render_paragraph(block)
}
```

**Result:** Tables are detected but NOT rendered as Markdown tables!

### 3. PyMuPDF4LLM Reference (`pymupdf_rag.py`)

**Critical Difference:**

```python
# PyMuPDF4LLM uses native PyMuPDF table detection:
tabs = page.find_tables(clip=parms.clip, strategy=table_strategy)
# Default strategy: "lines_strict"

# Renders using PyMuPDF's built-in to_markdown():
this_md += parms.tabs[i].to_markdown(clean=False)
```

**PyMuPDF's table detection advantages:**

1. Uses PDF graphics (lines/rules) to detect cell boundaries
2. Identifies header rows
3. Handles merged cells (colspan/rowspan)
4. Outputs proper Markdown pipe tables

### 4. Gap Analysis

| Feature                   | PyMuPDF4LLM            | EdgeQuake                  | Gap      |
| ------------------------- | ---------------------- | -------------------------- | -------- |
| Graphics-based detection  | ✅ Uses PDF line paths | ❌ Spatial only            | HIGH     |
| Markdown table output     | ✅ `to_markdown()`     | ❌ Falls back to paragraph | CRITICAL |
| Header row identification | ✅ Built-in            | ❌ No header detection     | HIGH     |
| Merged cell handling      | ✅ Supported           | ❌ Not supported           | MEDIUM   |
| Pipe table format         | ✅ `\| A \| B \|`      | ❌ No pipe format          | CRITICAL |

## Root Cause

**The table rendering is a stub that does nothing useful!**

Location: `src/layout/pymupdf_renderer.rs:158-164`

```rust
fn render_table(&self, block: &Block) -> String {
    self.render_paragraph(block)  // ← Tables become paragraphs!
}
```

## Files Analyzed

1. `src/processors/table_detection.rs` (1072 lines) - Detection logic
2. `src/layout/pymupdf_renderer.rs` (511 lines) - Markdown rendering
3. `zz-explore/pymupdf4llm/.../pymupdf_rag.py` (1377 lines) - Gold standard

## Next Steps

1. Implement proper Markdown table rendering in `render_table()`
2. Extract cell boundaries from `block.children` (TableCell blocks)
3. Generate pipe table format: `| Col1 | Col2 |`
4. Add separator row: `| --- | --- |`
