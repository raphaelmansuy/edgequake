# OODA-23 Observe: Deep Analysis of Lowest-Scoring PDF

## Target Document

**one_tool_2512.20957v2.pdf** - Overall: 75.9% (Text: 73.7%, Structure: 78.1%)

## Comparison Analysis

### Gold File (Human-Curated) - First Paragraph

```markdown
Locating the files and functions requiring modification in large open-source
software (OSS) repositories is challenging due to their scale and structural
complexity. Existing large language model (LLM)-based methods typically treat
this as a repository-level retrieval task and rely on multiple auxiliary tools,
which overlook code execution logic and complicate model control.
```

### Our Extraction - Same Section

```markdown
Locating the files and functions requiring modification in large open-source
software (OSS) repositories is challenging due to their scale and structural
complexity. Existing large language model (LLM)-based methods typically treat
this as a repository-level retrieval task and rely on multiple auxiliary tools,
which overlook code execution logic and complicate model control. We propose
RepoNavigator, an LLM agent equipped with a single execution-aware tool-jumping
to the definition of a invoked symbol.
```

### Key Differences Identified

1. **Text Fragmentation**: Our extraction has sentences split across paragraphs
   - "Learning (RL) directly from a pretrained model," appears separated
   - References like "et al., 2025)" appear as fragments

2. **Column Interleaving**: Two-column PDF causes text from different columns to mix
   - Line 35: "trains and texts only and developers can define" (mixed)
   - Author affiliations appear mid-document

3. **Missing Text**: Some content is dropped during extraction
   - Figure captions may be incomplete
   - Mathematical formulas not rendered properly

4. **Markitdown Comparison** (same PDF):
   - No arXiv margin artifacts (good)
   - Cleaner paragraph structure
   - But still has some issues with table rendering

## Root Cause Analysis

```
                    PDF Structure
                         │
           ┌─────────────┼─────────────┐
           │             │             │
    Left Column    Right Column   Figures
           │             │             │
           └─────────────┼─────────────┘
                         │
              Current Extraction
                         │
           ┌─────────────┼─────────────┐
           │             │             │
    Col 1 Block 1  Col 2 Block 1  Fig Caption
           │             │             │
    Col 1 Block 2  Col 2 Block 2      ...
           │             │             │
           └─────────────┼─────────────┘
                         │
              Reading Order Problem
                         │
           Blocks sorted by Y, then X
           But should be: Col1 fully, then Col2
```

## Metrics Breakdown

| Aspect              | Score | Issue                                 |
| ------------------- | ----- | ------------------------------------- |
| Text Preservation   | 73.7% | Word overlap reduced by fragmentation |
| Structural Fidelity | 78.1% | Paragraph/heading structure damaged   |

## Word Count Analysis

- Gold file: ~5,200 words
- Our extraction: ~4,800 words
- Missing: ~400 words (8%)

## Specific Problem Areas

1. **Abstract section**: Text fragments with "," at end of paragraphs
2. **Introduction**: Reference numbers scattered ("et al., 2025)")
3. **Method section**: Mathematical notation dropped or corrupted
4. **Tables**: Table content mixed with prose

## Hypothesis

The reading order algorithm is sorting blocks by Y-coordinate first, which
works for single-column documents but fails for two-column academic papers.

Need to investigate: `column_detection.rs` and how it handles multi-column layouts.
