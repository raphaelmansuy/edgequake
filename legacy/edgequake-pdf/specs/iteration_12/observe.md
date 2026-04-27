# OODA-12: OBSERVE — v2 PDF Structural Fidelity Deep Dive

## Current Quality Status

| Metric              | Score |
| ------------------- | ----- |
| Text Preservation   | 83.6% |
| Structural Fidelity | 74.5% |
| Overall Quality     | 79.1% |

## v2 PDF: Worst Performer

v2_2512.25072v1.pdf has the lowest structural score at **50.4%** vs 74.5% average.

### Structural Element Comparison

| Element | Gold | Extracted | Missing | Gap  |
| ------- | ---- | --------- | ------- | ---- |
| Headers | 27   | 19        | 8       | -30% |
| Lists   | 44   | 5         | 39      | -89% |
| Tables  | 45   | 17        | 28      | -62% |

**MAJOR FINDING**: Lists at -89% is the primary contributor to low structural score.

## Root Cause Analysis

### 1. Reference Section Structure in Gold

```
* [1] L. Sentis and O. Khatib, "A whole-body control framework..."
* [2] X. Cheng, J. Li, S. Yang, "Open-television: Teleoperation..."
* [3] X. Gu, Y.-J. Wang, and J. Chen, "Humanoid-gym..."
... (44 references total)
```

### 2. Reference Section Structure in Extracted

```
## REFERENCES

humanoids operating in human environments," in ICRA., 2006.
Teleoperation with immersive active visual feedback," in CoRL, 2024.
[26] Z. Fu, Q. Zhao, Q. Wu, G. Wetzstein, and C. Finn, "Humanplus:
```

**PROBLEM IDENTIFIED**: References from columns are being merged together!

- "[1] L. Sentis..." → Only end of text preserved
- "[2]" marker missing entirely
- "[26]" from second column merged with first column text

### 3. ListDetectionProcessor Log Analysis

```
ListItem '[36] offers fast inference...'  x1=54.0
ListItem '3) arm end-effector tracking...' x1=313.2
ListItem '[3] X. Gu, Y.-J. Wang...'      x1=58.0
ListItem '[8] E. Hsieh, W.-H...'         x1=58.0
ListItem '[13] A. Guzman-Rivera...'      x1=54.0
```

Only **5 list items detected** vs **44 expected**:

- 4 references: [3], [8], [13], [36]
- 1 numbered list item: "3)"

### 4. Column Detection Log

```
BlockMerge: Processing 57 blocks with 2 columns
Column 0 bbox: x1=0.0 y1=0.0 x2=300.0 y2=792.0
Column 1 bbox: x1=300.0 y1=0.0 x2=612.0 y2=792.0
```

Column detection IS working, but references still merging cross-column.

## Technical Diagnosis

### Pipeline Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ PDF Parsing │ ──▶ │    Line     │ ──▶ │   Block     │ ──▶ │ Structure   │
│ (lopdf)     │     │  Grouping   │     │  Building   │     │ Detection   │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                           │                   │
                           │ PROBLEM 1         │ PROBLEM 2
                           │ Lines merged      │ Blocks span
                           │ across columns    │ columns
                           ▼                   ▼
                    ┌───────────────────────────────────┐
                    │ References from col1 + col2       │
                    │ merged into single paragraphs     │
                    └───────────────────────────────────┘
```

### Key Files

- `src/backend/text_grouping.rs` - Line grouping
- `src/backend/block_builder.rs` - Block construction
- `src/processors/structure_detection.rs` - List detection
- `src/processors/block_merge.rs` - Column-aware merging

### Hypothesis

The issue is not in list pattern detection (which works - we added `^\[\d{1,3}\]\s*`).

The issue is in **block boundary detection**:

1. Multi-line references are being merged into single paragraphs
2. Column boundary not respected during line grouping
3. Reference items from both columns interleaved

## Measurement

Extracted 5 of 44 list items = **11.4% detection rate**

Need to find where reference blocks get merged and prevent it.

## Next Steps (Orient)

1. Examine block_merge.rs for cross-column merging logic
2. Check if hyphen continuation is causing unwanted merges
3. Look for line grouping thresholds that may be too aggressive
4. Consider adding reference-specific block splitting
