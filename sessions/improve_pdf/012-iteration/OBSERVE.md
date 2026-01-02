# OBSERVE - Loop 012

## Timestamp

Fri Jan 2, 2026 19:15:00 HKT

## Directory Scope

**Initial Assessment - All modules**

This iteration will analyze error patterns to identify the highest-impact directory for focused improvement.

## Test Results

### Cargo Tests

```
✅ All tests passing
- Total: 111 tests
- Status: PASS (0 failures)
```

### Validator Metrics (Current)

```
Composite Score:     32.5/100
Table Accuracy:      2.4%  (weight: 40%)
Style Accuracy:      31.5% (weight: 40%)
Robustness:          100%  (weight: 10%)
Performance:         90%   (weight: 10%)
```

### Historical Progress

- **Baseline (Loop 001)**: Composite 27.2 (Table 3.5%, Style 16.9%)
- **Loop 004**: Composite 32.5 (Table 2.4%, Style 31.5%) [+5.3 points]
- **Loop 012 (Current)**: Composite 32.5 (no change from Loop 004)

## Drift Analysis (3052 Total Drifts)

### By Severity

- 🔴 **CRITICAL**: 857 (28%)
- 🟠 **MAJOR**: 909 (30%)
- 🟡 **MINOR**: 1286 (42%)

### By Category (Top Issues)

1. **content:mismatch**: 2067 occurrences (68%) - **PRIMARY ISSUE**
2. **style:mismatch**: 470 occurrences (15%)
3. **list:mismatch**: 282 occurrences (9%)
4. **table:mismatch**: 140 occurrences (5%)
5. **heading:mismatch**: 82 occurrences (3%)

### Pattern Analysis

The overwhelming presence of **content:mismatch** (2067/3052 = 68%) indicates the extraction layer is not correctly capturing or preserving text content from PDFs. This is distinct from rendering issues.

Style mismatches (470) are significant but secondary. Table mismatches (140) are relatively fewer but carry 40% weight in the composite score.

## Real Dataset Output Patterns

### Observed Issues

1. **Camel joins**: Still present (e.g., AlphaEvolve: 70 instances)
2. **Double spaces**: Still present (e.g., agent_2510.09244v1: 1306 instances)
3. **Hyphen breaks**: Common across all documents (14-55 per doc)
4. **Table detection**: Lattice engine detecting columns but not converting correctly

### Per-Document Composite Scores

- 2900_Goyal_et_al: 34.7/100
- AlphaEvolve: 39.2/100 (best performer)
- agent_2510.09244v1: 36.6/100
- ccn_2512.21804v1: 20.6/100 (worst performer - only 3.9% style accuracy)
- one_tool_2512.20957v2: 31.6/100

## Warnings & Dead Code

```
- unused import: SectionPatternProcessor
- unused variable: avg_density (sota_backend.rs:1736)
- unused variable: body_size (sota_backend.rs:2340)
- unused variable: avg_freq (llm_enhance.rs:288)
- unused field: lattice_engine in SotaBackend
```

## Root Cause Hypothesis

**Primary Issue: Content Extraction**
The 2067 content mismatches suggest the text extraction phase is losing or corrupting content. This likely occurs in:

- `backend/sota_backend.rs` (text_blocks extraction from PDF operators)
- `processors/processor.rs` (TextProcessor, WhitespaceNormalizationProcessor)
- `extractor.rs` (overall pipeline)

**Secondary Issue: Style Preservation**
The 470 style mismatches indicate font attributes (bold, italic) are not being correctly:

- Extracted from PDF font descriptors (backend layer)
- Propagated through TextBlock/TextSpan structures
- Rendered as markdown (renderer layer)

**Tertiary Issue: Table Detection**
The lattice_engine is detecting columns but failing to extract cell content correctly. The unused lattice_engine field in SotaBackend suggests it's not integrated into the main pipeline.

## Next Steps (ORIENT Phase)

1. Examine drift_loop_012.json for specific content mismatch examples
2. Trace content extraction path: PDF operators → TextBlock → MarkdownRenderer
3. Identify the directory with highest content extraction impact
4. Prioritize fixing content extraction before style/table issues
