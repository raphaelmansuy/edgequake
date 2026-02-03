# OODA-21 Observe: Markitdown Pattern Analysis

## Date: 2025-02-03

## Current Baseline Quality

| Metric              | Current | Target | Gap    |
| ------------------- | ------- | ------ | ------ |
| Text Preservation   | 81.3%   | 98%    | -16.7% |
| Structural Fidelity | 80.3%   | 95%    | -14.7% |
| Overall Quality     | 80.8%   | 95%+   | -14.2% |

## Markitdown PDF Conversion Architecture Analysis

From source code analysis of `microsoft/markitdown`:

```
┌─────────────────────────────────────────────────────────────────────┐
│                   MARKITDOWN PDF EXTRACTION FLOW                     │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   PDF File                                                           │
│       │                                                              │
│       ▼                                                              │
│   ┌─────────────────┐                                               │
│   │   pdfplumber    │  ← Primary extraction engine                  │
│   │  (word-level)   │                                               │
│   └────────┬────────┘                                               │
│            │                                                         │
│       ┌────▼────┐                                                   │
│       │ Is Form │ → YES → _extract_form_content_from_words()       │
│       │ Style?  │         (table-like structured content)           │
│       └────┬────┘                                                   │
│            │ NO                                                      │
│            ▼                                                         │
│   ┌─────────────────┐                                               │
│   │    pdfminer     │  ← Fallback for text-heavy documents         │
│   │ (extract_text)  │                                               │
│   └────────┬────────┘                                               │
│            │                                                         │
│            ▼                                                         │
│   ┌─────────────────┐                                               │
│   │ Post-process    │  ← Merge partial numbering lines             │
│   │ (MasterFormat)  │                                               │
│   └────────┬────────┘                                               │
│            │                                                         │
│            ▼                                                         │
│       Markdown Output                                                │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Key Insights from Markitdown Source

### 1. Word-Level Extraction Strategy

Markitdown uses `page.extract_words(keep_blank_chars=True, x_tolerance=3, y_tolerance=3)`:

```python
# Key parameters:
# - keep_blank_chars=True: Preserves spacing
# - x_tolerance=3: Horizontal grouping tolerance (tight)
# - y_tolerance=3: Vertical grouping tolerance (tight)
```

**WHY THIS MATTERS:** Our current extraction uses lopdf with different grouping parameters. The tight tolerances (3px) provide better word-level precision.

### 2. Table Detection Algorithm (Form-Style)

Markitdown's table detection in `_extract_form_content_from_words()`:

```
┌─────────────────────────────────────────────────────────────┐
│           MARKITDOWN TABLE DETECTION FLOW                    │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Group words by Y position (rows)                        │
│     y_tolerance = 5px                                        │
│                                                              │
│  2. Analyze each row:                                        │
│     - Count distinct x-position groups (columns)            │
│     - Determine if paragraph (width > 55% page, len > 60)   │
│     - Check for partial numbering (.1, .2)                  │
│                                                              │
│  3. Build global column boundaries:                          │
│     - Collect ALL x-positions from rows with 3+ columns     │
│     - Cluster with 30px tolerance                           │
│     - Max 8 columns (more = dense text, not table)         │
│                                                              │
│  4. Classify rows as table or text:                          │
│     - Row uses 2+ established columns → table row           │
│     - Paragraphs and partial numbering → text               │
│                                                              │
│  5. Find table regions (consecutive table rows)              │
│     - Need 20%+ of rows to be table rows                    │
│                                                              │
│  6. Format as aligned markdown tables                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3. Critical Thresholds in Markitdown

| Threshold           | Value | Purpose                    |
| ------------------- | ----- | -------------------------- |
| y_tolerance         | 5px   | Row grouping               |
| x_tolerance_col     | 20px  | Column boundary clustering |
| Page width ratio    | 55%   | Paragraph detection        |
| Min paragraph chars | 60    | Paragraph classification   |
| Max columns         | 8     | Dense text detection       |
| Column alignment    | 40px  | Word-to-column assignment  |
| Min table rows      | 3     | Validate table quality     |
| Max long cell ratio | 30%   | Reject prose-as-table      |
| Min table row ratio | 20%   | Require enough structure   |

### 4. Post-Processing: Partial Numbering Merge

```python
PARTIAL_NUMBERING_PATTERN = re.compile(r"^\.\d+$")
# Handles: ".1", ".2", ".10" from MasterFormat
# Merges with following text line
```

## Comparison: Edgequake vs Markitdown

| Feature          | Edgequake                       | Markitdown                        | Notes                |
| ---------------- | ------------------------------- | --------------------------------- | -------------------- |
| Backend          | lopdf (Rust)                    | pdfplumber+pdfminer (Python)      | Different parsing    |
| Word grouping    | text_grouping.rs                | pdfplumber extract_words          | Similar approach     |
| Table detection  | lattice.rs + table_detection.rs | \_extract_form_content_from_words | Different algorithms |
| Column detection | column_detection.rs             | X-position clustering             | Similar concept      |
| Fallback         | None                            | pdfminer for text-heavy           | We need this         |

## Current Edgequake Table Detection Issues

Reviewing `table_detection.rs` and `lattice.rs`:

### Issue 1: Over-Aggressive Table Detection

From test output:

```
OODA-34: Skipping table detection for 2-column page 1 (preserving reading order)
```

The code is already skipping table detection on 2-column pages, but this might be too conservative.

### Issue 2: Column Detection Boundary

From output:

```
FIGURE->LEFT: X=54.9 boundary=295.0-15=280.0
```

The 15px margin is hardcoded. First Principles: This should be based on document characteristics.

### Issue 3: Block Merging Destroying Structure

From output showing before/after merge:

```
BMP-BEFORE-MERGE PAGE1 (68 blocks, 2 columns)
BMP-AFTER-MERGE PAGE1 (26 blocks)
```

68 → 26 blocks = 62% reduction. Some structure may be lost in merging.

## Research: Academic Paper Layout Characteristics

From analyzing extracted output patterns:

```
┌─────────────────────────────────────────────────────────────┐
│           TYPICAL 2-COLUMN ACADEMIC PAPER LAYOUT             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                     TITLE                            │   │
│  │                   AUTHORS                            │   │
│  │                 AFFILIATIONS                         │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌─────────────────────┐   ┌─────────────────────┐         │
│  │                     │   │                     │         │
│  │    LEFT COLUMN      │   │    RIGHT COLUMN     │         │
│  │                     │   │                     │         │
│  │  • Abstract         │   │  • Continues...     │         │
│  │  • Introduction     │   │                     │         │
│  │  • Methods          │   │  • Figures in-line  │         │
│  │                     │   │                     │         │
│  └─────────────────────┘   └─────────────────────┘         │
│                                                              │
│  Page width: ~612pt (8.5")                                  │
│  Gutter: ~20-30pt between columns                           │
│  Margin: ~55pt left/right                                   │
│  Column width: ~240-250pt each                              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Files to Investigate

| File                              | Purpose                   | Lines |
| --------------------------------- | ------------------------- | ----- |
| src/backend/extraction_engine.rs  | Main extraction           | ~700  |
| src/backend/text_grouping.rs      | Word → block grouping     | ~920  |
| src/backend/column_detection.rs   | Column boundary detection | ~200  |
| src/backend/lattice.rs            | Table line detection      | ~400  |
| src/processors/table_detection.rs | Table cell extraction     | ~500  |
| src/backend/layout_processing.rs  | Layout analysis           | ~800  |

## Immediate Opportunities

1. **Improve word-level precision** - Match markitdown's tight tolerances
2. **Add paragraph detection** - Don't treat paragraphs as table rows
3. **Global column boundary analysis** - Like markitdown's approach
4. **Post-processing for partial numbering** - Handle MasterFormat-style
5. **Fallback extraction mode** - For text-heavy documents

## Next Steps

1. Read current text_grouping.rs to understand word grouping parameters
2. Read column_detection.rs for current threshold logic
3. Identify specific parameter changes to improve quality
