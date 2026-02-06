# Iteration 04: DECIDE - Implement Table Markdown Rendering

## Decision

**Replace stub `render_table()` with proper Markdown pipe table rendering.**

### Implementation Plan

1. **Read table cells from `block.children`**
   - Filter children where `block_type == TableCell`
   - Extract text content from each cell

2. **Group cells into rows**
   - Sort by Y-coordinate (y0)
   - Group cells with similar Y into same row
   - Sort each row by X-coordinate (x0)

3. **Build Markdown table**
   - Calculate max columns from all rows
   - First row = header
   - Add `| --- | --- |` separator
   - Format data rows with pipes

### Code Location

File: `src/layout/pymupdf_renderer.rs`
Function: `render_table()` (line ~158)

### Expected Output

Input (TableCell children):

```
[Cell y0=100: "Name", Cell y0=100: "Age"]
[Cell y0=115: "Alice", Cell y0=115: "30"]
```

Output:

```markdown
| Name  | Age |
| ----- | --- |
| Alice | 30  |
```

### Tests to Add

1. `test_render_simple_table` - 2x2 table
2. `test_render_table_with_empty_cells` - Missing cell handling
3. `test_render_table_single_row` - Edge case (header only)

### Commit Message

```
OODA-IT04: Implement Markdown table rendering

- Replace stub render_table() with proper pipe table generation
- Group TableCell children by Y-coordinate into rows
- Format as GitHub-flavored Markdown table with | separators
- Add --- header separator after first row
```
