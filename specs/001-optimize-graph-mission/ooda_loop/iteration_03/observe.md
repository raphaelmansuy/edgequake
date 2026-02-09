# OODA Iteration 03 - Observe

**Mission**: Optimize Knowledge Graph Display & UX
**Mission File**: `specs/001-optimize-graph-mission/mission.md`
**Date**: 2026-02-09

---

## Re-read Mission Checklist

- [x] Graph loads ≤500 nodes initially → Verify implementation
- [ ] Expand neighbors works without errors → Needs testing
- [ ] Node labels visible on zoom → Verify settings
- [ ] Search queries server and centers view → Already implemented
- [ ] Loading time < 2 seconds for 500 nodes → Benchmark needed
- [ ] All tests passing → Run test suite

---

## Code Verification - Iteration 01/02 Changes

### 1. MAX_DISPLAY_NODES Constant ✅

**Location**: `edgequake_webui/src/stores/use-graph-store.ts:35`

```typescript
export const MAX_DISPLAY_NODES = 500;
```

**Status**: Constant exists and is exported.

### 2. Truncation Banner ✅

**Location**: `edgequake_webui/src/components/graph/truncation-banner.tsx:36`

```typescript
const newMax = Math.min(maxNodes * 1.5, 500);
```

**Status**: Load More capped at 500.

### 3. Graph Settings Panel ✅

**Location**: `edgequake_webui/src/components/graph/graph-settings-panel.tsx:13,94`

```typescript
import { useGraphStore, MAX_DISPLAY_NODES } from '@/stores/use-graph-store';
// ...
if (!isNaN(parsed) && parsed >= 100 && parsed <= MAX_DISPLAY_NODES) {
```

**Status**: localStorage validation uses MAX_DISPLAY_NODES.

### 4. Auto-optimize ✅

**Location**: `edgequake_webui/src/lib/graph/auto-optimize.ts:68-70`

```typescript
const maxLimits = {
  low: { maxNodes: 200, batchSize: 25, layoutIterations: 30 },
  medium: { maxNodes: Math.min(400, MAX_DISPLAY_NODES), batchSize: 50, ... },
  high: { maxNodes: MAX_DISPLAY_NODES, batchSize: 100, ... },
};
```

**Status**: All tiers capped at MAX_DISPLAY_NODES.

### 5. Label Visibility Settings ✅

**Location**: `edgequake_webui/src/components/graph/graph-renderer.tsx:465-468`

```typescript
const adaptiveLabelGridCellSize = isVeryLargeGraph ? 150 : (isLargeGraph ? 100 : 80);
const adaptiveLabelDensity = isVeryLargeGraph ? 0.6 : (isLargeGraph ? 0.7 : 0.8);
const adaptiveLabelThreshold = isVeryLargeGraph ? 4 : (isLargeGraph ? 3 : 2);
```

**Status**: Improved settings for better label visibility.

### 6. Entity Neighborhood Handler ✅

**Location**: `edgequake/crates/edgequake-api/src/handlers/entities.rs:796-820`

Multi-level lookup strategy:
1. Try normalized name
2. Try original name (special chars)
3. Fallback to search_nodes

**Status**: Robust entity lookup implemented.

### 7. Search Camera Focus ✅

**Location**: `edgequake_webui/src/components/graph/graph-search.tsx:309-319`

```typescript
const handleSelect = useCallback((nodeId: string) => {
  selectNode(nodeId);
  if (sigmaInstance) {
    focusCameraOnNode(sigmaInstance, nodeId, {
      ratio: 0.5,
      duration: 500,
      highlight: false,
    });
  }
}, [sigmaInstance, selectNode, onSelect]);
```

**Status**: Camera focus on search result implemented.

---

## Remaining Issues to Investigate

### Issue A: Graph Viewer Max Node Amplification

**Location**: `edgequake_webui/src/components/graph/graph-viewer.tsx`

Need to verify the "grow more" handler also uses MAX_DISPLAY_NODES.

### Issue B: Backend Graph Streaming

**Location**: `edgequake/crates/edgequake-api/src/handlers/graph.rs`

Need to verify backend enforces 500 node limit.

### Issue C: Slider Max Value in Settings Panel

**Location**: `edgequake_webui/src/components/graph/graph-settings-panel.tsx`

Need to verify slider max is capped at 500.

### Issue D: Accessibility Compliance

Mission requires WCAG compliance for:
- Keyboard navigation
- Screen reader support
- Color contrast

---

## Dependency Analysis

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Node Limit Enforcement Chain                    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  MAX_DISPLAY_NODES = 500 (use-graph-store.ts)                       │
│         │                                                            │
│         ├──> truncation-banner.tsx (Load More) ✅                    │
│         │                                                            │
│         ├──> graph-settings-panel.tsx (localStorage) ✅              │
│         │                                                            │
│         ├──> auto-optimize.ts (all tiers) ✅                        │
│         │                                                            │
│         ├──> graph-viewer.tsx (grow more) ❓ To verify              │
│         │                                                            │
│         └──> Backend graph.rs (streaming) ❓ To verify              │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Next Steps

1. Read graph-viewer.tsx to verify MAX_DISPLAY_NODES usage
2. Read graph.rs backend streaming handler
3. Check slider max value in settings panel
4. Assess accessibility compliance
5. Run E2E tests



