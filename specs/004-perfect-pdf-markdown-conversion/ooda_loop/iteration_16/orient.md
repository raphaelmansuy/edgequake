# OODA-16: Orient Phase

## First Principles Analysis

### The Core Question

Why do we skip multi-column pages for table detection?

**Original Reasoning**:

```
Multi-column text → blocks appear side-by-side at same Y
Table detection → looks for blocks at same Y in different X positions
Result → column text would be falsely detected as tables
```

**The Flaw**:
This reasoning conflates TWO different patterns:

1. **Column text**: Same Y, different X, BUT text flows TOP-TO-BOTTOM in each column
2. **Table rows**: Same Y, different X, AND content is SEMANTICALLY related left-to-right

### Distinguishing Tables from Columns

**Pattern Analysis**:

```
COLUMN TEXT (NOT a table):
┌─────────────────┬─────────────────┐
│ "Introduction"  │ "Results"       │  <- Same Y=50, different content
│ "This paper..." │ "We found..."   │  <- Same Y=70, different content
│ "discusses..."  │ "that the..."   │  <- Same Y=90, different content
└─────────────────┴─────────────────┘
Content is INDEPENDENT - left and right are different sections

TABLE (IS a table):
┌─────────────────┬─────────────────┐
│ "FunSearch"     │ "AlphaEvolve"   │  <- Header row (same Y)
│ "evolves func"  │ "evolves file"  │  <- Data row (same Y)
│ "Python only"   │ "any language"  │  <- Data row (same Y)
└─────────────────┴─────────────────┘
Content is RELATED - left and right are COMPARING same attributes
```

### Key Differentiating Signals

| Signal          | Column Text             | Table                         |
| --------------- | ----------------------- | ----------------------------- |
| Y-alignment     | Some overlap, not exact | Exact or near-exact Y match   |
| Row count       | Many (10+)              | Fewer (3-10 typical)          |
| Text length     | Long sentences          | Short cells                   |
| Content pattern | Flowing paragraphs      | Structured data               |
| Header presence | Usually no              | Often "Table N" or header row |

### Table 1 in AlphaEvolve - Detailed Analysis

From diagnose_tables output:

```
Block 15: y1=412.9 "FunSearch[83]"        Block 29: y1=412.9 "AlphaEvolve"
Block 16: y1=432.4 "evolves single..."    Block 30: y1=432.4 "evolves entire..."
Block 17: y1=459.5 "evolves code in..."   Block 31: y1=459.5 "evolves any..."
Block 18: y1=486.6 "millions of LLM..."   Block 32: y1=486.6 "thousands of..."
Block 19: y1=513.7 "minimal context..."   Block 33: y1=527.3 "can simultaneously..."
```

**Observations**:

1. **Exact Y-alignment**: y1=412.9, 432.4, 459.5, 486.6 match perfectly between columns
2. **Short text**: Each cell is <50 characters (typical table cells)
3. **Structured comparison**: Left describes FunSearch, right describes AlphaEvolve
4. **Row count**: 5 data rows (typical table size)

### Signal for Table Detection in Multi-Column Pages

**Heuristic**: A region is likely a TABLE if:

1. Blocks at same Y in different columns (already detected)
2. **Text is SHORT** (<100 characters per block)
3. **Rows are CONSISTENT** (3-10 rows with same pattern)
4. **Caption nearby** ("Table N" pattern)

### Risk Assessment

| Approach                   | Benefit                           | Risk                            |
| -------------------------- | --------------------------------- | ------------------------------- |
| Remove skip entirely       | Detects tables in multi-col       | May create false positives      |
| Add text length check      | Better precision                  | May miss tables with long cells |
| Look for "Table N" caption | Very precise                      | May miss unlabeled tables       |
| Require exact Y-match      | Distinguishes tables from columns | May be too strict               |

### Recommended Approach

**Enable table detection for multi-column pages, BUT add stricter criteria:**

1. **Require exact Y-alignment** (within 2pt) for "table rows"
   - Column text may have slight Y variations
   - Table cells are precisely aligned

2. **Check text length** (avg <100 chars per block in the row)
   - Tables have short cells
   - Paragraphs have long sentences

3. **Require 3+ consecutive aligned rows**
   - Prevents single-row false positives

4. **Optional: Look for nearby "Table" caption**
   - High-confidence signal when present

## Gap from Current State

**Current**: Skip all multi-column pages
**Needed**: Process multi-column pages with stricter table criteria

## Trade-offs

**Precision vs Recall**:

- More restrictive criteria → fewer false positives, may miss some tables
- Less restrictive → more tables detected, more false positives

**Recommended**: Start with strict criteria (exact Y, short text, 3+ rows) and relax if needed.
