# OODA-10: Observe

## Mission Reminder

**Target**: 95%+ quality metrics on all measures
**Current**: Overall 75.4%, Structural Fidelity 69.0%, Text Preservation 81.9%
**Gap**: Need 20+ percentage points improvement

## Current Quality Metrics

```
╔══════════════════════════════════════════════════════════════════╗
║  Comprehensive Quality Evaluation Results                        ║
╚══════════════════════════════════════════════════════════════════╝

📄 ccn_2512.21804v1       - Text:  77.2% | Structure:  71.9% | Overall:  74.5%
📄 2900_Goyal_et_al       - Text:  87.6% | Structure:  80.8% | Overall:  84.2%
📄 v2_2512.25072v1        - Text:  83.9% | Structure:  47.2% | Overall:  65.5%  ← WORST
📄 AlphaEvolve            - Text:  84.7% | Structure:  76.2% | Overall:  80.5%
📄 agent_2510.09244v1     - Text:  78.7% | Structure:  77.6% | Overall:  78.1%
📄 01_2512.25075v1        - Text:  79.4% | Structure:  50.4% | Overall:  64.9%  ← SECOND WORST
📄 one_tool_2512.20957v2  - Text:  81.6% | Structure:  78.8% | Overall:  80.2%

Averages:
   Text Preservation:    81.9%
   Structural Fidelity:  69.0%
   Overall Quality:      75.4%
```

## Root Cause Investigation

### Issue 1: Word Fragments Being Incorrectly Joined

**Observed in output**:
```
"whole-body loco-manipulation forwhiteboard wiping"
                               ^^^ MISSING SPACE
```

**Source location**: `src/schema/block.rs:339-349`

```rust
if is_same_visual_line && is_close_horizontally {
    let is_likely_word_fragment = matches!(
        (last_char, first_char),
        (Some(c1), Some(c2)) if c1.is_alphabetic() && c2.is_lowercase()
    ) && !self.text.trim_end().ends_with(' ');

    if is_likely_word_fragment {
        self.text = self.text.trim_end().to_string();
        self.text.push_str(other.text.trim_start());  // <-- NO SPACE!
```

**Problem**: 
- "for" ends with "r" (alphabetic)
- "whiteboard" starts with "w" (lowercase)
- Both conditions satisfied → incorrectly treated as word fragment

**Why this is wrong**:
The heuristic assumes `alphabetic + lowercase = continuation`, but this fails for common word boundaries like:
- "for whiteboard" → "forwhiteboard" ❌
- "the same" → "thesame" ❌
- "is critical" → "iscritical" ❌

### Issue 2: Aggressive Hyphen Removal

**Observed in output**:
```
"long-horizon tasks" rendered as "longhorizon tasks"
     ^ HYPHEN LOST
```

**Source location**: `src/schema/block.rs:330-332`

```rust
if ends_with_hyphen && starts_with_lowercase {
    // Explicit hyphenation: remove hyphen and join
    self.text = self.text.trim_end_matches('-').trim_end().to_string();
    self.text.push_str(other.text.trim_start());
}
```

**Problem**: The code assumes ALL hyphens at end-of-line are word-continuation hyphens:
- "modifi-" + "cation" → "modification" ✅ CORRECT
- "long-" + "horizon" → "longhorizon" ❌ WRONG (should be "long-horizon")

**Why this is wrong**:
Compound words like "long-horizon", "hand-eye", "self-supervised" have intentional hyphens.
The heuristic needs to distinguish:
- Word-continuation hyphen: "modifi-" → partial word being broken
- Compound-word hyphen: "long-" → complete morpheme with intentional hyphen

### Issue 3: Missing Paragraph Breaks

**Observed in output**:
```
"yet achieving robust whole-body coordination across the head, hands, and legs
remains a major challenge. We present a system that combinesa modular..."
                                                     ↑
                         MISSING LINE BREAK / RUN-ON TEXT
```

**Analysis**:
Blocks from different columns or paragraphs are being merged without paragraph separation.
The `should_merge()` function may be too permissive in its vertical gap threshold.

## Data Flow Analysis

```
┌─────────────────────────────────────────────────────────────────────┐
│                        PDF Extraction Pipeline                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  1. ContentParser                                                    │
│     └── Extracts raw text elements with X, Y coordinates             │
│                                                                      │
│  2. ElementProcessing::merge()                                       │
│     └── Merges horizontally adjacent elements on same line           │
│     └── OODA-09: Fixed column boundary detection                     │
│                                                                      │
│  3. TextGrouping                                                     │
│     └── Groups elements into lines                                   │
│     └── Handles two-column layout separation                         │
│                                                                      │
│  4. reading_order::multi_column_order()                              │
│     └── Orders blocks: left column first, then right column          │
│                                                                      │
│  5. BlockMergeProcessor::should_merge()                              │
│     └── Decides if consecutive blocks should merge                   │
│     └── Issue: May be too permissive with vertical gap               │
│                                                                      │
│  6. Block::merge()         ← CRITICAL BUG LOCATION                   │
│     └── Combines text from two blocks                                │
│     └── BUG 1: Word fragment detection too aggressive                │
│     └── BUG 2: Hyphen removal too aggressive                         │
│                                                                      │
│  7. MarkdownRenderer                                                 │
│     └── Renders blocks to markdown with proper formatting            │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Key Files Affected

| File | Lines | Issue |
|------|-------|-------|
| `src/schema/block.rs` | 339-349 | Word fragment detection incorrectly joins words |
| `src/schema/block.rs` | 330-332 | Hyphen removal too aggressive for compound words |
| `src/processors/layout_processing.rs` | 240-250 | Vertical gap threshold may be too permissive |

## Next Steps

1. Fix `Block::merge()` word fragment detection
2. Fix `Block::merge()` hyphen handling for compound words
3. Verify structural fidelity improvements
4. Run comprehensive quality tests
