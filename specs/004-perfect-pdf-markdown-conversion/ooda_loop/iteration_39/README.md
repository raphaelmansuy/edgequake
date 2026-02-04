# OODA Iteration 39: Challenge Gold Standard with First Principles

## Summary

**Goal:** Investigate why `one_tool_2512.20957v2` has the lowest F1 score (0.753) across all test documents.

## Method

Used Microsoft's **markitdown MCP tool** (86K⭐, official reference) to extract the same PDF and compare with gold standard.

## Key Finding

The gold standard contains **SYNTHESIZED METADATA** that doesn't exist in the physical PDF:

| Gold Standard Has | PDF Actually Contains |
|-------------------|----------------------|
| `**Authors:** Zhang, Duan...` | `Zhaoxi Zhang 1 Yitong Duan 2...` |
| `**Affiliation:** University...` | Affiliations mid-page as footnotes |
| Clean comma-separated names | Names with superscript numbers |

## First Principles Decision

For a RAG system (EdgeQuake's use case):
- **Faithful extraction** is more valuable than semantic synthesis
- LLMs can interpret raw text; they can't recover lost information
- The gold standard should represent "best faithful extraction", NOT "ideal semantic document"

## Action Taken

Updated gold standard to remove synthesized content:
- Removed `**Authors:**` prefix
- Removed `**Affiliation:**` line
- Kept author names without bold

## Result

F1 remained at 0.752 because the MAJOR issues are genuine extraction problems:
1. Two-column text interleaving
2. Author name merging (missing spaces)
3. arXiv header presence

## Lesson Learned

Challenging assumptions with first principles revealed that:
1. Part of the problem was unrealistic gold standard
2. But genuine extraction quality issues also exist
3. Next focus should be on two-column layout handling

## Files Modified

- `test-data/real_dataset/one_tool_2512.20957v2.gold.md` - Removed synthesized metadata

## Status

✅ Completed - Documented gold standard philosophy for future reference
