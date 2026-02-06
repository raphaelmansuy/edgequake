# IT33 — Observe

## Problem Statement

`TextTableReconstructionProcessor` fails to detect tables in academic papers (e.g., `lighrag_2410.05779v3.pdf`). Tables 1, 2, 3, 5 are not reconstructed — they appear as plain text dumps in the output.

## Root Cause Analysis

### 1. `table_like_score` returns 0 for percentage-rich column blocks

Academic tables like LightRAG Table 1 contain blocks with percentage values:

```
32.4%
23.6%
32.4%
32.4%
```

The existing scoring only checks for `|` separators, tab characters, and ≥3 columns of `key: value` pairs. Percentage-rich blocks score 0, so the table scanner skips them entirely.

### 2. PdfiumBackend produces fragmented blocks

Unlike PyMuPDF (which groups data into 3-4 large column blocks), PdfiumBackend produces 14+ small blocks for a single table:

```
Block 1: "Comprehensiveness\nDiversity\nEmpowerment\nOverall"  (4 lines)
Block 2: duplicate of Block 1
Block 3: duplicate of Block 1
Block 4: duplicate of Block 1
Block 5: "32.4%\n23.6%\n32.4%\n32.4%\nRQ-RAG\n31.6%\n..."    (14 lines, mixed)
Block 6: "45.6%\n22.8%\n41.2%\n45.2%"                         (4 lines)
Block 7: "Agriculture"                                          (1 line)
...
```

### 3. `parse_numeric_suffix` rejects `%` values

Values like "32.4%" fail `parse_numeric_suffix` because `%` is not stripped before attempting float parse.

## Evidence

- Tested with `lighrag_2410.05779v3.pdf` page 7 (Table 1)
- Block analysis via PdfiumBackend logging showed 14 blocks vs PyMuPDF's 3
- Table 4 (page 12) works because it has simpler structure with `parse_rows` path
