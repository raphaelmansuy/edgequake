# OODA Iteration 35 - Orient Phase

## Date: 2026-02-04

## Analysis

### Root Cause Identification

After comparing gold standards with our extractions, I've identified **three distinct quality issues**:

#### Issue 1: Multi-Column Text Interleaving (CRITICAL)

**Affected PDFs**: 01_2512, one_tool  
**Symptom**: Text from different columns gets mixed together  
**Example from 01_2512**:

```
Gold (correct):
"With the rapid advancement of Large Language Models (LLMs),
equipping LLMs with pre-built tools to form LLM agents has
become a common paradigm..."

Our extraction (wrong):
"With the rapid advancement of Large Language Models (LLMs) (Liu et al., 2024; Team, 2024; Yang et al., 2025a), equipping LLMs
with pre-built tools to form LLM agents
In the domain of software engineering (SWE), although LLM agents can effectively handle simple programming tasks (Hui et al.,
 2024; Guo et al., 2024a), their ability to operate on large-scale open-source software (OSS) repositowhich is realized through a language server."
```

**Root cause**: Our Y-position sorting algorithm doesn't handle multi-column layouts correctly. Text objects from different columns but similar Y positions get interleaved.

#### Issue 2: Word Truncation at Line Boundaries

**Affected PDFs**: 01_2512  
**Symptom**: Words split across lines aren't rejoined  
**Example**:

```
"reposito-" at end of line, "ries" at start of next
Should become: "repositories"
Our extraction: "repositowhich is realized" (broken)
```

**Root cause**: Hyphen-continuation algorithm is too aggressive or missing context.

#### Issue 3: Arxiv Header/Footer Leakage

**Affected PDFs**: All  
**Symptom**: Page headers/footers appear in body text  
**Example**: "arXiv:2512.20957v2 [cs.SE] 25 Dec 2025" appears inline

**Root cause**: Header/footer detection not removing all instances.

### Impact on Quality Metrics

| Issue                     | Precision Impact | Recall Impact |
| ------------------------- | ---------------- | ------------- |
| Multi-Column Interleaving | -5%              | -10%          |
| Word Truncation           | 0%               | -5%           |
| Header Leakage            | -15%             | 0%            |

### Comparison: Gold vs Our Extraction

| PDF      | Gold Words | Our Words | Difference | Root Cause                   |
| -------- | ---------- | --------- | ---------- | ---------------------------- |
| 01_2512  | 9759       | 7422      | -24%       | Multi-column + truncation    |
| one_tool | 4929       | 5982      | +21%       | Header leakage + duplication |

### ASCII Diagram: Multi-Column Issue

```
PDF Page Layout:                    Our Current Extraction:
┌───────────────────────────────┐
│ Column 1      │  Column 2     │   Y=100: "Text from col1 Text from col2"
│ ─────────────   ──────────────│   Y=200: "More col1 More col2"
│ Text from     │ Text from     │   (WRONG - columns mixed)
│ column 1      │ column 2      │
│               │               │   Correct should be:
│ More col1     │ More col2     │   Y=100-400: "Text from column 1 More col1"
│               │               │   Y=100-400: "Text from column 2 More col2"
└───────────────────────────────┘   (COLUMNS SEPARATED)
```

### Priority Order for Fixes

1. **Multi-Column Detection** - Will fix 01_2512 recall + one_tool interleaving
2. **Header/Footer Filtering** - Will fix one_tool precision
3. **Hyphen Continuation** - Polish for edge cases

### Relevant Code Locations

1. [text_extraction.rs](edgequake/crates/edgequake-pdf/src/text_extraction.rs) - Main text extraction
2. [lattice.rs](edgequake/crates/edgequake-pdf/src/text_extraction/lattice.rs) - Text positioning
3. [markdown_renderer.rs](edgequake/crates/edgequake-pdf/src/render/markdown_renderer.rs) - Output rendering

### Next Step

Investigate the lattice algorithm to understand how it currently handles multi-column layouts and identify the fix needed.
