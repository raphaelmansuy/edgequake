# OODA Iteration 03 - Observe

## Date: 2026-02-04

## Mission Recap
Target: Quality >= 0.95 against pymupdf4llm gold standards
Previous: Word F1 = 0.877 (misleading!)
Actual: Quality = 0.573 (revealed by new metrics)

---

## Observations

### 1. Current F1 Metric is Fundamentally Flawed

The word-set F1 calculation in `scripts/eval_pymupdf_pipeline.py`:

```python
def normalize(text: str) -> set[str]:
    words = text.lower().split()
    words = [w.strip("*_`#[]()") for w in words]
    words = [w for w in words if w and len(w) > 1]
    return set(words)  # <-- FATAL: SET loses order and duplicates!
```

**Problems identified**:
1. `set(words)` discards word ORDER - scrambled text scores same as correct
2. `set(words)` discards DUPLICATES - common words counted once
3. Strips markdown before comparison - formatting not validated
4. Single-char filter may remove important content

### 2. Pipeline Output Analysis

Ran pipeline on AlphaEvolve.pdf and compared to gold:

**Gold (first 50 chars)**:
```
# **AlphaEvolve : A coding agent for scientific and**
```

**Extracted (first 200 chars)**:
```
## **:**

## **agent**

## ***AlphaEvolve*****A**

## **for scientific**

## **and**

## **coding** **algorithmic discovery**
```

**Key issues**:
- Title fragmented into multiple blocks
- Words appear in wrong order
- Extra heading markers (`##` instead of `#`)
- Blank lines and structure completely wrong

### 3. Metric Research (Wikipedia/Literature)

#### ROUGE (Recall-Oriented Understudy for Gisting Evaluation)
- **ROUGE-1**: Unigram overlap (similar to current F1)
- **ROUGE-2**: Bigram overlap (captures word pairs)
- **ROUGE-L**: Longest Common Subsequence (captures ORDER)

ROUGE-L is most relevant for our use case because it:
- Penalizes scrambled text (LCS is shorter)
- Works at word level (not character)
- Gives F1-style score between 0 and 1

#### BLEU (BiLingual Evaluation Understudy)
- N-gram precision with brevity penalty
- Geometric mean of 1-4 gram precisions
- Captures phrase structure and fluency

#### Levenshtein Distance
- Edit distance (insertions, deletions, substitutions)
- Normalized: `1 - (edit_dist / max_len)`
- Lower is more similar, higher when normalized

### 4. Extraction Quality Deep Dive

Ran `convert_pdf_full` on all 7 gold standard files:

| File | Word F1 | But order looks... |
|------|---------|-------------------|
| ccn | 0.930 | Broken - multi-column interleaved |
| 2900_Goyal | 0.904 | Semi-broken - some sections OK |
| agent | 0.889 | Broken - headers fragmented |
| v2 | 0.874 | Broken - abstract scrambled |
| one_tool | 0.857 | Broken - title split |
| 01 | 0.851 | Broken - equations mixed |
| AlphaEvolve | 0.837 | Severely broken - title in 6 pieces |

**Conclusion**: High F1 scores were hiding severe reading order problems.

### 5. Root Cause Identification

The core issue is in block sorting (`sort_blocks_reading_order`):

```rust
// Current algorithm sorts by Y first, then X
blocks.sort_by(|a, b| {
    let y_cmp = a.y0.partial_cmp(&b.y0).unwrap_or(Ordering::Equal);
    if y_cmp != Ordering::Equal {
        return y_cmp;
    }
    a.x0.partial_cmp(&b.x0).unwrap_or(Ordering::Equal)
});
```

This fails for multi-column layouts where blocks at same Y should be read left-to-right within columns, not across columns.

---

## Quantitative Findings

### New Comprehensive Metrics (7 files):

| Metric | Average | Interpretation |
|--------|---------|----------------|
| Quality Score | 0.573 | Overall extraction quality |
| ROUGE-L | 0.491 | Only 49% words in correct order! |
| Word F1 | 0.914 | 91% of words ARE present |
| Structure | 0.295 | Document structure severely broken |
| Format | 0.312 | Markdown formatting inconsistent |

### Gap Analysis

| Metric | Current | Target | Gap |
|--------|---------|--------|-----|
| Quality | 0.573 | 0.95 | **-0.377** |
| ROUGE-L | 0.491 | 0.90 | **-0.409** |
| Word F1 | 0.914 | 0.95 | -0.036 |
| Structure | 0.295 | 0.80 | **-0.505** |
| Format | 0.312 | 0.70 | **-0.388** |

**Key Insight**: Content extraction is mostly working (F1=0.914).
The problem is ORDER (ROUGE-L=0.491) and STRUCTURE (0.295).

---

## Files Examined

1. `scripts/eval_pymupdf_pipeline.py` - Current F1 calculation
2. `scripts/eval_comprehensive.py` - NEW comprehensive metrics (created this iteration)
3. `src/layout/pymupdf_grouper.rs` - Block grouping and sorting
4. `src/backend/pdfium.rs` - Character extraction
5. Gold standards in `test-data/real_dataset/*.pymupdf.gold.md`

---

## Key Questions for Orient Phase

1. Why is block sorting producing scrambled order?
2. Is line grouping merging elements incorrectly?
3. Should we implement pymupdf4llm's column detection algorithm?
4. What tolerances are causing too-aggressive or too-loose grouping?
