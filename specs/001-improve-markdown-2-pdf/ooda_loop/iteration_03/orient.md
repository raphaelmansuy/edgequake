# OODA Iteration 03: Orient

**Date**: 2026-02-06
**Mission Re-read**: Confirmed

## Analysis

Changing `BlockType::ListItem` to `BlockType::ListItem(u8)` would touch 30+ references across the codebase — high churn, high regression risk, and conflates block classification with rendering metadata.

The better approach mirrors pymupdf4llm: compute list levels as a **post-processing step** that produces a separate `HashMap<usize, u8>` mapping block index to level. The renderer consumes this map without modifying the core data model.

## First Principles

- **Spatial position determines hierarchy**: `x0` (left edge) is the ground truth for nesting. Items further right are nested deeper.
- **Separation of concerns**: Block classification (what it is) should remain separate from rendering metadata (how deep it is).
- **Minimal invasiveness**: A new module with a pure function has zero risk to existing tests. The renderer calls it once and threads the result through.
- **Segment isolation**: Levels reset at segment boundaries (non-list items, column breaks). This prevents unrelated list groups from inheriting each other's hierarchy.

## Risk Assessment

| Risk | Probability | Mitigation |
|------|-------------|------------|
| Wrong level assignment | Low | 10pt threshold matches pymupdf4llm exactly |
| Performance overhead | Negligible | O(n log n) sort on small segments |
| Break existing tests | None | Additive change, flat lists default to level 0 |
