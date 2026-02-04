# OODA-17: Observe Phase

## Mission Status

- **Quality**: Text 85.7%, Structure 87.2%, Overall 86.5%
- **Target**: 95%+
- **Gap**: 8.5 percentage points

## Problem Identified

Multi-line titles are being split into separate blocks, then rendered as separate headings.

### Example: agent_2510.09244v1.pdf

**Expected**:

```markdown
# Fundamentals of Building Autonomous LLM Agents
```

**Actual**:

```markdown
# Fundamentals of Building Autonomous LLM

## Agents
```

### Root Cause Analysis

The PDF title wraps across two text lines:

```
Line 1: Y=0.0  X=152.9 font=14.3 "Fundamentals of Building Autonomous LLM"
Line 2: Y=17.9 X=277.8 font=14.3 "Agents"
```

Both lines are correctly detected as SPANNING (title zone + large font):

```
SPANNING: Y=0.0 X=152.9 font=14.3 title_zone=true large_font=true 'Fundamentals of Building Autonomous LLM'
SPANNING: Y=17.9 X=277.8 font=14.3 title_zone=true large_font=true 'Agents'
```

But they become TWO separate blocks because:

1. `group_single_column_layout()` groups by Y-coordinate
2. Each Y-level becomes a separate "line"
3. `block_builder.rs` creates one block per line
4. `BlockMergeProcessor` doesn't merge them because they're not strictly adjacent enough

### Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                        PDF Extraction                           │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                     TextGrouper                                  │
│  spanning_elements = [                                           │
│    {Y=0.0, font=14.3, "Fundamentals of Building Autonomous LLM"},│
│    {Y=17.9, font=14.3, "Agents"}                                 │
│  ]                                                               │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│              group_single_column_layout()                        │
│  spanning_lines = [                                              │
│    [Line 1: "Fundamentals..."],                                  │
│    [Line 2: "Agents"]                                            │
│  ]                                                               │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    BlockBuilder                                  │
│  blocks = [                                                      │
│    Block {text: "Fundamentals...", type: SectionHeader},         │
│    Block {text: "Agents", type: SectionHeader}      ← PROBLEM    │
│  ]                                                               │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                   MarkdownRenderer                               │
│  "# Fundamentals of Building Autonomous LLM"                     │
│  "## Agents"                          ← Should be joined!        │
└─────────────────────────────────────────────────────────────────┘
```

### Key Insight

The SPANNING elements should be MERGED before block creation, not after. These are visually contiguous title lines that should form a single block.

## Solution Options

### Option A: Merge in TextGrouper

Merge spanning lines with close Y-spacing and same font size before returning.

**Pros**: Early in pipeline, clean separation
**Cons**: More complex TextGrouper

### Option B: Add TitleMergeProcessor

Post-process blocks to merge consecutive title blocks on page 1.

**Pros**: Clear single responsibility, easy to test
**Cons**: Another processor in chain

### Option C: Modify BlockMergeProcessor

Add special handling for title blocks on first page.

**Pros**: Reuses existing processor
**Cons**: Adds complexity to existing code

## Recommendation

**Option A: Merge spanning lines in TextGrouper**

The spanning elements are already grouped together. We just need to merge lines that:

1. Have Y-spacing < 25pt (typical line spacing for titles)
2. Have same font size
3. Are in the title zone

This is the most efficient fix - prevents the problem rather than correcting it after.

## Metrics Impact

If we fix title merging:

- agent_2510.09244v1 Structure should improve (correct H1 title)
- Other documents with multi-line titles may also benefit
