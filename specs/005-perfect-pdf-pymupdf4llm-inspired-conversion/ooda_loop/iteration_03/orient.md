# OODA Iteration 03 - Orient

## Date: 2026-02-04

## Analysis of Findings

### The Metric Problem

The original F1 score was fundamentally broken for PDF extraction evaluation:

```
Original Approach:
- Extract words as SET
- Compute |gold ∩ extracted| / |gold| (recall)
- Compute |gold ∩ extracted| / |extracted| (precision)
- F1 = harmonic mean

Why it fails:
- SET ignores word ORDER → "cat sat" = "sat cat" = 1.0
- SET ignores DUPLICATES → "the the" = "the" = 1.0
- Strips markdown → bold/italic not validated
```

**Impact**: Gave us 0.877 when real quality was 0.573 (43% overestimation!)

### Why ROUGE-L is the Right Metric

ROUGE-L (Longest Common Subsequence) directly captures reading order:

```
Gold:     "The quick brown fox jumps over the lazy dog"
Good:     "The quick brown fox jumps over the lazy dog"  → LCS=9, ROUGE-L=1.0
Bad:      "dog lazy the over jumps fox brown quick The"  → LCS=1, ROUGE-L=0.11

SET F1 for both: 1.0 (all words present!)
ROUGE-L correctly penalizes the scrambled version.
```

### Root Cause Analysis

**Why is order broken?**

1. **Title Fragmentation** (AlphaEvolve example):
   - Gold: `# **AlphaEvolve : A coding agent for scientific and algorithmic discovery**`
   - Extracted: Title split into 6+ separate blocks due to:
     - Font changes (bold, italic)
     - Colon creating separate span
     - Words on slightly different Y positions

2. **Block Sorting Algorithm**:
   - Current: Sort by (y0, x0)
   - Problem: Doesn't handle multi-column layouts
   - Solution: Need pymupdf4llm's "smart sort key" with vertical overlap detection

3. **Line Tolerance**:
   - Current: 5pt (increased from 3pt, caused regression)
   - Problem: Too tight = fragmentation, too loose = merging
   - Need: Dynamic tolerance based on font size

4. **Missing Column Detection**:
   - pymupdf4llm uses `column_boxes()` to detect columns
   - Then reads within columns left-to-right
   - Our implementation doesn't detect columns

### Priority Analysis

| Issue | Impact on ROUGE-L | Effort | Priority |
|-------|-------------------|--------|----------|
| Block sorting | High | Medium | 1 |
| Line grouping tolerance | High | Low | 2 |
| Column detection | High | High | 3 |
| Markdown formatting | Medium | Medium | 4 |
| Span merging | Medium | Low | 5 |

### Comparison with pymupdf4llm

| Feature | pymupdf4llm | Our Implementation |
|---------|-------------|-------------------|
| Column detection | `column_boxes()` with join phases | None |
| Block sorting | Smart sort key with vertical overlap | Simple (y0, x0) |
| Line tolerance | Dynamic based on font | Fixed 5pt |
| Space handling | Built into pymupdf | Synthesized (fixed) |
| Reading order | Column-aware | Column-unaware |

### What We Need to Fix

1. **Implement Column Detection**:
   ```
   Phase 1: Vertical join (10pt tolerance)
   Phase 2: Boundary normalization (3pt)
   Phase 3: Smart sort key (vertical overlap)
   ```

2. **Fix Block Sorting**:
   ```rust
   // Current (broken for multi-column):
   sort by (y0, x0)
   
   // Needed (column-aware):
   for each block Q:
     find P = leftmost block with vertical overlap
     sort_key = (P.y0, Q.x0)  // ensures Q after P
   ```

3. **Tune Line Tolerance**:
   - Revert to 3pt (line_tolerance = 3.0)
   - Consider font-relative: `tolerance = 0.3 * font_size`

### Strategic Decision

**Focus on ORDER first (ROUGE-L)** because:
1. Content extraction is working (Word F1 = 0.914)
2. Order is the biggest gap (-0.409)
3. Structure follows from correct ordering
4. Formatting can be addressed last

### Proposed Solution Architecture

```
              ┌─────────────────────────────────┐
              │  PDF Input                      │
              └─────────────────────────────────┘
                              │
                              ▼
              ┌─────────────────────────────────┐
              │  PdfiumBackend.extract_chars()  │
              │  (Current - working well)       │
              └─────────────────────────────────┘
                              │
                              ▼
              ┌─────────────────────────────────┐
              │  chars_to_spans()               │
              │  (Current - mostly OK)          │
              └─────────────────────────────────┘
                              │
                              ▼
              ┌─────────────────────────────────┐
              │  spans_to_lines()               │
              │  FIX: Revert to 3pt tolerance   │
              └─────────────────────────────────┘
                              │
                              ▼
              ┌─────────────────────────────────┐
              │  lines_to_blocks()              │
              │  (Current - mostly OK)          │
              └─────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  NEW: column_boxes()                                        │
│  - Phase 1: Vertical join (10pt)                           │
│  - Phase 2: Boundary normalization (3pt)                   │
│  - Phase 3: Smart sort key with vertical overlap           │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
              ┌─────────────────────────────────┐
              │  Markdown Rendering             │
              │  (Current - working)            │
              └─────────────────────────────────┘
```

### Metrics Framework Established

Created `scripts/eval_comprehensive.py` with:
- **Quality Score**: 0.4×ROUGE-L + 0.3×Word_F1 + 0.15×Structure + 0.1×Format + 0.05×BLEU
- Multi-dimensional breakdown for debugging
- Per-file and aggregate reporting

This gives us visibility into what's actually broken.

---

## Recommendations for Decide Phase

1. **Immediate**: Revert line_tolerance from 5pt to 3pt
2. **Short-term**: Implement smart sort key with vertical overlap
3. **Medium-term**: Implement full column detection (3 phases)
4. **Ongoing**: Use comprehensive metrics for all evaluations
