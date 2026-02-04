# OODA-02: Observe

## Mission Re-Read ✅

**File**: `specs/005-perfect-pdf-pymupdf4llm-inspired-conversion.md`
**Goal**: F1 >= 0.95 by implementing pymupdf4llm algorithms with pdfium-render backend

---

## Current State

| Metric           | OODA-01 End | Target | Gap    |
| ---------------- | ----------- | ------ | ------ |
| Average F1       | 0.871       | 0.95   | -0.079 |
| 01_2512.25075v1  | 0.814       | 0.90   | -0.086 |
| AlphaEvolve      | 0.813       | 0.90   | -0.087 |
| one_tool         | 0.829       | 0.90   | -0.071 |
| v2_2512.25072v1  | 0.907       | 0.90   | ✓      |
| 2900_Goyal       | 0.898       | 0.90   | -0.002 |
| ccn_2512.21804v1 | 0.941       | 0.90   | ✓      |
| agent_2510       | 0.896       | 0.90   | -0.004 |

**3 files below target**: 01_2512, AlphaEvolve, one_tool

---

## Root Cause Analysis: Low F1 Files

### Issue 1: Rotated Text Scattered

**Example**: `01_2512.25075v1.pdf` contains:

```
5 2 0 2 c e D 1 3
```

This is the arXiv date "31 Dec 2025" written vertically in the margin. Each character is extracted as a separate entity with rotated coordinates.

**pymupdf4llm solution** (get_text_lines.py line 121):

```python
if only_horizontal and abs(1 - line_dir[0]) > 1e-3:  # only accept horizontal text
    continue
```

**Our implementation**: Missing this check entirely.

### Issue 2: Reading Order Wrong for Multi-Column

**Example**: `one_tool_2512.20957v2.pdf` shows:

```
Abstract Locating the files and functions requiring modifi cation in large 5 itories
```

The "5" is from the right column (arXiv ID) being interleaved with left column text.

**pymupdf4llm solution** (multi_column.py lines 283-305):

```python
# Smart sort key: for block Q, find left-most block P with vertical overlap
# Use (P.y0, Q.x0) as sort key
left_rects = sorted([
    r for r in new_rects
    if r.x1 < box.x0  # to the left
    and (box.y0 <= r.y0 <= box.y1 or box.y0 <= r.y1 <= box.y1)  # vertical overlap
], key=lambda r: r.x1)

if left_rects:
    key = (left_rects[-1].y0, box.x0)  # use left block's Y
else:
    key = (box.y0, box.x0)
```

**Our implementation**: Uses simpler center-based column detection.

### Issue 3: Paragraph Fragmentation

**Example**: `AlphaEvolve.pdf` output has 10,016 lines vs gold standard's 2,547 lines (4x fragmentation).

**pymupdf4llm solution** (multi_column.py lines 217-245):

```python
# join_rects_phase2: normalize x0/x1 within 3pt, then join vertically
x0 = min([bb.x0 for bb in prects if abs(bb.x0 - b.x0) <= 3])
x1 = max([bb.x1 for bb in prects if abs(bb.x1 - b.x1) <= 3])

# join if similar borders and gap <= 10pt
if abs(r.x0 - r0.x0) <= 3 and abs(r.x1 - r0.x1) <= 3 and abs(r0.y1 - r.y0) <= 10:
    r0 |= r  # merge
```

**Our implementation**: Fixed block gap of 20pt, no boundary normalization.

### Issue 4: Word Boundary Detection

**pymupdf4llm solution** (get_text_lines.py lines 79-89):

```python
# Spans are joined if gap < 10% of font size
delta = s1["size"] * 0.1
if s0["bbox"].x1 + delta < s1["bbox"].x0:
    continue  # don't join
```

**Our implementation**: Uses 25% threshold (too aggressive).

---

## pymupdf4llm Algorithm Inventory

### File: `multi_column.py`

| Function            | Lines   | Purpose                                         | Our Status  |
| ------------------- | ------- | ----------------------------------------------- | ----------- |
| `column_boxes()`    | 70-335  | Main entry point, orchestrates phases           | Partial     |
| `join_rects_phase1` | 189-211 | Join touching rects (10pt vertical gap allowed) | **Missing** |
| `join_rects_phase2` | 213-246 | Normalize boundaries (3pt), then join           | **Missing** |
| `join_rects_phase3` | 248-328 | Smart sort key for reading order                | **Missing** |
| `clean_nblocks()`   | 148-186 | Remove duplicates, sort by bottom then left     | **Missing** |

### File: `get_text_lines.py`

| Function           | Lines   | Purpose                             | Our Status  |
| ------------------ | ------- | ----------------------------------- | ----------- |
| `get_raw_lines()`  | 25-177  | Extract spans, group into lines     | Partial     |
| `sanitize_spans()` | 66-99   | Sort and join broken spans          | Partial     |
| Filter rotated     | 121-123 | Skip non-horizontal text            | **Missing** |
| Superscript bbox   | 140-145 | Adjust superscript bbox to neighbor | **Missing** |

---

## Key Constants Comparison

| Constant            | pymupdf4llm          | Our Implementation | Action Needed         |
| ------------------- | -------------------- | ------------------ | --------------------- |
| Vertical join gap   | 10pt                 | 20pt               | Reduce to 10pt        |
| Boundary normalize  | 3pt                  | None               | Add 3pt normalization |
| Line tolerance      | 3pt                  | 3pt                | ✓ OK                  |
| Word join threshold | 10% font_size        | 25% font_size      | Reduce to 10%         |
| Horizontal filter   | `abs(1-dir[0])>1e-3` | None               | Add filter            |

---

## Codebase Current State

### Files to Modify

```
edgequake/crates/edgequake-pdf/src/layout/
├── pymupdf_grouper.rs    (~619 lines) - Main grouping logic
├── pymupdf_structs.rs    (~611 lines) - Data structures
├── pymupdf_renderer.rs   (~449 lines) - Markdown output
└── reading_order.rs      (~739 lines) - Reading order (partially unused)
```

### Key Functions to Update

1. **`chars_to_spans()`** in `pymupdf_grouper.rs`
   - Current: word boundary at 25% font_size
   - Target: 10% font_size per pymupdf4llm

2. **`lines_to_blocks()`** in `pymupdf_grouper.rs`
   - Current: simple grouping with 20pt gap
   - Target: 3-phase joining algorithm

3. **`sort_blocks_reading_order()`** in `pymupdf_grouper.rs`
   - Current: center-based column detection
   - Target: Smart sort key using left-block Y

---

## Test Evidence

### Character Position Check

```bash
RUST_LOG=debug cargo run -r --bin trace_content -- test.pdf | head -50
```

Shows character-level extraction is accurate - the issue is downstream algorithms.

### Block Count Check

```
AlphaEvolve.pdf:
  Our extraction: 10,016 lines
  Gold standard:  2,547 lines
  Ratio: 4x (severe fragmentation)
```

---

## Next: Orient

Analyze which changes will have highest impact on F1 improvement.
