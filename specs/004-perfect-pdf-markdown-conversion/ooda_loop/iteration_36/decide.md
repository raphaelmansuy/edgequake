# OODA Iteration 36 - Decide Phase

## Date: 2026-02-04

## Decision

### Option Analysis

| Option                                | Impact | Risk   | Effort |
| ------------------------------------- | ------ | ------ | ------ |
| A: Fix footer threshold               | Medium | Low    | Low    |
| B: Add "Figure N." pattern protection | High   | Low    | Medium |
| C: Investigate text ordering          | Medium | Medium | High   |

### Selected Approach: Option B - Protect Figure Captions

**Rationale:**

- Figure captions are high-value content (rich semantic information)
- The pattern "Figure N." is highly recognizable
- This fix would protect ALL figure captions, not just the missing ones
- Low risk of false positives

### Implementation Plan

1. In text_grouping.rs, add check before footer/header classification
2. If text starts with "Figure \d+." or "Table \d+.", do NOT classify as footer
3. This protects captions from being filtered out

### Code Change Location

[text_grouping.rs](edgequake/crates/edgequake-pdf/src/backend/text_grouping.rs)

Around line 200-250 where footer/header classification happens.

### Expected Outcome

- Figure 4 caption should appear in extraction
- Figure 7 caption should appear correctly
- No impact on other text (conservative change)

### Fallback Plan

If Option B doesn't fully resolve the issue:

1. Add debug logging to trace Figure caption elements
2. Check exact Y-position and font-size of missing captions
3. Adjust thresholds if needed

### Decision

**IMPLEMENT FIGURE CAPTION PROTECTION**

Add regex pattern check for "Figure \d+." and "Table \d+."
before classifying text as footer/header/affiliation.
