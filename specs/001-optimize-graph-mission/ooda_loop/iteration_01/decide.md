# OODA Iteration 01 - Decide

**Mission**: Optimize Knowledge Graph Display & UX
**Mission File**: `specs/001-optimize-graph-mission/mission.md`
**Date**: 2026-02-09

---

## Decisions

### Decision 1: Enforce 500 Node Hard Limit

**What**: Cap `maxNodes` at 500 in all code paths

**Where**:

1. `use-graph-store.ts` - Add `MAX_DISPLAY_NODES = 500` constant
2. `truncation-banner.tsx:38` - Change `10000` to `MAX_DISPLAY_NODES`
3. `graph-viewer.tsx:550` - Change `10000` to `MAX_DISPLAY_NODES`

**How**: Replace hardcoded 10000 with centralized constant

---

### Decision 2: Improve Label Visibility

**What**: Increase label density and always show high-degree node labels

**Where**:

1. `graph-renderer.tsx:475` - Change labelDensity from 0.3 to 0.6 for large graphs
2. `graph-renderer.tsx:476` - Change labelThreshold from 10 to 4

**How**: Update constants in adaptive settings

---

### Decision 3: Focus Camera on Search Result

**What**: After server search selects a node, focus camera on it

**Where**:

1. `graph-search.tsx:handleSelect` - Already has `focusCameraOnNode`, verify it works

**How**: Ensure camera focus happens after nodes are added to graph

---

### Decision 4: Fix Entity Expand with Fallback

**What**: If entity not found by ID, try searching by label

**Where**:

1. `entities.rs:get_entity_neighborhood` - Add fallback search

**How**: Wrap get_node check with search_nodes fallback

---

## Implementation Order

1. **Fix node limit** (5 min) - Immediate UX improvement
2. **Fix labels** (10 min) - Critical visibility fix
3. **Verify search camera** (5 min) - Already implemented, verify
4. **Fix entity expand** (15 min) - Backend change + test

---

## Testing Plan

1. Run `make dev` and load graph page
2. Verify node count ≤ 500
3. Verify labels visible without extreme zoom
4. Search for entity and verify camera centers on result
5. Right-click node and verify "Expand" works

---

## Acceptance Criteria

- [ ] Graph loads max 500 nodes (even after Load More)
- [ ] Labels visible at default zoom for at least top 20 nodes by degree
- [ ] Search: selecting result focuses camera
- [ ] Expand neighbors: works without "Entity not found" error

---

## Next Step

Act phase: Implement the decided changes.
