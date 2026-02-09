# OODA Iteration 01 - Observe

**Mission**: Optimize Knowledge Graph Display & UX
**Mission File**: `specs/001-optimize-graph-mission/mission.md`
**Date**: 2026-02-09

---

## Observation Findings

### Issue 1: Too Many Nodes (1700+ displayed)

**Location**: `edgequake_webui/src/stores/use-graph-store.ts:252`

```typescript
maxNodes: 200, // Initial value - CORRECT
```

**Problem**: The "Load More" feature in `truncation-banner.tsx:38` increases limit:

```typescript
const newMax = Math.min(maxNodes * 1.5, 10000); // Allows up to 10000!
```

**Also**: `graph-viewer.tsx:550`:

```typescript
useGraphStore.getState().setMaxNodes(Math.min(currentMax * 1.5, 10000));
```

**Impact**: Users can load up to 10000 nodes, far exceeding the 500 max requirement.

---

### Issue 2: Entity Expand Fails - "CRÉANCES_CLIENTS not found"

**Location**: `edgequake_webui/src/hooks/use-graph-expansion.ts:92`

```typescript
const neighborhood = await getEntityNeighborhood(nodeId, 1);
```

**API Call**: `/api/v1/graph/entities/${entityId}/neighborhood`

**Backend Handler**: `edgequake/crates/edgequake-api/src/handlers/entities.rs:796`

```rust
let entity_name = normalize_entity_name(&entity_name);
// normalize_entity_name() does: name.to_uppercase().replace(' ', "_")
```

**Root Cause**:

- Entity stored in DB might have accented characters stripped or handled differently
- The node ID in frontend (`CRÉANCES_CLIENTS`) may not match the normalized ID in storage
- URL encoding of special characters (É) might cause mismatches

---

### Issue 3: Labels Not Visible

**Location**: `edgequake_webui/src/components/graph/graph-renderer.tsx:470-476`

```typescript
const isVeryLargeGraph = nodeCount > 500 || edgeCount > 1000;

// When isVeryLargeGraph = true:
const adaptiveLabelGridCellSize = 200; // Very sparse grid
const adaptiveLabelDensity = 0.3; // Only 30% label density
const adaptiveLabelThreshold = 10; // High threshold - labels only at extreme zoom
```

**Impact**: With 1700 nodes, labels are essentially invisible unless zoomed very close.

---

### Issue 4: Search Doesn't Refresh Graph

**Location**: `edgequake_webui/src/components/graph/graph-search.tsx`

**Current Behavior**:

1. Client-side search via MiniSearch on loaded nodes
2. Server fallback when graph is truncated (recently added)
3. Server results added via `addNodesToGraph()` but graph not recentered

**Missing**: Focus camera on selected node after server search returns

---

### Issue 5: Node Limit Not Enforced at Backend

**Backend**: `edgequake/crates/edgequake-api/src/handlers/graph.rs`

Need to verify `max_nodes` parameter is properly enforced in streaming endpoint.

---

## Key Files Inventory

| File                     | Purpose          | Issue                              |
| ------------------------ | ---------------- | ---------------------------------- |
| `use-graph-store.ts`     | State management | maxNodes=200 but can grow to 10000 |
| `truncation-banner.tsx`  | Load More UI     | Amplifies maxNodes 1.5x            |
| `graph-viewer.tsx`       | Main viewer      | Also amplifies maxNodes            |
| `use-graph-expansion.ts` | Expand neighbors | Entity ID mismatch                 |
| `graph-renderer.tsx`     | Sigma.js setup   | Label LOD too aggressive           |
| `graph-search.tsx`       | Search component | No camera focus on result          |
| `entities.rs`            | API handlers     | normalize_entity_name() issue      |

---

## Measurements Taken

1. **Current node count**: 1708 (from screenshot)
2. **Max allowed by UX**: 10000 (from code)
3. **Desired max**: 500
4. **Label density at >500 nodes**: 0.3 (30%)
5. **Error seen**: `Entity 'CRÉANCES_CLIENTS' not found`

---

## Next Step

Orient phase: Analyze impact and design fixes for each issue.
