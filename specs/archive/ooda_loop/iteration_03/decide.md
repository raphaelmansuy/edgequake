# OODA Iteration 03 - Decide

## Date: 2026-02-04

## Decision Summary

This iteration focused on **establishing correct quality metrics** rather than code changes. The key decision was to replace the misleading word-set F1 with a comprehensive multi-dimensional quality score.

---

## Decisions Made

### Decision 1: Implement Comprehensive Quality Metrics ✅

**Why**: The original word-set F1 (0.877) was hiding the real problem. We needed metrics that:
1. Capture word ORDER (not just presence)
2. Capture document STRUCTURE
3. Capture markdown FORMATTING
4. Give a single composite QUALITY score

**Chosen Approach**: Multi-dimensional evaluation
```
Quality = 0.40×ROUGE-L + 0.30×Word_F1 + 0.15×Structure + 0.10×Format + 0.05×BLEU-4
```

**Rationale**:
- ROUGE-L (40%): Most important because ORDER is the core problem
- Word F1 (30%): Content accuracy matters but is already high
- Structure (15%): Headings, paragraphs, lines
- Format (10%): Bold, italic, lists
- BLEU-4 (5%): Phrase structure refinement

### Decision 2: Use ROUGE-L as Primary Order Metric ✅

**Why**: ROUGE-L (Longest Common Subsequence) is the gold standard for measuring text order:
- Published, peer-reviewed metric
- Captures longest in-order match
- F1-style score between 0 and 1
- Widely used in NLP evaluation

**Alternatives Considered**:
- Levenshtein: Good for character-level, heavy for word-level
- BLEU: Designed for translation, includes brevity penalty
- Jaccard: Same problem as SET F1 (ignores order)

### Decision 3: Defer Code Changes to Next Iteration

**Why**: Needed to establish correct metrics FIRST to:
1. Understand the true quality gap (0.377 not 0.073)
2. Identify what's actually broken (order, not content)
3. Have a reliable measurement for future changes

**Next Iteration Will**:
1. Revert line_tolerance from 5pt to 3pt
2. Implement proper smart sort key algorithm
3. Add column detection if needed

---

## Action Plan for Act Phase

| Action | File | Description |
|--------|------|-------------|
| Create eval script | `scripts/eval_comprehensive.py` | Multi-dimensional metrics |
| Update mission spec | `specs/005-*.md` | Document new metrics |
| Create OODA files | `ooda_loop/iteration_03/` | Document findings |
| Run evaluation | All 7 gold standards | Establish new baseline |

---

## Success Criteria for This Iteration

1. ✅ Comprehensive evaluation script created
2. ✅ All 7 files evaluated with new metrics
3. ✅ Mission spec updated with metrics section
4. ✅ Gap correctly identified (0.377 not 0.073)
5. ✅ Root cause identified (order, not content)

---

## Deferred Decisions

The following decisions are deferred to iteration 04:

1. **Line Tolerance Tuning**: Should we use 3pt, 5pt, or dynamic?
2. **Column Detection Algorithm**: Full implementation vs simplified
3. **Block Sorting Algorithm**: pymupdf4llm exact port vs adaptation
4. **Space Character Handling**: Further improvements needed?

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| ROUGE-L computation slow | Low | Low | O(n²) is fine for short texts |
| Metrics not predictive | Medium | High | Validate against manual review |
| Over-optimization to metrics | Medium | Medium | Keep multiple dimensions |

---

## Commit Message Template

```
OODA-03: Establish comprehensive quality metrics

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
  - Format: 0.312 (markdown fidelity)
```
