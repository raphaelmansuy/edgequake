# OODA Loop 1 - OBSERVE

## Iteration Info

- **Loop:** 1
- **Directory:** `crates/edgequake-pdf/src/processors`
- **Focus:** Table detection and cell extraction
- **Date:** 2026-01-03

## Baseline Metrics (from PDF-Markdown Validator SKILL)

```
Documents processed: 5
Table Accuracy:      2.4%  ← CRITICAL BOTTLENECK
Style Accuracy:      31.1%
Robustness:          100.0%
Performance:         90.0%

Composite Score:     32.4/100
```

### Per-Document Breakdown:

- `2900_Goyal_et_al`: Table: 0.0%, Style: 37.6%, Composite: 34.0
- `AlphaEvolve`: Table: 0.3%, Style: 50.2%, Composite: 39.2
- `agent_2510.09244v1`: Table: 0.0%, Style: 44.0%, Composite: 36.6
- `ccn_2512.21804v1`: Table: 0.0%, Style: 3.9%, Composite: 20.6
- `one_tool_2512.20957v2`: Table: 11.4%, Style: 20.0%, Composite: 31.6

## Test Results

- ✅ `cargo test -p edgequake-pdf` passes (with warnings)
- ✅ All 5 PDFs process without crashes
- ✅ Markdown validation passes

## Table Detection Observations

From evaluator logs:

- Many tables **rejected** due to high `crossing_ratio` (text elements cross column boundaries)
- Example rejects:

  - `crossing_ratio=0.12 (2/16)` → reject
  - `crossing_ratio=0.85 (34/40)` → reject
  - `crossing_ratio=0.40 (20/50)` → reject

- **Root cause hypothesis:** Current table detection uses a simplistic column-crossing heuristic that fails when:
  1. PDF text extraction produces word-level blocks that naturally cross "visual" column boundaries
  2. Multi-line cells have slight alignment variations
  3. Table captions or merged cells don't align perfectly

## Key Patterns Observed

1. **Table Detection is too strict:** The `crossing_ratio` threshold appears too aggressive, rejecting valid tables
2. **Cell extraction may be incomplete:** Even when tables are detected (11.4% for `one_tool`), cell content accuracy is low
3. **No lattice-based table extraction:** Current approach appears layout-based only, missing explicit table structures in PDFs

## Next Steps (Orient Phase)

Need to investigate:

1. `crates/edgequake-pdf/src/processors/processor.rs` - Table detection logic and `crossing_ratio` calculation
2. How table cells are extracted and mapped to markdown
3. Whether PDF contains explicit table structures (lattice) that we're ignoring
