# IT10 Observe: Table Detection Analysis

## Current State

### Table Detection Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    TABLE DETECTION PIPELINE                 │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. LatticeEngine (backend/lattice.rs)                      │
│     • Detects BORDERED tables using graphical lines         │
│     • Works: Simple boxes, grids with visible borders       │
│     • Fails: Borderless tables (academic papers)            │
│     • Lines 53-163                                          │
│                                                             │
│  2. TableDetectionProcessor (processors/table_detection.rs) │
│     • Detects tables from spatial block arrangement         │
│     • Groups blocks by Y-coord, finds multi-column rows     │
│     • Lines 76-519                                          │
│     • DISABLED for multi-column pages (OODA-34)             │
│                                                             │
│  3. TextTableReconstructionProcessor                        │
│     • Reconstructs tables from text patterns                │
│     • Looks for "Table N" captions, then scans nearby blocks│
│     • Lines 543-980                                         │
│     • Limited: Only works if caption present                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Problem Evidence

From LightRAG paper extraction:

```
2026-02-05T11:55:36.165842Z  INFO Lattice detected 0 tables on page 4
2026-02-05T11:55:36.146576Z  INFO Lattice detected 0 tables on page 13
```

The paper has Table 4 on page 13, but LatticeEngine detected 0 tables because:

1. Academic papers use borderless tables
2. LatticeEngine only finds tables via graphical lines

### Quality Target

| Category | Current | Target | Gap |
| -------- | ------- | ------ | --- |
| Tables   | 50/100  | 80/100 | 30  |

### Root Cause Analysis

```
┌─────────────────────────────────────────────────────────────┐
│                    WHY TABLES ARE MISSED                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. LatticeEngine needs visible lines (borders)             │
│     → Most academic tables have NO borders                  │
│                                                             │
│  2. TableDetectionProcessor is DISABLED for multi-column    │
│     → OODA-34 skips it to preserve reading order            │
│     → Academic papers ARE multi-column                      │
│                                                             │
│  3. TextTableReconstructionProcessor needs "Table N" caption│
│     → Doesn't detect tables without captions                │
│     → Doesn't detect structural patterns (aligned columns)  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### PyMuPDF4LLM Approach

From `helpers/pymupdf_rag.py` line 1059:

```python
tabs = page.find_tables(clip=parms.clip, strategy=table_strategy)
```

PyMuPDF uses native `page.find_tables()` with different strategies:

- `"lines"` - Similar to our LatticeEngine (uses graphical lines)
- `"text"` - Uses text alignment patterns (we're missing this!)

### Missing Capability

We need **whitespace/alignment-based table detection** that:

1. Detects columns by text alignment (x-coordinates)
2. Detects rows by y-coordinate clustering
3. Works WITHOUT graphical lines
4. Works WITHIN multi-column layouts (per-column detection)

### Code References

- `src/backend/lattice.rs:53-163` - LatticeEngine
- `src/processors/table_detection.rs:76-519` - TableDetectionProcessor
- `src/processors/table_detection.rs:543-980` - TextTableReconstructionProcessor
- `src/layout/column_detector.rs:336-345` - `is_table` heuristics (unused?)

### Test Count

Current: 516 tests passing (after IT09)
