# Iteration 04: ORIENT - Table Rendering Gap Analysis

## Analysis Summary

The table detection is **partially working** but the rendering is **completely broken**.

### Architecture Assessment

```
┌─────────────────────────────────────────────────────────────┐
│                  DATA FLOW ANALYSIS                         │
├─────────────────────────────────────────────────────────────┤
│  PDF → TableDetectionProcessor → Block(type=Table)          │
│                                    ↓                        │
│                        block.children: Vec<TableCell>       │
│                                    ↓                        │
│               MarkdownRenderer::render_table()              │
│                                    ↓                        │
│         self.render_paragraph(block)  ← BUG: IGNORES CELLS! │
└─────────────────────────────────────────────────────────────┘
```

### Root Cause: render_table() Ignores Children

The `TableDetectionProcessor` correctly:

1. Identifies table regions
2. Creates `Block` with `BlockType::Table`
3. Adds cells as `block.children` with `BlockType::TableCell`

But `MarkdownRenderer::render_table()` ignores `block.children` and just renders
the block as a paragraph (concatenating all text without structure).

### Solution Strategy

**Goal:** Transform table cells into Markdown pipe table format

```markdown
| Header 1 | Header 2 | Header 3 |
| -------- | -------- | -------- |
| Cell 1   | Cell 2   | Cell 3   |
| Cell 4   | Cell 5   | Cell 6   |
```

**Algorithm:**

1. Group `block.children` by Y-coordinate (rows)
2. First row may be header (use `---` separator)
3. For each row, join cell text with `|`
4. Pad columns for alignment (optional)

### Risk Assessment

| Risk                        | Impact           | Mitigation                      |
| --------------------------- | ---------------- | ------------------------------- |
| Cells not properly detected | Garbled output   | Validate cell count per row     |
| Merged cells                | Columns misalign | Skip merged cell handling (MVP) |
| Unicode in cells            | Display issues   | Already handled by text cleanup |
| Empty cells                 | Missing columns  | Insert empty `\|  \|`           |

### First Principles

**WHY Markdown tables?**

1. LLMs can parse structured Markdown tables
2. RAG systems extract entity-relationships from tables
3. Preserves semantic structure (header vs data)

**WHY pipe format?**

1. Industry standard (GitHub, CommonMark)
2. PyMuPDF4LLM uses this format
3. Readable in plain text

## Decision Preview

Implement proper `render_table()` that:

1. Groups cells by Y-coordinate into rows
2. Detects column count from max cells per row
3. Outputs pipe-separated Markdown table
4. Adds `---` separator after first row (assumed header)
