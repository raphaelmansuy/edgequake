# OODA Iteration 19 - Orient

## Analysis: Unifying Prose Detection

### Root Cause

The `structure_detection.rs` font-size-based heading detection (line ~372) lacks
the prose indicator check that `heading_classifier.rs` already has. This creates
a gap where prose text with large fonts gets classified as headings.

### Solution Strategy: Extract Shared Prose Detection

First Principles: A heading is a SHORT, DECLARATIVE label for a section.
Prose text contains articles, copulas, and connecting words that indicate
it's a sentence fragment, not a label.

```
┌──────────────────────────────────────────────────────────┐
│           PROSE INDICATOR DETECTION (FIRST PRINCIPLES)    │
├──────────────────────────────────────────────────────────┤
│                                                           │
│  "Introduction"          → No prose indicators → HEADING  │
│  "Methods and Results"   → "and" not in indicator list    │
│  "This is the second"    → "is" (pos 1) + "the" (lower)  │
│                            → PROSE → NOT heading           │
│  "What We Deliver"       → "We" (uppercase) → OK          │
│  "Architecture"          → Single word → HEADING           │
│                                                           │
│  Rule: If word[i] ∈ {is,the,a,an,it,this,that,as,are,   │
│         was} AND word[i+1] starts lowercase → PROSE       │
│                                                           │
└──────────────────────────────────────────────────────────┘
```

### Implementation Plan

1. Extract prose detection logic into a standalone function in `heading_classifier.rs`
2. Call it from `structure_detection.rs`'s `headingish` computation
3. This follows DRY principle (don't repeat yourself)
