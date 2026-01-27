# OODA Loop 4 - OBSERVE (Truth Validation)

**Date:** 2026-01-03  
**Directory Scope:** `crates/edgequake-pdf/src/backend/`  
**Focus:** Validate metrics against ACTUAL markdown comparison (First Principles)

## Baseline Metrics (From Validator)

```
Table Accuracy:      2.4%
Style Accuracy:      31.1%
Composite Score:     32.4/100
```

## CRITICAL FIRST PRINCIPLES VALIDATION

### User's Challenge: "fix the metrics calculation if is not reflecting the truth"

Let me directly compare generated vs gold markdown to validate the 11.4% table accuracy claim for one_tool document.

### Direct Markdown Comparison

**GOLD (one_tool_2512.20957v2.gold.md lines 135-142):**

```markdown
| Agent Pipeline    | Model            | Function-level Recall | Funct Precision | Funct Sample-F1 | Funct IoU | File-level Recall | File Precision | File Sample-F1 | File IoU |
| ----------------- | ---------------- | --------------------- | --------------- | --------------- | --------- | ----------------- | -------------- | -------------- | -------- |
| **Closed-source** |                  |                       |                 |                 |           |                   |                |                |          |
| RepoSearcher      | Claude3.7-Sonnet | 66.80                 | 28.30           | 19.90           | 17.89     | 89.71             | 33.15          | 21.04          | 20.67    |
| RepoNavigator     | Claude3.7-Sonnet | 31.03                 | 31.72           | 34.43           | 30.22     | 72.26             | 75.95          | 73.01          | 71.37    |

...
```

**Structure:**

- 10 columns properly aligned
- Header row with column names
- Separator row with dashes
- Data rows with clean alignment
- **This is a PROPERLY FORMATTED TABLE**

**GENERATED (one_tool_2512.20957v2.mdf.gen lines 377-380):**

```markdown
| CoSIL                                                             | Training | Free | 48.61 | 13.40 | 19.81 | 12.12 | 78.35 |
| ----------------------------------------------------------------- | -------- | ---- | ----- | ----- | ----- | ----- | ----- |
| Agentless Training Free 25.20 14.30 16.14 12.28 75.65 19.76 29.88 | 19.30    |      |       |       |       |       |       |
| Orcaloca Training Free 29.92 20.98 22.77 18.92 52.17 52.15 50.93  | 48.72    |      |       |       |       |       |       |
```

**Problems:**

1. **WRONG STARTING ROW:** Table starts at "CoSIL" (mid-table data row), not the header
2. **WRONG COLUMN COUNT:** 8 columns instead of 10
3. **BROKEN CELL STRUCTURE:** "Agentless Training Free" should be 3 cells, not 1
4. **MISSING TEXT SPLITTING:** Line 379 has "Agentless Training Free 25.20..." all in first cell

### ROOT CAUSE ANALYSIS (First Principles)

The validator is CORRECT to give low scores. The generated table is fundamentally broken:

1. **Header Detection Failed:** Started extracting from wrong row
2. **Column Detection Failed:** Miscounted columns (8 vs 10)
3. **Cell Boundary Detection Failed:** Not splitting text properly into cells

### Why Previous Loops Failed to Fix This

- **Loop 1 (crossing_ratio):** Fixed lattice grid detection, but doesn't help with cell TEXT splitting
- **Loop 2 (containment):** Fixed which characters go in which cells, but doesn't fix WHICH CELLS EXIST
- **Loop 3 (decorative filter):** Cleaned up cell content, but doesn't fix cell boundaries

### The REAL Problem (First Principles Truth)

**The lattice detector finds the TABLE GRID correctly**, but then `extract_text_in_rect()` has two fundamental failures:

1. **Column Detection:** Not properly identifying where column boundaries are
2. **Text Tokenization:** Not splitting whitespace-separated text into separate cells

Looking at line 379:

```
| Agentless Training Free 25.20 14.30 16.14 12.28 75.65 19.76 29.88 | 19.30 |
```

This should be:

```
| Agentless | Training Free | 25.20 | 14.30 | 16.14 | 12.28 | 75.65 | 19.76 | 29.88 | 19.30 |
```

**The entire text string "Agentless Training Free 25.20..." is being placed in ONE CELL instead of being split across MULTIPLE CELLS.**

### Hypothesis (First Principles)

The lattice detector:

1. Finds the table grid lines correctly ✓
2. Creates cells from grid intersections ✓
3. BUT: When extracting text, it's grabbing ALL text within the cell's bounding box

**The problem:** The PDF has text elements positioned at different X-coordinates within the same lattice cell. The code should:

- Split text by X-coordinate clusters
- Assign each cluster to the appropriate column
- NOT just dump all text into one cell

### Validator Metrics Are CORRECT

The 11.4% table accuracy for one_tool is accurate because:

- Table detected: ✓ (1.0 F1 for detection)
- Cell content: ❌ (near 0% because all cells have wrong content)
- Average: ~11% seems reasonable

The 2.4% overall is also accurate:

- agent_2510: 0% (no tables detected at all - whitespace tables)
- one_tool: 11% (tables detected but cells broken)
- Others: similarly broken

## Action Plan (First Principles Fix)

### Priority 1: Fix Cell Text Splitting (THIS IS THE REAL PROBLEM)

**Current behavior:**

```rust
extract_text_in_rect(cell_bbox) -> "Agentless Training Free 25.20 14.30..."
```

**Needed behavior:**

```rust
extract_text_in_rect(cell_bbox) -> split by X-position -> ["Agentless", "Training Free", "25.20", ...]
Then match these tokens to column boundaries
```

### Priority 2: Fix Column Boundary Detection

The lattice creates cells, but the column X-coordinates might not align with where text is actually positioned.

### Priority 3: Whitespace Tables (Agent_2510)

Still needed, but SECONDARY to fixing the cell text splitting issue.

## Truth Statement (First Principles)

**The validator metrics are CORRECT.**  
**The code is BROKEN at a fundamental level - it does not split multi-column text within cells.**  
**Previous loops addressed symptoms (grid detection, character placement, decoration), not the root cause.**

This is why table accuracy is stuck at 2.4% - we've been optimizing the wrong thing.
