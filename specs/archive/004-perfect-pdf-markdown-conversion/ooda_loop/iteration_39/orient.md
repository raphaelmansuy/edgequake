# OODA Iteration 39: Orient Phase

## Root Cause Analysis

```
┌─────────────────────────────────────────────────────────────────────┐
│                    PDF EXTRACTION TRUTH                              │
├─────────────────────────────────────────────────────────────────────┤
│  PHYSICAL PDF        │  GOLD STANDARD      │  MARKITDOWN OUTPUT     │
├──────────────────────┼─────────────────────┼────────────────────────┤
│ Title                │ Title               │ Title                  │
│ Authors + superscript│ **Authors:** clean  │ Authors + superscript  │
│ Abstract             │ **Affiliation:**    │ Abstract               │
│ Introduction         │ Abstract            │ Introduction           │
│ ...content...        │ Introduction        │ ...content...          │
│ ¹Affiliations        │ ...content...       │ ¹Affiliations          │
│ ²Affiliations        │                     │ ²Affiliations          │
└──────────────────────┴─────────────────────┴────────────────────────┘
```

## First Principles Analysis

### Question: What SHOULD a PDF extractor produce?

**Option A: Faithful Physical Extraction**

- Extract text in physical reading order
- Preserve what's actually in the PDF
- Let downstream LLM do semantic interpretation
- Result: Affiliations mid-document, superscripts preserved

**Option B: Semantic Synthesis**

- Understand document semantics
- Recognize "affiliations" should be with authors
- Add metadata prefixes like `**Authors:**`
- Reorder content for semantic clarity

### For RAG Systems (EdgeQuake's Use Case)

**Option A is CORRECT** because:

1. LLMs can interpret raw text; they can't recover lost information
2. Faithful extraction preserves ALL information
3. Semantic cleanup should happen at query time, not extraction time
4. Different downstream tasks may need different interpretations

## Comparison: Why AlphaEvolve scores 1.0 but one_tool scores 0.75?

| Aspect         | AlphaEvolve Gold     | one_tool Gold             |
| -------------- | -------------------- | ------------------------- |
| Authors format | Bold without prefix  | `**Authors:**` prefix     |
| Affiliations   | Inline after authors | `**Affiliation:**` prefix |
| Physical match | YES                  | NO                        |

AlphaEvolve gold matches physical extraction. one_tool gold is semantically enhanced.

## Conclusion

The gold standard for `one_tool_2512.20957v2` is **UNREALISTIC** - it expects semantic synthesis that no pure extraction tool (including Microsoft's markitdown) can achieve.
