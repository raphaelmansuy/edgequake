# IT10 Orient: Gap Analysis and Strategy

## Key Finding: Dead Code for Table Detection

```
┌─────────────────────────────────────────────────────────────┐
│              DEAD CODE DISCOVERY                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  layout/column_detector.rs:282-345                          │
│                                                             │
│  pub fn is_likely_table(&self, ...) -> bool                 │
│                                                             │
│  This function EXISTS but is NEVER CALLED!                  │
│                                                             │
│  It has sophisticated table detection logic:                │
│  - Groups items by Y-coordinate into rows                   │
│  - Checks fill_ratio (items vs column width)                │
│  - Detects short_ratio (sparse data cells)                  │
│  - Detects uniform column widths                            │
│  - Multi-item row detection                                 │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Architecture Gap

```
Current Pipeline:
┌──────────────────────────────────────────────────────────────┐
│  PDF                                                         │
│   │                                                          │
│   ├──> LatticeEngine ──> Detects BORDERED tables             │
│   │                      (lines/grids only)                  │
│   │                                                          │
│   ├──> ExtractionEngine ──> Text elements                    │
│   │         │                                                │
│   │         └──> ColumnDetector.detect_columns()             │
│   │              (DOES NOT call is_likely_table!)            │
│   │                                                          │
│   └──> TableDetectionProcessor                               │
│         (DISABLED for multi-column pages!)                   │
│                                                              │
│  RESULT: Borderless tables in multi-column PDFs = MISSED    │
└──────────────────────────────────────────────────────────────┘

Needed Pipeline:
┌──────────────────────────────────────────────────────────────┐
│  PDF                                                         │
│   │                                                          │
│   ├──> LatticeEngine ──> Detects BORDERED tables             │
│   │                                                          │
│   ├──> ExtractionEngine                                      │
│   │         │                                                │
│   │         └──> For EACH column:                            │
│   │              1. ColumnDetector.detect_columns()          │
│   │              2. ColumnDetector.is_likely_table()  ← NEW  │
│   │              3. If table: StreamTableDetector ← NEW      │
│   │                                                          │
│   └──> TableDetectionProcessor (for single-column pages)     │
│                                                              │
│  RESULT: Borderless tables detected via alignment patterns   │
└──────────────────────────────────────────────────────────────┘
```

## PyMuPDF4LLM Comparison

| Feature              | PyMuPDF4LLM           | EdgeQuake-PDF              |
| -------------------- | --------------------- | -------------------------- |
| Line-based tables    | ✅ `strategy="lines"` | ✅ LatticeEngine           |
| Text-based tables    | ✅ `strategy="text"`  | ⚠️ is_likely_table (DEAD)  |
| Caption-based        | ✅                    | ✅ TextTableReconstruction |
| Multi-column support | ✅                    | ❌ Disabled by OODA-34     |

## Root Cause Priority

1. **HIGH**: `is_likely_table()` exists but isn't wired into pipeline
2. **HIGH**: TableDetectionProcessor disabled for multi-column
3. **MEDIUM**: Need per-column table detection within multi-column pages
4. **LOW**: TextTableReconstructionProcessor only triggers on captions

## Quick Win Analysis

```
┌─────────────────────────────────────────────────────────────┐
│                    EFFORT vs IMPACT                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  HIGH IMPACT + LOW EFFORT:                                  │
│  • Wire is_likely_table() into extraction pipeline          │
│  • Add flag to detect table-like regions                    │
│                                                             │
│  HIGH IMPACT + MEDIUM EFFORT:                               │
│  • Per-column table detection for multi-column pages        │
│  • StreamTableDetector (text alignment-based)               │
│                                                             │
│  MEDIUM IMPACT + HIGH EFFORT:                               │
│  • Full table reconstruction from aligned text              │
│  • Header detection, cell merging                           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Decision: Incremental Approach

For IT10, focus on **wiring the dead code**:

1. Wire `is_likely_table()` into the detection pipeline
2. Add table region flagging when detected
3. Test with academic papers

This is the quickest path to improved table detection (reuse existing code).
