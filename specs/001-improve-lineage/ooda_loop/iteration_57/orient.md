# Analysis - Iteration 57

## Global Fix vs Per-Instance Fix

### Option A: Apply `!block` to ALL ScrollAreas globally

Modify `scroll-area.tsx` to always render the viewport children with `display: block` override.

**Pros**: Prevents the issue globally, no need to add per-instance overrides.
**Cons**: May break intentional horizontal scroll behavior in some ScrollAreas (e.g., code blocks, tables).

### Option B: Keep per-instance fix (current approach)

Apply `!block` only where needed.

**Pros**: Surgical, doesn't risk breaking other ScrollAreas.
**Cons**: Requires manual application if new overflow comes up.

### Recommendation: Keep Option B (per-instance)

The Radix `display: table` wrapper serves a purpose — it enables horizontal scroll when `<ScrollBar orientation="horizontal" />` is present. We only want to override it when we know horizontal scroll is not needed.
