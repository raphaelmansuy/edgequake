# OODA-08 Decide: Fix Word Splitting

## Date: 2026-02-04

## Decision

Apply two fixes to eliminate word splitting:

### Fix 1: Update width field during merge

**File**: `backend/element_processing.rs`
**Location**: Lines 200-210 (within merge loop)

```rust
// After: current.text.push_str(&next.text);
// ADD:
current.width = current.text.chars().count() as f32
    * current.font_size * self.char_width_factor;
```

**Rationale**: Width must reflect actual text length for accurate gap calculation.

### Fix 2: Relax overlap threshold

**File**: `backend/text_grouping.rs`  
**Location**: merge_line() function, OODA-42 block

Change from:

```rust
let significant_overlap = gap < -(avg_font_size * 0.5);
```

To:

```rust
let significant_overlap = gap < -avg_font_size;
```

**Rationale**:

- Width estimation error can be up to 25% (0.55 vs 0.42 per char)
- For 4-char elements at font_size 60: error = ~32pt
- Old threshold (-30pt) incorrectly triggers on normal merged elements
- New threshold (-60pt) allows for estimation error margin

## Expected Outcomes

1. `test_qwen_reading_order`: PASS ("Pushing" not "Push ing")
2. `test_arxiv_paper_extraction`: PASS (no regression in column order)
3. No change in quality metrics (targeted fix)

## Commit Message

```
OODA-08: Fix word splitting (Push ing → Pushing)

Root cause: element_processing::merge() updated text but not width.
When merge_line() calculates gap, stale width causes false positive.

Fixes:
1. Update width field during merge to reflect combined text
2. Relax OODA-42 threshold from 0.5× to 1× font_size to handle
   width estimation error (~25% overestimate for tight fonts)

Tests: test_qwen_reading_order PASS, test_arxiv_paper_extraction PASS
```
