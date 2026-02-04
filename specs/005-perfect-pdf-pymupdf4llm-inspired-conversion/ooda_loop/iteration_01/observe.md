# OODA-01: Observe

## Mission Re-Read ✅

**File**: `specs/005-perfect-pdf-pymupdf4llm-inspired-conversion.md`
**Goal**: F1 ≥ 0.95 by implementing pymupdf4llm algorithms with PyMuPDF backend

---

## Current State Analysis

### Baseline Metrics

```
Average F1: 0.685
Per-document:
  01_2512.25075v1: F1=0.552  (WORST)
  one_tool_2512.20957v2: F1=0.596
  AlphaEvolve: F1=0.620
  v2_2512.25072v1: F1=0.689
  2900_Goyal_et_al: F1=0.722
  ccn_2512.21804v1: F1=0.807
  agent_2510.09244v1: F1=0.810  (BEST)
```

### Root Cause: Text Position Accuracy

**Evidence from debugging session:**

```
pymupdf extraction:
  Y=275.7 X=679.4 text=' can'  (LEFT column)

our lopdf extraction:
  Y=316.6 X=580.3 text=' can'  (incorrectly placed in RIGHT column)
```

The lopdf-based extraction produces WRONG X/Y coordinates due to:

1. Complex CTM matrix handling
2. Font glyph width estimation (uses 55% of font size, not actual widths)
3. Text matrix accumulation errors

### pymupdf4llm Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    pymupdf4llm Pipeline                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  page.get_text("dict")                                          │
│         │                                                       │
│         ▼                                                       │
│  ┌─────────────────┐                                            │
│  │  get_raw_lines  │  ← Line grouping with tolerance=3pt        │
│  │  (get_text_     │  ← sanitize_spans() joins broken text      │
│  │   lines.py)     │  ← Y-delta check: |y1-y1'| <= 3 OR         │
│  └────────┬────────┘    |y0-y0'| <= 3                           │
│           │                                                     │
│           ▼                                                     │
│  ┌─────────────────┐                                            │
│  │  column_boxes   │  ← 3-phase rectangle joining               │
│  │  (multi_column  │  ← Phase 1: vertical join (10pt gap)       │
│  │   .py)          │  ← Phase 2: boundary normalize (3pt)       │
│  └────────┬────────┘  ← Phase 3: smart sort key                 │
│           │                                                     │
│           ▼                                                     │
│  ┌─────────────────┐                                            │
│  │  to_markdown    │  ← Header detection via font histogram     │
│  │  (document_     │  ← Table/list/code detection               │
│  │   layout.py)    │  ← Style preservation (bold/italic)        │
│  └─────────────────┘                                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Key Files in pymupdf4llm

| File                 | Lines | Purpose                            |
| -------------------- | ----- | ---------------------------------- |
| `get_text_lines.py`  | 304   | `get_raw_lines()` - line grouping  |
| `multi_column.py`    | 531   | `column_boxes()` - 3-phase joining |
| `document_layout.py` | 1182  | `to_markdown()` - final rendering  |
| `pymupdf_rag.py`     | 1377  | Fallback RAG extraction            |
| `utils.py`           | 1158  | Helper functions                   |

### Key Algorithm: Line Grouping (get_raw_lines)

```python
# Lines 156-159 in get_text_lines.py
# Two spans are on same line if EITHER tops OR bottoms are close
if abs(sbbox.y1 - sbbox0.y1) <= y_delta or abs(sbbox.y0 - sbbox0.y0) <= y_delta:
    line.append(s)  # same line
```

**Our bug:** We only check single Y coordinate with loose tolerance (font_size \* 0.5).

### Key Algorithm: Span Joining (sanitize_spans)

```python
# Lines 79-90 in get_text_lines.py
# Join spans if gap < 10% of font size AND same style
delta = s1["size"] * 0.1
if s0["bbox"].x1 + delta < s1["bbox"].x0:
    continue  # don't join - gap too large
```

**Our bug:** We use fixed threshold, not font-size-relative.

### Key Algorithm: Smart Sort Key (Phase 3)

```python
# Lines 283-298 in multi_column.py
# For each block, find left-most block with vertical overlap
# Use (left_block.y0, current.x0) as sort key
left_rects = sorted([
    r for r in new_rects
    if r.x1 < box.x0  # to the left
    and (box.y0 <= r.y0 <= box.y1 or box.y0 <= r.y1 <= box.y1)  # vertical overlap
], key=lambda r: r.x1)

if left_rects:
    key = (left_rects[-1].y0, box.x0)  # use left block's Y
else:
    key = (box.y0, box.x0)  # use own Y
```

---

## Current Codebase Structure

```
edgequake/crates/edgequake-pdf/src/
├── backend/
│   ├── content_parser.rs    # 659 lines - lopdf text extraction
│   ├── extraction_engine.rs # 1303 lines - main extraction
│   ├── text_grouping.rs     # 1376 lines - line merging
│   └── block_builder.rs     # ~400 lines - block construction
├── layout/
│   ├── mod.rs               # 333 lines - LayoutAnalyzer
│   ├── reading_order.rs     # 739 lines - reading order detection
│   └── column_detector.rs   # ~300 lines - DBSCAN clustering
├── processors/
│   └── layout_processing.rs # 1388 lines - pipeline processors
└── lib.rs                   # Main exports
```

### Dependencies

```toml
# Cargo.toml
lopdf = "0.34"  # PDF parsing (low-level)
# No pymupdf - would need Python FFI or subprocess
```

---

## Python Environment Check

```bash
$ python3 -c "import pymupdf; print(pymupdf.version)"
# Need to verify pymupdf is available
```

---

## Next Steps (Orient phase)

1. Verify pymupdf is installed and accessible
2. Design PyMuPDF backend interface
3. Plan migration path from lopdf to pymupdf
4. Identify which algorithms to port to Rust
