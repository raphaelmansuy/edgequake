# EdgeQuake WebUI Graph Visualization

> Deep dive into the WebGL graph rendering engine, layout algorithms, and interaction models.

**Version**: 1.0.0 | **Last Updated**: 2026-01-09

---

## 1. Visualization Stack

The Knowledge Graph view is powered by a high-performance stack capable of rendering 10k+ nodes:

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Data Structure** | `graphology` | Robust graph object model (MultiGraph compliant). |
| **Logic/Layout** | `graphology-layout-forceatlas2` | Physics simulation for node positioning. |
| **Rendering** | `sigma.js` (v3) | WebGL renderer optimized for React. |
| **Wrapper** | `@react-sigma/core` | React context provider and hooks for Sigma instance. |

---

## 2. Rendering Pipeline (`GraphRenderer`)

The `GraphRenderer` component is the orchestrator. It does not store data; instead, it subscribes to the `graph` object found in `useGraphStore`.

```tsx
// Simplified Architecture
<SigmaContainer settings={sigmaSettings}>
  <GraphEvents />       {/* Click/Hover Listeners */}
  <LayoutController />  {/* Physics Engine */}
  <GraphControls />     {/* HUD */}
</SigmaContainer>
```

### 2.1 Sigma Settings

We configure Sigma for maximum legibility and performance:
-   `renderEdgeLabels`: On hover only (expensive).
-   `labelRenderedSizeThreshold`: 6px (hides labels on zoom-out).
-   `zIndex`: True (sorts nodes by importance/degree).
-   `allowInvalidContainer`: True (prevents crashes during resize).

---

## 3. Layout Engine (`LayoutController`)

We use **ForceAtlas2**, a continuous physics simulation algorithm.

### 3.1 Web Worker Execution

The layout calculation is CPU-intensive (O(n²)). To keep the UI responsive (avoiding "Jank"), we run the layout in a **Web Worker**.

```typescript
// components/graph/layout-controller.tsx
const { start, stop } = useWorkerLayoutForceAtlas2({
  settings: {
    slowDown: 10,
    gravity: 1,
    scalingRatio: 2,
  }
});

// Auto-start on data change, stop after X seconds or stability
useEffect(() => {
  if (needsLayout) {
    start();
    setTimeout(stop, 2000); // Cool-down phase
  }
}, [graphHash]);
```

### 3.2 N-Overlap

After the physics simulation stabilizes, we run a "N-Overlap" pass to ensure nodes don't physically obscure each other, improving readability of labels.

---

## 4. Interaction Model (`GraphEvents`)

Interaction is handled separate from rendering to decouple event loops.

### 4.1 Click & Selection
-   **Click Node**: Sets `selectedNodeId` in store. Opens `NodeDetails` panel.
-   **Click Stage**: Clears selection.
-   **Right Click**: Opens `GraphContextMenu` (Expand, Hide, Focus).

### 4.2 Hover Effects
When hovering a node:
1.  Identify neighbors (in/out edges).
2.  Set `hoveredNodeId` in store.
3.  **Reducer Pattern**: Sigma uses a "reducer" function to visually mute non-neighbor nodes and highlight the active subgraph.

```typescript
// Visual Reducer Logic
sigma.setSetting('nodeReducer', (node, data) => {
  if (!hoveredNode) return data;
  if (node === hoveredNode || neighbors.has(node)) {
    return { ...data, color: data.color, zIndex: 10 };
  }
  return { ...data, color: '#eee', zIndex: 0, label: '' }; // Muted
});
```

---

## 5. Camera Management

We use `lib/graph/camera-utils.ts` to manage the viewport.

-   **Zoom to Fit**: Calculates bounding box of all visible nodes and animates camera.
-   **Focus Node**: Smoothly fliess the camera to coordinates (x,y) of a specific entity.

---

## 6. Clustering & Communities

Nodes are colored by their "Community" (detected by Louvain algorithm on the backend).

-   **Visuals**: Each community ID maps to a specific color palette (from Tailwind colors).
-   **Filtering**: Users can toggle visibility of entire communities via `GraphLegend`.

---

## 7. Performance Tips

1.  **Avoid React Re-renders**: Do not pass the `graph` object as a prop if possible. Use `flight-mode` (store references) where the Sigma instance reads directly from `graphology`.
2.  **Label Threshold**: Keep `labelRenderedSizeThreshold` high so labels don't clutter the view on zoom-out.
3.  **Edge Rendering**: Edge thickness and curved edges (bezier) are expensive. Use straight lines for datasets > 2000 edges.
