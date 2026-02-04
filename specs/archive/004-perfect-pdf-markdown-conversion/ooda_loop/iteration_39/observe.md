# OODA Iteration 39: Challenge Gold Standard with First Principles

## Observe Phase

### Current Metrics

| File                  | F1    | Precision | Recall |
| --------------------- | ----- | --------- | ------ |
| one_tool_2512.20957v2 | 0.752 | 0.667     | 0.863  |
| AlphaEvolve           | 1.000 | 1.000     | 1.000  |
| 2900_Goyal_et_al      | 0.943 | 0.921     | 0.967  |
| agent_2510.09244v1    | 0.957 | 0.987     | 0.928  |

### Key Observation

The `one_tool_2512.20957v2` file has the **lowest F1 score** (0.752) across all test documents, significantly below others.

### Investigation Using markitdown MCP

Used Microsoft's markitdown MCP tool (86K⭐, official reference) to extract the same PDF and compare with gold standard:

**markitdown output (physical PDF order):**

```
One Tool Is Enough: Reinforcement Learning for Repository-Level LLM Agents

Zhaoxi Zhang 1 Yitong Duan 2 Yanzhi Zhang 2 Yiming Xu 1 Jiyan He 2 Yunfang Wu 1
...
Abstract
...
1. Introduction
...
1School of Computer Science, Peking University
2Zhongguancun Academy.
```

**Gold standard (line 3-4):**

```markdown
**Authors:** Zhaoxi Zhang, Yitong Duan, Yanzhi Zhang, Yiming Xu, Jiyan He, Yunfang Wu
**Affiliation:** School of Computer Science, Peking University; Zhongguancun Academy
```

### Discovery

The gold standard contains **SYNTHESIZED METADATA** that doesn't exist in the physical PDF:

1. `**Authors:**` prefix - editorially added, not in PDF
2. `**Affiliation:**` prefix - editorially added, not in PDF
3. Comma-separated author names - PDF has spaces/superscripts
4. Affiliations moved to top - PDF has them mid-page as footnotes
5. arXiv header omitted - exists in physical PDF

### Implications

Both markitdown and our extractor produce output where:

- Authors have superscript affiliation numbers
- Affiliations appear mid-document (their physical location)
- arXiv header is present

This means the gold standard represents an **idealized, semantically enhanced** version that requires human-level understanding to produce, NOT faithful extraction.
