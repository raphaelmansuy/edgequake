# OODA-06 Observe: Two-Column Merge Issue

## Observation Summary

Comparing edgequake output against markitdown reference for `agentfail_2601.22984v1.pdf` reveals **critical two-column layout issues**.

## Evidence

### Issue 1: Column Text Concatenation

**Edgequake Output (broken):**

```
peded by three obstacles: (1) Taxonomic Gap: Hallucina-veal that no agent achieves robust reliability. We identify
tion taxonomies tailored to DRAs remain under-explored;a strategic dichotomy between over-confidence and over-
```

**Markitdown Output (correct):**

```
peded by three obstacles: (1) Taxonomic Gap: Hallucina-
tion taxonomies tailored to DRAs remain under-explored;
(2) Data Acquisition Barriers: Proprietary DRAs either
```

The left column text is being merged with right column text on the same line.

### Issue 2: Incorrect Reading Order

The system appears to be reading across columns instead of down columns:

- Should read: Left column top-to-bottom, THEN right column top-to-bottom
- Actually reads: Mixed left-right concatenation on each "row"

### Issue 3: Missing Line Breaks

Words are concatenated without spaces at column boundaries:

- "Hallucina-veal" instead of "Hallucina-\n" + "veal"
- "under-explored;a" instead of "under-explored;\n" + "a"

## Root Cause Hypothesis

The column detection is working (logs show "Detected SINGLE-COLUMN layout") but for academic papers with two-column layout, this is incorrect.

Looking at the extraction logs:

```
Detected SINGLE-COLUMN layout (left_starts=19, right_starts=3, balance=0.16)
```

The column detection threshold may be:

1. Too strict - requiring more evidence for two-column detection
2. Not properly analyzing academic paper layouts
3. Missing the typical academic paper two-column boundary at x≈300

## Files to Investigate

1. `column_detection.rs` - How columns are detected
2. `text_grouping.rs` - How blocks are assigned to columns
3. `extraction_engine.rs` - How reading order is determined
4. `block_builder.rs` - How blocks are formed from elements

## Quality Metrics Impact

- **TPS (Text Preservation)**: Words exist but corrupted → 60-70%
- **SFS (Structural Fidelity)**: Two-column structure destroyed → 20-30%
- **ROA (Reading Order)**: Completely broken → 10-20%

This is a **critical** issue affecting all two-column academic papers (arXiv papers, conference papers, etc.)
