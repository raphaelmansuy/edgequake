# OODA-27 Observe: Analysis of Current PDF Extraction Quality

## Re-read Mission Status ✅

Re-read `specs/004-perfect-pdf-markdown-conversion.md` at iteration start.

## Current Quality Metrics

From comprehensive test run:

- **Text Preservation: 81.4%** (target: ≥98%)
- **Structural Fidelity: 80.2%** (target: ≥95%)
- **Overall Quality: 80.8%** (target: 95%+)

This is an improvement from OODA-26 (was 74.6%), but still 14.2% below target.

## New Test Documents Analysis

Analyzed new PDFs in `zz_test_docs/`:

- `stackplanner_2601.05890v1.pdf` - arXiv two-column paper (1.1MB)
- `kvzap_2601.07891v1.pdf` - arXiv paper (5.1MB)
- `lightrag_2410.05779v3.pdf` - arXiv paper (1.1MB)
- `paper_banana_2601.23265v1.pdf` - Large paper (42.5MB)
- French car manuals (6-10MB each)
- Business documents

## Markitdown Comparison

### Markitdown Output (stackplanner)

```
6
2
0
2

n
a
J
...
```

Markitdown produced **fragmented, character-by-character output** for arXiv papers with:

- Individual characters on separate lines
- Wrong reading order (showing arXiv date header fragments)
- Table data appearing scattered

### Our EdgeQuake Output (stackplanner)

```markdown
## STACKPLANNER: A Centralized Hierarchical Multi-Agent System...

Ruizhe Zhang, Xinke Jiang, Zhibang Yang...

### Abstract

Multi-agent systems based on large language models...
```

**Our output is significantly better than markitdown** for academic papers:

- Title and authors correctly extracted
- Abstract identified
- Two-column reading order mostly working

## Key Issues Observed

### 1. Text Truncation in Column Merging

In the output, some lines appear truncated:

```
memory control. STACKPLANNER addresses

task-level memory control, and by learning to
```

Middle content is missing between these lines.

### 2. Incomplete Sentence Fragments

```
By enabling task decomposition, parallel exploration,
```

(No continuation)

### 3. Some Reading Order Issues

Right column content occasionally interleaves with left:

```
...Crucially, both issues stem f
rom
memory. Addressing this deficiency gives rise to two
```

## Architecture Analysis

### Current Pipeline Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│ PDF Binary                                                           │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ ContentParser (content_parser.rs)                                    │
│   - Parses PDF operators (Tm, Tf, Tj, TJ, etc.)                      │
│   - Extracts text elements with X, Y, font_size                      │
│   - Groups runs of text by font/position                             │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ Column Detection (column_detection.rs)                               │
│   - Peak detection method: finds left/right column margins          │
│   - Fallback: gap detection in projection histogram                 │
│   - Returns column boundary X-coordinate                             │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ Text Grouping (text_grouping.rs)                                     │
│   - Separates elements into left/right columns                       │
│   - Groups elements into lines by Y-position                         │
│   - Merges lines with proper spacing                                 │
│   - Outputs column-first reading order                               │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ Extraction Engine (extraction_engine.rs)                             │
│   - Coordinates column detection + text grouping                     │
│   - Handles Y-normalization                                          │
│   - Creates blocks with text + bounding boxes                        │
│   - OODA-12: Skip Y-sort for 2-column pages                          │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ Processors (layout_processing.rs)                                    │
│   - BlockMergeProcessor: Merge adjacent blocks                       │
│   - OODA-23: Cross-column hyphenation merge                          │
│   - OODA-26: Preserve reading order (no Y-sort)                      │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│ Markdown Renderer (renderers/markdown.rs)                            │
│   - Converts blocks to markdown syntax                               │
│   - Handles headers, lists, code blocks                              │
│   - Joins paragraphs with proper spacing                             │
└─────────────────────────────────────────────────────────────────────┘
```

### Markitdown Architecture (from GitHub research)

Markitdown uses pdfplumber with a simpler approach:

1. Word-level extraction with `page.extract_words()`
2. Row grouping by Y-position (5pt tolerance)
3. Form detection: checks if rows have 3+ columns with aligned X-positions
4. Table vs paragraph detection: line_width > 55% of page = paragraph
5. Fallback to pdfminer for plain text documents

**Key differences:**

- Markitdown has NO column detection for academic papers
- Falls back to pdfminer for multi-column text (which produces fragmented output)
- Our approach is **fundamentally better** for academic papers

## Root Cause Analysis

### Issue 1: Text Truncation Source

Looking at `text_grouping.rs`, the `group_two_column_layout()` function:

1. Separates elements by column boundary
2. Filters elements into left/right columns
3. Some elements near the boundary may be misclassified

The 15pt margin around boundary:

```rust
let margin = 15.0;
```

Elements in the margin zone may be assigned incorrectly or dropped.

### Issue 2: Line Merging Gaps

In `merge_line_elements()`, lines are grouped by Y-position with a tolerance.
If a line spans across the column boundary, it may be split and incomplete.

### Issue 3: Block Merging Logic

The `BlockMergeProcessor::can_merge()` has many conditions that may reject valid merges:

- Font size difference > 1pt
- Horizontal zone difference
- Gap between blocks too large

## Markitdown Best Practices to Adopt

From markitdown's PDF converter:

1. **Word-level tolerance:** Uses `x_tolerance=3, y_tolerance=3` for word extraction
2. **Row grouping:** Uses 5pt Y-tolerance for grouping words into rows
3. **Table detection:** Checks for 20%+ table rows before treating as form
4. **Paragraph detection:** line_width > 55% of page = paragraph, not table

## Next Steps for Orient Phase

1. Analyze specific text truncation patterns in stackplanner output
2. Compare our line merging vs markitdown's simpler word concatenation
3. Investigate if elements are being filtered during column separation
4. Check if block merging is rejecting valid paragraph continuations
