# Visual & Interaction Audit: EdgeQuake Knowledge Graph

> **Document:** 03-visual-interaction-audit.md  
> **Last Updated:** 2025-12-30

---

## 1. Critical Issue: Responsive Layout Broken

### 1.1 Problem Description

**Severity:** 🔴 P0 - Critical  
**Affected Breakpoints:** Tablet (768px), Mobile (375px)

The knowledge graph canvas is **completely invisible** on tablet and mobile viewports. Users cannot see or interact with the graph on these devices.

### 1.2 Root Cause Analysis

**Layout Structure:**

```tsx
// graph-viewer.tsx
<div className="flex h-full overflow-hidden">
  {/* Left Panel: EntityBrowserPanel - w-64 (256px) */}
  <EntityBrowserPanel />

  {/* Main Graph Area - flex-1 */}
  <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
    {/* ... */}
  </div>

  {/* Right Panel: ResizablePanel - defaultWidth={320} */}
  <ResizablePanel ... defaultWidth={320} />
</div>
```

**Issue:** On 768px viewport:

- Left panel: 256px fixed
- Right panel: 320px default
- **Remaining for graph: 192px** (too narrow, or panels overlap)

On 375px viewport:

- Both panels combined: 576px
- Viewport: 375px
- **Graph area: 0px or negative**

### 1.3 Recommended Fix

```tsx
// graph-viewer.tsx - Add responsive panel behavior

import { useMediaQuery } from '@/hooks/use-media-query';

export function GraphViewer() {
  // Responsive breakpoints
  const isMobile = useMediaQuery('(max-width: 640px)');
  const isTablet = useMediaQuery('(max-width: 1024px)');

  // Track panel states (may need to persist)
  const {
    leftPanelCollapsed,
    rightPanelCollapsed,
    setLeftPanelCollapsed,
    setRightPanelCollapsed
  } = useUIPreferencesStore();

  // Auto-collapse panels on smaller screens
  useEffect(() => {
    if (isMobile) {
      setLeftPanelCollapsed(true);
      setRightPanelCollapsed(true);
    } else if (isTablet) {
      // On tablet, collapse right but allow left
      setRightPanelCollapsed(true);
    }
  }, [isMobile, isTablet]);

  return (
    <div className="flex h-full overflow-hidden">
      {/* Left Panel - Hidden on mobile */}
      {!isMobile && <EntityBrowserPanel />}

      {/* Main Graph Area */}
      <div className={cn(
        "flex-1 flex flex-col overflow-hidden",
        "min-w-[200px]",  // Ensure minimum width
        isMobile && "w-full"  // Full width on mobile
      )}>
        {/* Mobile toggle for panels */}
        {isMobile && (
          <div className="absolute top-2 left-2 z-20 flex gap-1">
            <Button
              variant="secondary"
              size="icon"
              className="h-8 w-8"
              onClick={() => setLeftPanelCollapsed(!leftPanelCollapsed)}
            >
              <Menu className="h-4 w-4" />
            </Button>
          </div>
        )}

        {/* ... rest of graph area */}
      </div>

      {/* Right Panel - Hidden on mobile */}
      {!isMobile && (
        rightPanelCollapsed ? (
          <CollapsedRightPanel ... />
        ) : (
          <ResizablePanel ... />
        )
      )}

      {/* Mobile slide-over panels */}
      {isMobile && (
        <>
          <MobileEntityDrawer
            open={!leftPanelCollapsed}
            onClose={() => setLeftPanelCollapsed(true)}
          />
          <MobileDetailsDrawer
            open={!rightPanelCollapsed}
            onClose={() => setRightPanelCollapsed(true)}
          />
        </>
      )}
    </div>
  );
}
```

**useMediaQuery Hook:**

```tsx
// hooks/use-media-query.ts
import { useState, useEffect } from "react";

export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(false);

  useEffect(() => {
    const media = window.matchMedia(query);
    if (media.matches !== matches) {
      setMatches(media.matches);
    }

    const listener = () => setMatches(media.matches);
    media.addEventListener("change", listener);

    return () => media.removeEventListener("change", listener);
  }, [matches, query]);

  return matches;
}
```

### 1.4 Acceptance Criteria

| Breakpoint | Expected Behavior                                       |
| ---------- | ------------------------------------------------------- |
| 1440px+    | Both panels visible, graph fills remaining space        |
| 1024px     | Left panel visible, right panel collapsed, graph ~700px |
| 768px      | Both panels collapsed, graph fills viewport             |
| 375px      | Both panels as slide-over drawers, graph full screen    |

---

## 2. Desktop Layout Analysis (1440px)

### 2.1 Overall Structure

| Component           | Width  | Status             |
| ------------------- | ------ | ------------------ |
| Left Entity Browser | 256px  | ✅ Good            |
| Main Graph Canvas   | ~544px | ⚠️ Could be larger |
| Right Details Panel | 320px  | ✅ Good            |
| **Total**           | 1120px | ✅ Fits            |

### 2.2 Observed UI Elements

**Entity Browser Panel:**

- ✅ Grouped view by entity type
- ✅ List view option
- ✅ Search filter
- ✅ Sort by Name/Degree
- ✅ Entity count badges
- ✅ Connection strength indicator
- ✅ Keyboard navigation
- ✅ Panel collapse

**Graph Canvas:**

- ✅ Sigma.js rendering
- ✅ Node hover highlighting
- ✅ Node selection
- ✅ Node dragging
- ✅ Zoom controls
- ⚠️ No curved edges
- ⚠️ No node borders
- ⚠️ Layout causes brief freeze

**Toolbar:**

- ✅ Search
- ✅ Layout control
- ✅ Export
- ✅ Keyboard shortcuts help
- ✅ Guided tour trigger
- ✅ Refresh button
- ✅ Zoom in/out/reset

**Legend Overlay:**

- ✅ Entity type colors
- ✅ Toggle visibility per type
- ✅ Entity count badges
- ✅ Collapsible

**Details Panel:**

- ✅ Node properties display
- ✅ Copy to clipboard
- ✅ Expandable long values
- ✅ Related nodes list
- ✅ Entity edit dialog
- ⚠️ No expand/prune buttons
- ✅ Graph filters section

### 2.3 Interaction Patterns

| Interaction      | Behavior               | Quality              |
| ---------------- | ---------------------- | -------------------- |
| Node click       | Selects, shows details | ✅ Good              |
| Node hover       | Highlights + neighbors | ✅ Good              |
| Node drag        | Moves node position    | ✅ Good              |
| Node right-click | Context menu           | ✅ Good              |
| Canvas drag      | Pan view               | ✅ Good              |
| Scroll wheel     | Zoom in/out            | ✅ Good              |
| Double-click     | Zoom to node           | ⚠️ Not implemented   |
| Keyboard arrows  | Navigate nodes         | ✅ Good (in browser) |

---

## 3. Visual Quality Assessment

### 3.1 Node Rendering

**Current State:**

- Default Sigma node program (filled circles)
- No borders
- Type-based coloring
- Size based on degree

**LightRAG Reference:**

- NodeBorderProgram (white borders)
- More polished appearance
- Better node separation

**Improvement:**

```typescript
import { NodeBorderProgram } from '@sigma/node-border';

// In Sigma settings
nodeProgramClasses: {
  default: NodeBorderProgram,
},
```

### 3.2 Edge Rendering

**Current State:**

- Straight lines with arrows
- Weight-based thickness
- Uniform gray color

**LightRAG Reference:**

- Curved edges (EdgeCurvedArrowProgram)
- Better visual flow
- Easier to trace connections

**Improvement:**

```typescript
import { EdgeCurvedArrowProgram } from '@sigma/edge-curve';

// In Sigma settings
edgeProgramClasses: {
  curved: EdgeCurvedArrowProgram,
},
defaultEdgeType: 'curved',
```

### 3.3 Color Scheme

**Current Type Colors:**
| Type | Color | Hex |
|------|-------|-----|
| PERSON | Blue | #3b82f6 |
| ORGANIZATION | Green | #10b981 |
| LOCATION | Amber | #f59e0b |
| EVENT | Red | #ef4444 |
| CONCEPT | Purple | #8b5cf6 |
| DOCUMENT | Indigo | #6366f1 |
| DEFAULT | Slate | #64748b |

**Assessment:** ✅ Good contrast, distinguishable colors

### 3.4 Label Rendering

**Current State:**

- Hardcoded color (#374151)
- Fixed size (12px)
- Inter font

**Issues:**

- Not theme-aware (dark mode needs lighter labels)
- No grid optimization

**Improvement:**

```typescript
labelColor: {
  color: isDark ? '#e2e8f0' : '#1e293b',
},
labelGridCellSize: 60,
labelRenderedSizeThreshold: 12,
```

---

## 4. Animation & Motion Assessment

### 4.1 Current Animations

| Animation            | Duration | Easing  | Quality    |
| -------------------- | -------- | ------- | ---------- |
| Camera pan to node   | 500ms    | Default | ✅ Good    |
| Node hover highlight | Instant  | None    | ⚠️ Abrupt  |
| Panel collapse       | 200ms    | Default | ✅ Good    |
| Filter toggle        | 0ms      | None    | ⚠️ Jarring |

### 4.2 Missing Animations

| Animation              | Recommended Duration | Priority  |
| ---------------------- | -------------------- | --------- |
| Layout transition      | 300ms                | 🔴 High   |
| Node visibility change | 150ms                | 🟠 Medium |
| Edge visibility change | 150ms                | 🟠 Medium |
| Node size change       | 200ms                | 🟡 Low    |

### 4.3 Layout Animation Fix

```typescript
import { animateNodes } from "sigma/utils";

// When applying new layout
const positions = calculateNewPositions();
animateNodes(graph, positions, {
  duration: 300,
  easing: "quadraticInOut",
});
```

---

## 5. Accessibility Audit

### 5.1 Strengths

| Feature             | Implementation             | Score |
| ------------------- | -------------------------- | ----- |
| ARIA labels         | Comprehensive              | ✅ A  |
| Keyboard navigation | Entity browser + shortcuts | ✅ A  |
| Focus management    | Visible focus rings        | ✅ A  |
| Color contrast      | Good type colors           | ✅ A  |
| Screen reader text  | sr-only hints              | ✅ A  |

### 5.2 Gaps

| Feature                   | Current State    | Recommendation               |
| ------------------------- | ---------------- | ---------------------------- |
| Graph canvas keyboard nav | ❌ None          | Add arrow key node traversal |
| Skip to main content      | ❌ None          | Add skip link                |
| Reduced motion preference | ❌ Not respected | Check prefers-reduced-motion |
| High contrast mode        | ❌ Not supported | Add forced colors support    |

### 5.3 Recommended Accessibility Improvements

```tsx
// Respect reduced motion
const prefersReducedMotion = useMediaQuery("(prefers-reduced-motion: reduce)");

const animationDuration = prefersReducedMotion ? 0 : 300;

// In animations
animateNodes(graph, positions, {
  duration: animationDuration,
});
```

---

## 6. Touch Interaction Analysis

### 6.1 Current Touch Support

| Gesture        | Expected     | Actual                | Status |
| -------------- | ------------ | --------------------- | ------ |
| Single tap     | Select node  | Works                 | ✅     |
| Double tap     | Zoom in      | Not implemented       | ⚠️     |
| Pinch zoom     | Zoom in/out  | Works (Sigma default) | ✅     |
| Two-finger pan | Pan view     | Works                 | ✅     |
| Long press     | Context menu | Not implemented       | ⚠️     |
| Swipe          | Navigate     | Not implemented       | ⚠️     |

### 6.2 Touch Improvements

```typescript
// Long press for context menu on touch devices
let longPressTimer: NodeJS.Timeout | null = null;

sigma.on("downNode", (e) => {
  if ("touches" in e.original) {
    longPressTimer = setTimeout(() => {
      openContextMenu(e.node, e.x, e.y);
    }, 500);
  }
});

sigma.on("upNode", () => {
  if (longPressTimer) {
    clearTimeout(longPressTimer);
    longPressTimer = null;
  }
});
```

---

## 7. Performance During Interaction

### 7.1 Observed Issues

| Interaction   | Nodes | Observed Behavior | Target         |
| ------------- | ----- | ----------------- | -------------- |
| Layout change | 100   | ~500ms pause      | <100ms         |
| Layout change | 500   | ~2s freeze        | <200ms (async) |
| Filter toggle | Any   | Instant           | ✅ Good        |
| Node drag     | Any   | Smooth            | ✅ Good        |
| Zoom          | Any   | Smooth            | ✅ Good        |

### 7.2 Root Cause

Synchronous ForceAtlas2 layout blocks the main thread:

```typescript
// Current: Blocking
forceAtlas2.assign(graph, { iterations: 100 });
```

### 7.3 Solution

Web Worker-based layout (see [05-performance-report.md](./05-performance-report.md)):

```typescript
const supervisor = new FA2LayoutSupervisor(graph, settings);
supervisor.start(); // Non-blocking
```

---

## 8. Cross-Reference: LightRAG Visual Features

### 8.1 Features to Adopt

| Feature                 | LightRAG Implementation | EdgeQuake Gap |
| ----------------------- | ----------------------- | ------------- |
| Node borders            | NodeBorderProgram       | Missing       |
| Curved edges            | EdgeCurvedArrowProgram  | Missing       |
| Layout animation        | animateNodes() 300ms    | Missing       |
| Theme-aware labels      | labelColorDarkTheme     | Missing       |
| Label grid optimization | labelGridCellSize: 60   | Missing       |
| Fullscreen mode         | FullScreenControl       | Limited       |

### 8.2 EdgeQuake Advantages to Preserve

| Feature            | EdgeQuake         | LightRAG     |
| ------------------ | ----------------- | ------------ |
| Entity browser     | Rich grouped view | None         |
| Interactive legend | Toggle visibility | Display only |
| Community coloring | Cluster detection | None         |
| Context menu       | Rich actions      | None         |
| Keyboard shortcuts | Comprehensive     | Limited      |
| Guided tour        | Onboarding        | None         |
| Graph export       | PNG/JSON          | None         |

---

## 9. Summary of Recommendations

### Critical Fixes

1. **Responsive layout** - Graph invisible on tablet/mobile
2. **Web Worker layout** - UI freezes during layout
3. **Touch interactions** - Long press for context menu

### Visual Quality

4. **Node borders** - Add NodeBorderProgram
5. **Curved edges** - Add EdgeCurvedArrowProgram
6. **Theme-aware labels** - Dynamic colors
7. **Layout animation** - 300ms transitions

### Accessibility

8. **Graph keyboard nav** - Arrow key node traversal
9. **Reduced motion** - Respect user preference
10. **Skip link** - Add skip to main content

---

_Related Documents:_

- [01-executive-summary.md](./01-executive-summary.md)
- [02-architecture-comparison.md](./02-architecture-comparison.md)
- [06-recommendations-roadmap.md](./06-recommendations-roadmap.md)
