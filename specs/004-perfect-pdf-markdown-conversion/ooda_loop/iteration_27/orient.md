# OODA-27 Orient: Root Cause Analysis with First Principles

## Re-read Mission Status ✅

Re-read `specs/004-perfect-pdf-markdown-conversion.md` at iteration start.

## Key Insight from Markitdown Analysis

After deep analysis of markitdown's PDF converter (`_pdf_converter.py`), I identified **fundamental architectural differences** between their approach and ours:

### Markitdown Architecture (Simple & Effective for Forms)

```
┌────────────────────────────────────────────────────────────────┐
│ PDF Page                                                        │
└─────────────────────────┬──────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ pdfplumber.extract_words()                                      │
│   - x_tolerance=3, y_tolerance=3 (tight)                        │
│   - Returns word-level bounding boxes                           │
└─────────────────────────┬──────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ Row Grouping (Y-position based)                                 │
│   - Group by round(word.top / 5) * 5  (5pt tolerance)           │
│   - Sort rows by Y                                              │
│   - Sort words within row by X                                  │
└─────────────────────────┬──────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ Column Detection (Gap-based)                                    │
│   - Find x-positions with gaps > 50pt                           │
│   - Detect 3-8 columns → table                                  │
│   - >8 columns → dense text, skip                               │
└─────────────────────────┬──────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ Row Classification                                              │
│   - line_width > 55% of page + >60 chars → paragraph            │
│   - 20%+ table rows → form-style document                       │
│   - <20% table rows → plain text (use pdfminer fallback)        │
└─────────────────────────┬──────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ Output                                                          │
│   - Tables: pipe-separated markdown                             │
│   - Text: plain concatenation                                   │
│   - Fallback: pdfminer.high_level.extract_text()                │
└────────────────────────────────────────────────────────────────┘
```

### Why Markitdown Fails on Academic Papers

Markitdown produces **fragmented character output** for multi-column academic papers because:

1. **No column detection algorithm**: Uses gap-based table detection, not reading order
2. **pdfminer fallback is line-by-line**: Returns text without understanding 2-column layout
3. **Designed for forms**: 55% page width = paragraph, but academic columns are ~45% each

### Our EdgeQuake Architecture (Better for Academic Papers)

```
┌────────────────────────────────────────────────────────────────┐
│ PDF Page                                                        │
└─────────────────────────┬──────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ lopdf + ContentParser                                           │
│   - Direct PDF operator parsing (Tm, Tf, Tj, TJ)                │
│   - Character-level positioning                                 │
│   - Font metrics and encoding handling                          │
└─────────────────────────┬──────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ Column Detection (Peak Analysis)                                │
│   - X-position histogram projection                             │
│   - Gap detection for column boundary                           │
│   - Returns column boundary X-coordinate                        │
└─────────────────────────┬──────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ Text Grouping (Reading Order)                                   │
│   - Left column first, then right                               │
│   - Line-by-line within each column                             │
│   - Y-tolerance based line merging                              │
└─────────────────────────┬──────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ Block Merging & Processing                                      │
│   - Merge adjacent blocks                                       │
│   - Cross-column hyphenation handling (OODA-23)                 │
│   - Structure detection (headers, lists, etc.)                  │
└─────────────────────────┬──────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ Markdown Renderer                                               │
│   - Converts blocks to markdown                                 │
│   - Headers, lists, code blocks                                 │
└────────────────────────────────────────────────────────────────┘
```

## Root Cause Analysis

### Issue 1: Text Truncation in Columns

**Symptom:** Lines appear incomplete:

```
memory control. STACKPLANNER addresses

task-level memory control, and by learning to
```

**Root Cause Analysis Using First Principles:**

The PDF coordinate system has Y increasing upward, but PDF content streams output in reading order (top-to-bottom). Our Y-normalization may be inverting the order.

Looking at `text_grouping.rs`:

```rust
// Column boundary margin: elements within 15pt of boundary are edge cases
let margin = 15.0;

// Elements near boundary may be filtered or misclassified
if element.x >= col_boundary - margin && element.x <= col_boundary + margin {
    // What happens here?
}
```

**Hypothesis:** Elements near the column boundary (within 15pt) may be:

1. Dropped entirely
2. Assigned to the wrong column
3. Causing text fragments to be lost

### Issue 2: Markitdown's Simplicity as a Feature

**First Principles:** Markitdown succeeds on forms because:

1. Forms have consistent columnar structure
2. Word-level extraction is sufficient (no character positioning needed)
3. 20% threshold filters noise effectively

**But fails on academic papers because:**

1. Two-column layout ≠ table with 2 columns
2. pdfminer fallback doesn't understand reading order
3. No hyphenation handling across columns

### Issue 3: Our Quality Gap (80.8% vs 95% target)

Breaking down the gap:

- **Text Preservation: 81.4%** → 16.6% of text is lost somewhere
- **Structural Fidelity: 80.2%** → 14.8% of structure is missed

**Where is text lost?**

1. Column boundary filtering (margin zone)
2. Font encoding failures (CID fonts)
3. Block merging rejection (font size, position thresholds)

**Where is structure lost?**

1. Table detection false negatives
2. Header detection failures
3. List detection failures

## Critical Gap: Fast Quality Tests

**Problem:** Current comprehensive tests take 118s, which is too long for iterative development.

**First Principles Solution:** Create micro-benchmark tests that:

1. Test ONE specific quality aspect
2. Use ONE small PDF (< 100KB)
3. Complete in < 1 second
4. Provide actionable metrics

### Proposed Fast Quality Metrics

| Metric            | PDF                         | Size   | Target Time |
| ----------------- | --------------------------- | ------ | ----------- |
| Text Preservation | AI_Services\_\_Elitizon.pdf | 110KB  | < 200ms     |
| Column Reading    | stackplanner excerpt        | 50KB   | < 200ms     |
| Table Detection   | existing lattice test       | 20KB   | < 100ms     |
| Font Encoding     | Qwen.pdf (first page)       | varies | < 300ms     |

## Decisions for Act Phase

1. **Create fast quality metric test** (highest priority per user request)
2. **Fix column boundary filtering** - investigate 15pt margin zone
3. **Use markitdown as gold baseline** for simple documents
4. **Preserve our advantage** for multi-column academic papers

## Algorithm Complexity Analysis (O-notation)

| Component        | Current     | Target      | Notes                                 |
| ---------------- | ----------- | ----------- | ------------------------------------- |
| Word extraction  | O(n)        | O(n)        | n = characters, linear is optimal     |
| Column detection | O(w)        | O(w)        | w = words, histogram projection       |
| Row grouping     | O(w·log(w)) | O(w·log(w)) | Sorting by Y-position                 |
| Line merging     | O(l²)       | O(l)        | l = lines, can optimize               |
| Block merging    | O(b²)       | O(b)        | b = blocks, can optimize with spatial |

**Current bottleneck:** Line merging is O(l²) due to pairwise comparison. For large documents (1000+ lines), this becomes slow.

## Summary

**Root Causes:**

1. Column boundary margin (15pt) may filter valid text
2. Block merging conditions are too strict
3. Tests are too slow for iterative development

**Key Actions:**

1. Build fast quality test using AI_Services\_\_Elitizon.pdf (known good markitdown output)
2. Investigate column boundary handling in text_grouping.rs
3. Optimize line merging from O(l²) to O(l)
