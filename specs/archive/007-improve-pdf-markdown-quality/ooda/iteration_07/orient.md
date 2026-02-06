# Iteration 07: ORIENT - Analysis

## Problem Statement

Three-column PDFs fail because `detect_columns()` only searches for ONE gutter near center (50%).
Three-column layouts have gutters at ~33% and ~66%, which are outside the ±20% search range.

## Impact

From mission file quality targets:

- Multi-column: **60/100 → 85/100** (Critical priority)

Affects:

- Academic papers (commonly 2+ columns)
- Newsletters (often 3 columns)
- Technical documents

## Algorithm Comparison

### Current (Broken for 3+ columns)

```
┌────────────────────────────────────────────────────┐
│                 CURRENT ALGORITHM                  │
│                                                    │
│  Search zone: center ± 20%                         │
│  ├─────────────┼─────────────┤                     │
│  0%           50%          100%                    │
│               ▲                                    │
│           search here only                         │
│                                                    │
│  3-column gutters at 33% and 66% → MISSED!         │
└────────────────────────────────────────────────────┘
```

### Proposed (Full-width scan)

```
┌────────────────────────────────────────────────────┐
│                PROPOSED ALGORITHM                  │
│                                                    │
│  Scan entire width for gaps                        │
│  │  Col1  │  Col2  │  Col3  │                      │
│  0%      33%      66%     100%                     │
│          ▲        ▲                                │
│       gutter1  gutter2                             │
│                                                    │
│  Returns: [(0,33), (33,66), (66,100)]              │
└────────────────────────────────────────────────────┘
```

## Implementation Plan

1. Replace single-gutter detection with histogram-based approach
2. Build histogram of line right-edges (where text stops)
3. Find consistent gaps (where no text exists)
4. Each gap becomes a column boundary

## Risk Assessment

| Change         | Risk   | Mitigation                              |
| -------------- | ------ | --------------------------------------- |
| Break 2-column | Medium | Histogram will still find center gutter |
| False gutters  | Low    | Require minimum line count on each side |
| Performance    | Low    | O(n) histogram construction             |
