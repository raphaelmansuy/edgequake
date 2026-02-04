# OODA-23 Orient: Root Cause Analysis

## Problem Statement

Despite correct two-column detection and reading order sorting, output shows:

1. Sentence fragments ("reposito-" ... "ries remains")
2. Figure captions interrupting prose
3. Affiliations appearing mid-document

## First Principles Analysis

```
                    PDF Two-Column Layout
                           │
           ┌───────────────┼───────────────┐
           │               │               │
    Left Column      Figure Area      Right Column
           │               │               │
     Para 1, sent 1   Fig Caption    Para 1 cont'd
     Para 1, sent 2        │         Para 2
           │               │               │
           └───────────────┼───────────────┘
                           │
                Current Extraction Flow
                           │
    ┌──────────────────────┼──────────────────────┐
    │                      │                      │
    Step 1: Extract     Step 2: Detect     Step 3: Merge
    Text Elements       Columns             Blocks
    │                      │                      │
    Lines ordered       Split at X=295     Merge adjacent
    by PDF stream                          blocks in same
    order                                  column
    │                      │                      │
    └──────────────────────┼──────────────────────┘
                           │
                    PROBLEM: Spanning Elements
                           │
            Figure captions span both columns
            but are assigned to ONE column
            based on their center X position
                           │
                    Results in:
            - Figures inserted mid-paragraph
            - Sentence continuations broken
            - Footnotes at wrong positions
```

## Root Causes Identified

### 1. Hyphenation Breaking

**Symptom**: "reposito-" on one line, "ries" on next
**Location**: Block merge processor
**Issue**: Hyphenated words at end of lines aren't being joined

**Fix Needed**: Detect trailing hyphen at end of block and merge with next block's first word

### 2. Spanning Element Positioning

**Symptom**: Figure caption appears between sentence fragments
**Location**: `reading_order.rs` `merge_column_orders()`
**Issue**: Spanning elements are inserted based on Y position, but should be inserted at paragraph boundaries

**Fix Needed**: Delay spanning elements until end of current paragraph

### 3. Block Segmentation at Column Boundary

**Symptom**: Text continues from left column to right but blocks are separated
**Location**: `text_grouping.rs` column detection
**Issue**: Some sentences span across columns in the original PDF layout

**Fix Needed**: Better sentence boundary detection

## Impact Analysis

| Issue                  | Impact on TPS                            | Difficulty | Priority |
| ---------------------- | ---------------------------------------- | ---------- | -------- |
| Hyphenation            | -3% (word fragments count as mismatches) | Low        | HIGH     |
| Spanning elements      | -2% (extra words in wrong position)      | Medium     | MEDIUM   |
| Cross-column sentences | -5% (sentence structure broken)          | High       | LOW      |

## Technical Deep Dive: Hyphenation

PDF stores text as positioned glyphs. When text wraps with hyphenation:

- Word "repositories" becomes "reposito-" + "ries"
- These are on different lines at different Y positions
- Current block merger sees them as separate blocks

**Detection heuristic**:

```
if block.text.ends_with('-')
   && next_block starts_with lowercase
   && vertical_gap < 2 * line_height:
    MERGE and remove hyphen
```

## Decision

Focus OODA-23 on implementing **hyphenation merge fix** as it has:

- Highest TPS impact (+3%)
- Lowest implementation complexity
- Clear, testable behavior
