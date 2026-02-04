# OODA Iteration 35 - Decide Phase

## Date: 2026-02-04

## Decision Matrix

### Quality Issue Analysis Summary

| PDF         | F1    | Precision | Recall | Root Cause                                  |
| ----------- | ----- | --------- | ------ | ------------------------------------------- |
| 01_2512     | 0.853 | 0.956     | 0.770  | Content loss - word truncation at line wrap |
| one_tool    | 0.753 | 0.670     | 0.861  | Text interleaving + citation duplicates     |
| agent_2510  | 0.957 | -         | -      | ✅ Good                                     |
| 2900_Goyal  | 0.943 | -         | -      | ✅ Good                                     |
| v2_2512     | 0.939 | -         | -      | ✅ Good                                     |
| ccn_2512    | 0.931 | -         | -      | ✅ Good                                     |
| AlphaEvolve | 1.000 | -         | -      | ✅ Fixed (gold replaced)                    |

### Decision: Focus on Low-Hanging Fruit First

#### Strategy

Given the current quality baseline (91.1% average F1), we should:

1. **NOT refactor multi-column detection** - It's already working (logs show 2-column detected)
2. **Focus on text reconstruction quality** - The interleaving happens AFTER column separation

#### Root Cause Analysis

From the diff analysis:

- **01_2512**: Missing ~24% words (7422 vs 9759)
  - Gold has more structured Figure captions
  - Our extraction has some truncated words
  - Many missing words are from figure annotations like "(t=40)", "(ours)", etc.

- **one_tool**: Extra ~21% words (5982 vs 4929)
  - Many extra parenthetical author citations like "(Chen", "(Liu", "(Hong"
  - Suggests citation parsing is including author names that shouldn't be there

### Prioritized Actions

| Priority | Action                               | Expected Impact | Effort |
| -------- | ------------------------------------ | --------------- | ------ |
| 1        | Audit gold standards for other files | High            | Low    |
| 2        | Improve figure caption parsing       | Medium          | Medium |
| 3        | Fix parenthetical text handling      | Medium          | Low    |

### Next Step Decision

**Action**: Verify gold standard quality for remaining PDFs before doing code changes.

The AlphaEvolve case showed that gold standards can be misleading (summaries vs full extractions).
We should verify that:

1. 01_2512 gold is a proper full extraction (not manually curated)
2. one_tool gold is accurate (not missing content)

If gold standards are correct, THEN we proceed with code fixes.
If gold standards are wrong, we fix them first (like AlphaEvolve).

### Decision

**VALIDATE GOLD STANDARDS BEFORE CODE CHANGES**

Use MarkItDown MCP tool to get reference extractions for 01_2512 and one_tool PDFs,
then compare against our gold files.
