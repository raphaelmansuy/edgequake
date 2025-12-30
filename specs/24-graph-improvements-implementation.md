# Implementation Plan: Knowledge Graph UI Improvements

> **Spec:** 24-graph-improvements-implementation.md  
> **Created:** 2025-12-30  
> **Status:** In Progress

---

## Overview

This plan implements the critical fixes identified in the LightRAG vs EdgeQuake audit:

1. **P0: Responsive Layout Fix** - Graph invisible on tablet/mobile
2. **P0: Web Worker Layouts** - Prevent UI freezes on large graphs
3. **P1: Visual Quality** - Curved edges, node borders, theme-aware labels
4. **P1: Layout Animations** - Smooth 300ms transitions

---

## Implementation Checklist

### Phase 1: Responsive Layout Fix (P0)

- [ ] 1.1 Update `graph-viewer.tsx` to use `useMediaQuery` hook
- [ ] 1.2 Auto-collapse left panel on tablet (< 1024px)
- [ ] 1.3 Auto-collapse right panel on tablet (< 1024px)
- [ ] 1.4 Hide both panels and use mobile drawers on mobile (< 640px)
- [ ] 1.5 Add mobile toggle buttons in toolbar
- [ ] 1.6 Create `MobileEntityDrawer` component
- [ ] 1.7 Create `MobileDetailsDrawer` component
- [ ] 1.8 Update legend to show on mobile via button toggle

### Phase 2: Visual Quality Improvements (P1)

- [ ] 2.1 Import `@sigma/node-border` and `@sigma/edge-curve`
- [ ] 2.2 Add `NodeBorderProgram` to Sigma settings
- [ ] 2.3 Add `EdgeCurvedArrowProgram` for curved edges
- [ ] 2.4 Make label colors theme-aware (dark/light)
- [ ] 2.5 Add label grid optimization

### Phase 3: Layout Animations (P1)

- [ ] 3.1 Import `animateNodes` from sigma utils
- [ ] 3.2 Apply 300ms animated transitions on layout change
- [ ] 3.3 Add smooth camera transitions

### Phase 4: Web Worker Layouts (P0)

- [ ] 4.1 Switch to `@react-sigma/core` wrapper (already in deps)
- [ ] 4.2 Implement `useWorkerLayoutForceAtlas2` from react-sigma
- [ ] 4.3 Add layout supervisor pattern for non-blocking layouts
- [ ] 4.4 Add play/pause toggle for continuous layout

### Phase 5: E2E Testing

- [ ] 5.1 Create E2E test for responsive behavior (768px, 375px)
- [ ] 5.2 Create E2E test for graph interactions
- [ ] 5.3 Verify all tests pass

---

## Detailed Implementation

### 1.1-1.5 Responsive Layout in graph-viewer.tsx

**Goal:** Make panels responsive to viewport size.

```tsx
// Add at top of GraphViewer
const isMobile = useMediaQuery("(max-width: 640px)");
const isTablet = useMediaQuery("(max-width: 1024px)");

// Track mobile drawer states
const [mobileEntityDrawerOpen, setMobileEntityDrawerOpen] = useState(false);
const [mobileDetailsDrawerOpen, setMobileDetailsDrawerOpen] = useState(false);

// Auto-collapse panels based on viewport
useEffect(() => {
  const { leftPanelCollapsed, setLeftPanelCollapsed, setRightPanelCollapsed } =
    useUIPreferencesStore.getState();
  if (isTablet && !leftPanelCollapsed) {
    setLeftPanelCollapsed(true);
  }
  if (isTablet) {
    setRightPanelCollapsed(true);
  }
}, [isTablet]);
```

### 2.1-2.5 Visual Quality in graph-renderer.tsx

```tsx
import { createNodeBorderProgram } from "@sigma/node-border";
import { EdgeCurvedArrowProgram } from "@sigma/edge-curve";

// In Sigma constructor
const sigma = new Sigma(graph, containerRef.current, {
  nodeProgramClasses: {
    border: createNodeBorderProgram({
      borders: [{ size: { value: 2 }, color: { attribute: "borderColor" } }],
    }),
  },
  defaultNodeType: "border",
  edgeProgramClasses: {
    curvedArrow: EdgeCurvedArrowProgram,
  },
  defaultEdgeType: "curvedArrow",
  labelColor: { color: isDark ? "#e2e8f0" : "#374151" },
  labelGridCellSize: 60,
});
```

### 3.1-3.3 Layout Animations

```tsx
import { animateNodes } from "sigma/utils";

// After calculating new positions
const positions = getNewLayoutPositions();
animateNodes(graph, positions, { duration: 300, easing: "quadraticInOut" });
```

---

## Execution Order

1. Start with Phase 1 (responsive) - most critical user-facing bug
2. Then Phase 2 (visual quality) - relatively simple changes
3. Then Phase 3 (animations) - quick win
4. Finally Phase 4 (web workers) - most complex but important
5. Phase 5 (E2E tests) - run throughout to verify

---

## Success Criteria

1. ✅ Graph visible and usable on 768px tablet viewport
2. ✅ Graph visible and usable on 375px mobile viewport
3. ✅ Panels collapse/expand via drawers on mobile
4. ✅ Curved edges render correctly
5. ✅ Node borders visible
6. ✅ Labels adapt to theme
7. ✅ Layout changes are animated
8. ✅ Layout calculation doesn't freeze UI
9. ✅ All E2E tests pass
