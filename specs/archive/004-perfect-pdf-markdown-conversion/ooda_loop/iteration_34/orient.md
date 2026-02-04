# OODA Iteration 34 - Orient Phase

## Quality Assessment After Test File Regeneration

We discovered that the test files were stale. After regeneration, here are the actual quality metrics:

### Current Quality Baseline (Feb 4, 2026)

| PDF                   | F1 Score  | Precision | Recall | Notes                    |
| --------------------- | --------- | --------- | ------ | ------------------------ |
| agent_2510.09244v1    | **0.957** | 0.987     | 0.928  | ✅ Excellent             |
| 2900_Goyal_et_al      | **0.943** | 0.921     | 0.967  | ✅ Excellent             |
| v2_2512.25072v1       | **0.939** | 0.969     | 0.911  | ✅ Excellent             |
| ccn_2512.21804v1      | **0.931** | 0.986     | 0.883  | ✅ Great                 |
| 01_2512.25075v1       | 0.853     | 0.956     | 0.770  | Good - recall issue      |
| one_tool_2512.20957v2 | 0.753     | 0.670     | 0.861  | ⚠️ Precision issue       |
| AlphaEvolve           | 0.563     | 0.395     | 0.981  | ❌ Major precision issue |

**Average F1: 0.848 (84.8%)** - Up from 81.3% TPS!

## Analysis of Low-Scoring Documents

### AlphaEvolve (F1=0.563, Precision=0.395)

**Key Issue**: Very low precision (39.5%) means we're extracting too much text that doesn't match gold standard.

**Probable Causes**:

1. Document has 44 pages - very large, lots of figure captions and references
2. Many "camel_join=286" - camelCase words getting joined incorrectly
3. Double_space=216 - extra spaces being inserted
4. arXiv_header=28 - header/footer content being included

**Impact on Quality**: This single document drags down the average significantly:

- Without AlphaEvolve: (0.853+0.943+0.957+0.931+0.753+0.939)/6 = **0.896** (89.6%)

### one_tool_2512.20957v2 (F1=0.753, Precision=0.670)

**Key Issue**: Moderate precision (67%) suggests extra text being included.

**Probable Causes**:

1. "camel_join=126" - 126 camelCase joins (second highest)
2. "arxiv_header=23" - header content mixed in
3. Double_space=262 - formatting issues

## First Principles Analysis

### Why is precision low on some documents?

```
Precision = (our_text ∩ gold_text) / our_text
```

Low precision = we're outputting text that ISN'T in gold:

- Header/footer content (arXiv metadata)
- Figure numbers and captions
- Reference formatting variations
- Page numbers

### Why is recall low on some documents?

```
Recall = (our_text ∩ gold_text) / gold_text
```

Low recall = gold has text that WE MISS:

- Text lost in column merge
- Font encoding failures
- Table cell content

### Pattern Distribution Analysis

```
┌─────────────────────────────────────────────────────────────────┐
│                    Pattern Impact by Document                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  camel_join: Text fragments being merged incorrectly            │
│  ├── AlphaEvolve: 286 (HIGHEST)                                 │
│  ├── one_tool: 126                                              │
│  ├── 01_2512: 117                                               │
│  └── Others: <100                                               │
│                                                                  │
│  arxiv_header: Header/footer content included                   │
│  ├── AlphaEvolve: 28 (HIGHEST)                                  │
│  ├── one_tool: 23                                               │
│  └── Others: <15                                                │
│                                                                  │
│  double_space: Extra whitespace                                 │
│  ├── 01_2512: 473 (HIGHEST)                                     │
│  ├── v2_2512: 314                                               │
│  ├── one_tool: 262                                              │
│  └── AlphaEvolve: 216                                           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Strategic Assessment

### What's Working Well (Keep)

1. **agent_2510.09244v1** extraction (F1=0.957) - The "truncated list items" issue from observe.md was STALE test data
2. **Column detection** (OODA-12) - Working correctly for 2-column layouts
3. **Speed optimization** - Achieved <0.1s/page target
4. **Test infrastructure** - 4-tier pyramid working well

### What Needs Improvement (Focus)

1. **Header/Footer Filtering** - Too much arXiv metadata included
2. **CamelCase Handling** - Text fragments getting merged without spaces
3. **Large Document Handling** - AlphaEvolve (44 pages) has worst quality

### Risk Assessment

| Improvement             | Impact | Effort | Risk   |
| ----------------------- | ------ | ------ | ------ |
| Header/footer removal   | HIGH   | MEDIUM | LOW    |
| CamelCase spacing       | MEDIUM | LOW    | LOW    |
| Figure caption handling | MEDIUM | HIGH   | MEDIUM |

## Conclusion

The extraction engine is fundamentally working well (5 of 7 documents at F1 > 0.85). The two problematic documents share common patterns:

1. High `camel_join` counts → text merging issues
2. High `arxiv_header` counts → header/footer content leaking

**Recommended focus for OODA-35+**: Improve precision by filtering header/footer content and fixing camelCase merging.
