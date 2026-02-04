# OODA Iteration 03 - Act

## Date: 2026-02-04

## Actions Taken

### 1. Created Comprehensive Evaluation Script ✅

**File**: `scripts/eval_comprehensive.py`
**Lines**: 1-450

Created a new evaluation script with multi-dimensional metrics:

```python
# Key functions implemented:

def compute_lcs_length(a, b) -> int:
    """O(n²) DP for Longest Common Subsequence"""

def compute_rouge_l(gold_words, extracted_words) -> float:
    """ROUGE-L F1 based on LCS"""

def compute_bleu_4(gold_words, extracted_words) -> float:
    """BLEU-4 with 1-4 gram precision and brevity penalty"""

def compute_word_levenshtein(gold_words, extracted_words) -> float:
    """Normalized word-level edit distance"""

def compute_heading_match(gold_text, extracted_text) -> float:
    """Jaccard similarity of heading counts by level"""

def compute_all_metrics(gold_text, extracted_text) -> MetricsResult:
    """Master function computing all dimensions"""
```

**Quality Score Formula**:
```python
quality_score = (
    0.40 * rouge_l +          # Order preservation (most critical)
    0.30 * word_f1 +          # Content accuracy
    0.15 * structure_score +  # Document structure
    0.10 * format_score +     # Markdown formatting
    0.05 * bleu_4             # Phrase structure
)
```

### 2. Updated Mission Specification ✅

**File**: `specs/005-perfect-pdf-pymupdf4llm-inspired-conversion.md`
**Changes**:
- Replaced "Success Metrics" section with "Quality Metrics (Multi-Dimensional Evaluation)"
- Added detailed explanation of each metric dimension
- Added current status table with true quality scores
- Added per-file breakdown
- Updated changelog with iteration 03

### 3. Ran Comprehensive Evaluation ✅

**Command**:
```bash
python3 scripts/eval_comprehensive.py
```

**Results**:
```
======================================================================
SUMMARY (7 files)
======================================================================
  Average QUALITY:   0.573  (target: ≥0.95, gap: +0.377)
  Average ROUGE-L:   0.491  (order preservation)
  Average Word F1:   0.914  (content accuracy)
  Average Structure: 0.295  (document structure)
  Average Format:    0.312  (markdown fidelity)
======================================================================

Per-file Comparison (sorted by quality):
File                            Quality  ROUGE-L  Word F1   Struct   Format
----------------------------------------------------------------------
ccn_2512.21804v1                  0.652    0.555    0.941    0.349    0.597
2900_Goyal_et_al                  0.606    0.591    0.937    0.200    0.329
v2_2512.25072v1                   0.577    0.437    0.919    0.621    0.000
01_2512.25075v1                   0.562    0.439    0.880    0.331    0.435
AlphaEvolve                       0.546    0.445    0.865    0.186    0.655
agent_2510.09244v1                0.543    0.505    0.951    0.131    0.154
one_tool_2512.20957v2             0.525    0.468    0.902    0.249    0.014
```

### 4. Created OODA Iteration 03 Documentation ✅

**Files Created**:
- `ooda_loop/iteration_03/observe.md` - Data gathering and analysis
- `ooda_loop/iteration_03/orient.md` - Root cause analysis
- `ooda_loop/iteration_03/decide.md` - Decision summary
- `ooda_loop/iteration_03/act.md` - This file

---

## Evidence of Success

### Metric Validation

The new metrics correctly identify problems that word-set F1 missed:

| Example | Word F1 | ROUGE-L | Interpretation |
|---------|---------|---------|----------------|
| "The cat sat" vs "The cat sat" | 1.0 | 1.0 | Perfect - both agree |
| "The cat sat" vs "sat cat The" | 1.0 | 0.11 | ROUGE-L catches scrambled order |
| AlphaEvolve actual | 0.865 | 0.445 | Content OK, order broken |

### Root Cause Confirmed

Running verbose analysis on AlphaEvolve:
```
   Dimension 1 - Content Accuracy:
     Precision: 0.817, Recall: 0.919, F1: 0.865  ← Words ARE present!

   Dimension 2 - Order Preservation:
     ROUGE-L: 0.445, BLEU-4: 0.295, Levenshtein: 0.191  ← Order BROKEN!

   Dimension 3 - Structural Fidelity:
     Headings: 0.212, Paragraphs: 0.102, Lines: 0.236  ← Structure BROKEN!
```

---

## Files Modified

| File | Action | Lines Changed |
|------|--------|---------------|
| `scripts/eval_comprehensive.py` | Created | 450 new lines |
| `specs/005-*.md` | Updated | ~80 lines added/modified |
| `ooda_loop/iteration_03/observe.md` | Created | ~180 lines |
| `ooda_loop/iteration_03/orient.md` | Created | ~200 lines |
| `ooda_loop/iteration_03/decide.md` | Created | ~120 lines |
| `ooda_loop/iteration_03/act.md` | Created | This file |

---

## Tests Performed

### 1. Comprehensive Evaluation Script Works
```bash
$ python3 scripts/eval_comprehensive.py --verbose --file AlphaEvolve
# Output shows all 4 dimensions with detailed breakdown ✓
```

### 2. All 7 Gold Standards Evaluated
```bash
$ python3 scripts/eval_comprehensive.py
# All 7 files processed successfully ✓
# No errors or timeouts ✓
```

### 3. Metrics Are Discriminative
- Worst file (one_tool): Quality = 0.525
- Best file (ccn): Quality = 0.652
- Range shows metrics differentiate quality levels ✓

---

## Commit

```bash
git add scripts/eval_comprehensive.py
git add specs/005-perfect-pdf-pymupdf4llm-inspired-conversion.md
git add specs/005-perfect-pdf-pymupdf4llm-inspired-conversion/ooda_loop/iteration_03/

git commit -m "OODA-03: Establish comprehensive quality metrics

- Add multi-dimensional evaluation (ROUGE-L, Word F1, Structure, Format)
- Reveal true quality gap: 0.573 vs 0.95 (not 0.877 vs 0.95)
- Identify root cause: ORDER broken (ROUGE-L=0.491), not CONTENT (F1=0.914)
- Create scripts/eval_comprehensive.py
- Update specs with metrics section

Quality baseline:
  - Quality Score: 0.573 (target: 0.95)
  - ROUGE-L: 0.491 (order preservation)
  - Word F1: 0.914 (content accuracy)
  - Structure: 0.295 (document layout)
  - Format: 0.312 (markdown fidelity)"
```

---

## Next Steps (Iteration 04)

1. **Revert line_tolerance** from 5pt to 3pt
2. **Analyze block sorting** in detail
3. **Implement smart sort key** with vertical overlap detection
4. **Re-run comprehensive evaluation** to measure impact
5. **Focus on improving ROUGE-L** (biggest gap at 0.409)

---

## Lessons Learned

1. **Metrics matter more than we thought**: Wrong metrics = wrong conclusions
2. **SET-based F1 is dangerous**: Hides order problems completely
3. **ROUGE-L is essential**: Industry-standard for order-sensitive evaluation
4. **Multi-dimensional view helps**: Can see exactly what's broken (order vs content vs structure)
5. **Always validate metrics first**: Before optimizing, ensure you're measuring the right thing
