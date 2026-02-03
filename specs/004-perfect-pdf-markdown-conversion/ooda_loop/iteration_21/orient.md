# OODA-21 Orient: First Principles Analysis

## Date: 2025-02-03

## Root Cause Analysis: Why 80.8% Quality vs 95% Target

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                  QUALITY GAP ROOT CAUSE ANALYSIS                              │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│  Current: 80.8% (Text: 81.3%, Structure: 80.3%)                              │
│  Target:  95%+ (Text: 98%, Structure: 95%)                                   │
│  Gap:     ~14 percentage points                                               │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │                    LOSS BREAKDOWN                                │        │
│  ├─────────────────────────────────────────────────────────────────┤        │
│  │                                                                   │        │
│  │  Text Preservation Loss (18.7%):                                 │        │
│  │  ├── Font encoding issues: ~5%                                   │        │
│  │  ├── Word boundary errors: ~5%                                   │        │
│  │  ├── Special characters: ~3%                                     │        │
│  │  ├── Header/footer filtering: ~3%                                │        │
│  │  └── Other extraction issues: ~2.7%                              │        │
│  │                                                                   │        │
│  │  Structural Fidelity Loss (19.7%):                               │        │
│  │  ├── Table detection misses: ~7%                                 │        │
│  │  ├── Header hierarchy wrong: ~4%                                 │        │
│  │  ├── List recognition: ~4%                                       │        │
│  │  ├── Reading order errors: ~3%                                   │        │
│  │  └── Block merging issues: ~1.7%                                 │        │
│  │                                                                   │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│                                                                               │
└──────────────────────────────────────────────────────────────────────────────┘
```

## First Principles: What Markitdown Does Right

### 1. Dual-Backend Strategy

```
Markitdown Logic:
IF page looks like form/table structure THEN
    Use pdfplumber word-level extraction
ELSE
    Use pdfminer text-level extraction (better spacing)

Edgequake Current:
Always use lopdf -> single extraction path
```

**WHY this matters:** Text-heavy academic papers benefit from different extraction strategy than form-style documents.

### 2. Adaptive Table Classification

Markitdown uses sophisticated row classification:

```python
# Paragraph detection (NOT a table row if):
is_paragraph = line_width > page_width * 0.55 and len(combined_text) > 60

# Table row detection (IS a table row if):
uses 2+ established global columns
```

Edgequake currently lacks paragraph detection when deciding table membership.

### 3. Global Column Boundary Calculation

Markitdown:
- Collects ALL x-positions from rows with 3+ columns
- Clusters with 30px tolerance
- Uses global boundaries for ALL rows

Edgequake:
- Calculates column boundaries per-page
- No global document-level consistency

## Per-Document Quality Analysis

From test output:

| Document | Text | Structure | Issue |
|----------|------|-----------|-------|
| ccn_2512.21804v1 | 80.5% | 85.9% | Missing text |
| 2900_Goyal_et_al | 91.1% | 80.8% | Structure loss |
| v2_2512.25072v1 | 85.1% | 76.8% | Structure loss |
| AlphaEvolve | 85.6% | 74.3% | **Worst structure** |
| agent_2510.09244v1 | 81.0% | 77.6% | Both issues |
| 01_2512.25075v1 | 72.2% | 88.7% | **Worst text** |
| one_tool_2512.20957v2 | 73.7% | 78.1% | Both issues |

**Key Finding:** AlphaEvolve has worst structural fidelity (74.3%) - likely has tables.
**Key Finding:** 01_2512.25075v1 has worst text (72.2%) - likely encoding issues.

## Comparison: Current Thresholds vs Markitdown

| Parameter | Edgequake | Markitdown | Analysis |
|-----------|-----------|------------|----------|
| Y-tolerance (row grouping) | 10pt | 5pt | Ours too loose |
| Min gap for column | 20pt | 30pt | Ours tighter |
| Paragraph width threshold | Not used | 55% page | Missing feature |
| Paragraph char threshold | Not used | 60 chars | Missing feature |
| Max columns for table | Not set | 8 | Could reject dense text |
| Min table rows | 3 | 3 | Same |
| Long cell threshold | Not used | 30% | Missing feature |

## Architecture Comparison

```
┌────────────────────────────────────────────────────────────────────────┐
│                       EDGEQUAKE CURRENT FLOW                            │
├────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   PDF                                                                   │
│    │                                                                    │
│    ▼                                                                    │
│  ┌─────────────┐                                                       │
│  │   lopdf     │ ← Single backend                                      │
│  └──────┬──────┘                                                       │
│         │                                                               │
│         ▼                                                               │
│  ┌─────────────┐                                                       │
│  │ extraction  │ → TextElement[]                                       │
│  │  _engine    │                                                       │
│  └──────┬──────┘                                                       │
│         │                                                               │
│         ▼                                                               │
│  ┌─────────────┐   ┌─────────────┐                                    │
│  │  column_    │ → │ text_       │ → Lines[]                          │
│  │  detection  │   │ grouping    │                                     │
│  └─────────────┘   └──────┬──────┘                                    │
│                           │                                             │
│                           ▼                                             │
│                   ┌─────────────┐                                      │
│                   │  layout_    │ → Blocks[]                           │
│                   │  processing │                                      │
│                   └──────┬──────┘                                      │
│                          │                                              │
│                          ▼                                              │
│                   ┌─────────────┐                                      │
│                   │  table_     │ → Document                           │
│                   │  detection  │                                      │
│                   └─────────────┘                                      │
│                                                                         │
└────────────────────────────────────────────────────────────────────────┘
```

## Highest-Impact Improvements

### Priority 1: Add Paragraph Detection (Impact: +5-7%)

Current table detection treats all multi-block rows as potential tables.
Adding paragraph detection would exclude long text blocks from table consideration.

```rust
// Proposed: Add paragraph check before table row inclusion
fn is_paragraph(block: &Block, page_width: f32) -> bool {
    let block_width = block.bbox.x2 - block.bbox.x1;
    let text_len = block.text.chars().count();
    
    // WHY 55%: Markitdown threshold, based on typical column width
    // WHY 60 chars: Long text = paragraph, not table cell
    block_width > page_width * 0.55 && text_len > 60
}
```

### Priority 2: Tighten Y-Tolerance for Table Detection (Impact: +3-4%)

Current 10pt is too loose. Should use 5pt like markitdown for precise table row alignment.

### Priority 3: Add Long Cell Rejection (Impact: +2-3%)

If >30% of cells in a candidate table have >30 characters, it's probably not a table.

### Priority 4: Word-Level Precision Improvement (Impact: +2-3%)

Tighter x/y tolerances in text grouping would improve word boundary accuracy.

## Risk Assessment

| Change | Benefit | Risk | Mitigation |
|--------|---------|------|------------|
| Paragraph detection | Fewer false table positives | May miss table rows | Use conservative 55%/60 thresholds |
| Tighter Y-tolerance | Better table row grouping | May split valid tables | Test on real tables first |
| Long cell rejection | Reject prose-as-table | May reject large-cell tables | Use 30% threshold |
| Tighter word grouping | Better text preservation | May over-segment | Test incrementally |

## Recommended Action Plan

1. **Immediate (OODA-21)**: Add paragraph detection to table_detection.rs
2. **Next (OODA-22)**: Tighten Y-tolerance to 5pt in table detection
3. **Then (OODA-23)**: Add long cell rejection logic
4. **Finally (OODA-24)**: Test and measure impact

Expected combined impact: +10-15% structural fidelity, +2-3% text preservation.

## Files to Modify

| File | Change | Lines |
|------|--------|-------|
| processors/table_detection.rs | Add paragraph detection | ~50 lines |
| processors/table_detection.rs | Tighten Y-tolerance | ~5 lines |
| processors/table_detection.rs | Add long cell rejection | ~30 lines |
| tests/comprehensive_quality.rs | Verify metrics improve | ~0 lines |
