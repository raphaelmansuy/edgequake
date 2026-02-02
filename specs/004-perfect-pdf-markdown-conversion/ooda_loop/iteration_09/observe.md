# OODA-09 Observe: Cross-Column Text Merging Investigation

## Mission Reminder

Target: 95%+ quality metrics (currently 74.6% overall, 68.0% structural fidelity)
Focus: Structural fidelity is the main bottleneck

## Baseline Metrics (Feb 2, 2026)

| PDF                   | Text  | Structure | Overall |
| --------------------- | ----- | --------- | ------- |
| ccn_2512.21804v1      | 76.8% | 79.7%     | 78.2%   |
| 2900_Goyal_et_al      | 87.1% | 80.8%     | 83.9%   |
| **v2_2512.25072v1**   | 82.9% | **45.6%** | 64.2%   |
| AlphaEvolve           | 84.7% | 65.3%     | 75.0%   |
| agent_2510.09244v1    | 78.7% | 77.6%     | 78.1%   |
| **01_2512.25075v1**   | 79.9% | **48.1%** | 64.0%   |
| one_tool_2512.20957v2 | 78.9% | 78.8%     | 78.8%   |

**Averages:**

- Text Preservation: 81.3%
- Structural Fidelity: 68.0%
- Overall Quality: 74.6%

## Problem Evidence

Extracted from `v2_2512.25072v1.pdf`:

```markdown
Abstract- Humanoid robots hold great promise for oper-manipulate objects [1]. Achie
ving this level of dexterity and  
ating in human-centric environments, yet achieving robustflexibility, however, rema
ins highly challenging.
```

**Issues observed:**

1. `oper-manipulate` - Words from different columns merged without space
2. `robustflexibility` - Two words concatenated
3. `legsremains` - Column text merged incorrectly

## Code Path Analysis

```
┌─────────────────────────────────────────────────────────────────┐
│                    PDF Text Extraction Flow                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. extraction_engine.rs                                         │
│     └── extract_page() - extracts raw TextElements               │
│                                                                  │
│  2. text_grouping.rs                                             │
│     ├── group_into_lines() - groups elements into lines          │
│     │   └── group_two_column_layout() - handles 2-col docs       │
│     │       └── group_single_column_layout() - Y-band grouping   │
│     │           └── sort_line_by_runs() - X-proximity sorting    │
│     │                                                            │
│     └── merge_line() ← PROBLEM: Space insertion logic            │
│         └── Calculates word_gap_threshold based on typical_spacing│
│             - Uses 1.5x typical spacing as threshold             │
│             - May not account for column boundaries              │
│                                                                  │
│  3. block_builder.rs                                             │
│     └── build() - converts lines to blocks                       │
│         └── text_grouper.merge_line() called per line            │
│                                                                  │
│  4. BlockMergeProcessor                                          │
│     └── should_merge() - checks column boundaries                │
│         ⚠️ Has column awareness but operates AFTER line merging  │
│                                                                  │
│  5. MarkdownRenderer                                             │
│     └── render_page() - outputs final markdown                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Key Code Locations

### text_grouping.rs:599-700 - merge_line()

```rust
// word_gap_threshold = typical_spacing * 1.5
// Problem: typical_spacing calculated from all elements, not per-column
for (i, elem) in elements.iter().enumerate() {
    if spacing > effective_threshold && !starts_with_punct {
        text.push(' ');  // Space insertion decision
    }
    text.push_str(&elem.text);
}
```

### text_grouping.rs:475-527 - group_single_column_layout()

```rust
let y_tolerance = elem.font_size * 0.5;
if y_diff > y_tolerance {
    // New line
}
```

- Uses 0.5x font size as Y tolerance
- Adjacent elements with Y < tolerance are grouped together

## Hypothesis

**Primary Issue**: In two-column layouts, elements from different logical text runs are being merged into the same "line" because:

1. `group_single_column_layout()` uses Y-tolerance of 0.5x font_size
2. When left and right columns are processed sequentially (not interleaved), blocks from the same visual Y-band but different columns may end up in the same line
3. `merge_line()` then joins them without proper space detection

**Secondary Issue**: The space insertion in `merge_line()` uses `typical_spacing * 1.5` which may be too permissive for multi-column documents where elements from different columns have large X-gaps but similar Y-coordinates.

## Log Evidence

From extraction logs:

```
TG-TWOCOL: Using two-column layout with boundary=300.0
LEFT-COL: Y=386.0 X=64.0 'Imitation learning is a widely used approach for a'
GAP->RIGHT: Y=374.2 X=313.2 'Arm Control. Similar to HATO ['
LEFT-COL: Y=397.9 X=54.0 'skills from expert demonstrations. This paradigm i'
```

The logs show left and right column elements being processed separately, but the resulting blocks still have cross-column merging issues.

## Next Steps

1. Investigate why elements are being merged across column boundaries
2. Check if column detection is working correctly during line grouping
3. Consider adding column-awareness to `merge_line()` function
