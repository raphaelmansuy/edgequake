# IT33 — Orient

## Analysis

The table detection pipeline has three layers:

```
┌─────────────────────────────────────────────┐
│  1. Caption Detection (regex)               │  ← "Table N:" pattern match
│  2. table_like_score (heuristic)            │  ← Scores nearby blocks 0-10
│  3. Row Parsing / Column Reconstruction     │  ← Builds markdown table
└─────────────────────────────────────────────┘
```

Layer 1 works fine. Layer 2 fails because it has no percentage-value awareness. Layer 3 only has row-oriented parsing — no column-oriented reconstruction for PdfiumBackend's fragmented blocks.

## Gap Analysis

| Aspect                  | Current  | Needed                                        |
| ----------------------- | -------- | --------------------------------------------- |
| Percentage scoring      | 0 points | +3 bonus for >50% percentage lines            |
| Short multiline scoring | 0 points | +1 for blocks with ≤20 char avg, 3+ lines     |
| Numeric-line scoring    | 0 points | +2 for blocks where >50% lines are numeric    |
| Column reconstruction   | None     | Parse linearized [label, val, val, ...] grids |
| `%` handling            | Rejected | Strip before float parse                      |

## First Principles

Academic tables have a characteristic pattern:

- **Columns are linearized**: Each column becomes a vertical text block
- **Labels alternate with values**: `[label1, val1, val2, ..., label2, val1, val2, ...]`
- **Values are mostly numeric/percentage**: Easy to distinguish from labels
- **Column count = values between consecutive labels**

This is a structural pattern independent of content — it works for any table with numeric data columns.
