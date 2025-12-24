# Camera Focus Fix - Detailed Implementation

← [Back to Index](./00-index.md) | [Implementation Plan](./02-implementation-plan.md) →

## Problem Summary

When clicking "Focus on Selected Node" in the Knowledge Graph, the camera zooms to an empty area instead of centering on the selected node.

## Technical Analysis

### Sigma.js Camera System

Sigma.js uses a camera system where:

- **Graph coordinates** (`x`, `y`): The actual position of nodes in the graph (can be any range, e.g., -100 to +100)
- **Camera coordinates** (`x`, `y`): Normalized values representing what portion of the graph is centered in the viewport (0.5, 0.5 = center)
- **ratio**: Zoom level (lower = more zoomed in)

### The Bug

**Current code passes graph coordinates directly to camera.animate():**

```tsx
// WRONG: Graph coordinates passed to camera
sigmaInstance.getCamera().animate({
  x: graph.getNodeAttribute(nodeId, "x"), // e.g., -50
  y: graph.getNodeAttribute(nodeId, "y"), // e.g., 75
  ratio: 0.3,
});
```

This results in the camera trying to center on position `-50, 75` in camera space, which is far outside the normal `0-1` viewport range.

## Solution

### Option A: Normalize coordinates using graph bounding box

```tsx
const handleFocusOnNode = useCallback(() => {
  if (!sigmaInstance || !selectedNodeId) return;

  const graph = sigmaInstance.getGraph();
  if (!graph.hasNode(selectedNodeId)) return;

  // Get node position in graph coordinates
  const nodeX = graph.getNodeAttribute(selectedNodeId, "x") as number;
  const nodeY = graph.getNodeAttribute(selectedNodeId, "y") as number;

  // Get graph bounding box (extent of all nodes)
  const { x: xExtent, y: yExtent } = sigmaInstance.getBBox();

  // Calculate normalized position (0-1 range)
  const graphWidth = xExtent[1] - xExtent[0];
  const graphHeight = yExtent[1] - yExtent[0];

  // Avoid division by zero
  const normalizedX = graphWidth > 0 ? (nodeX - xExtent[0]) / graphWidth : 0.5;
  const normalizedY =
    graphHeight > 0 ? (nodeY - yExtent[0]) / graphHeight : 0.5;

  // Animate camera to focus on node
  sigmaInstance.getCamera().animate(
    {
      x: normalizedX,
      y: normalizedY,
      ratio: 0.3, // Zoom in
    },
    { duration: 500 }
  );

  // Highlight the node
  graph.setNodeAttribute(selectedNodeId, "highlighted", true);
  sigmaInstance.refresh();
}, [sigmaInstance, selectedNodeId]);
```

### Option B: Use Sigma's graphToViewport (more accurate)

```tsx
const handleFocusOnNode = useCallback(() => {
  if (!sigmaInstance || !selectedNodeId) return;

  const graph = sigmaInstance.getGraph();
  if (!graph.hasNode(selectedNodeId)) return;

  const nodeX = graph.getNodeAttribute(selectedNodeId, "x") as number;
  const nodeY = graph.getNodeAttribute(selectedNodeId, "y") as number;

  // Convert to viewport coordinates first
  const viewportPos = sigmaInstance.graphToViewport({ x: nodeX, y: nodeY });
  const dims = sigmaInstance.getDimensions();

  // Get current camera state
  const camera = sigmaInstance.getCamera();
  const currentState = camera.getState();

  // Calculate offset from center
  const centerX = dims.width / 2;
  const centerY = dims.height / 2;

  // How far is the node from viewport center (in pixels)?
  const offsetX = (viewportPos.x - centerX) / dims.width;
  const offsetY = (viewportPos.y - centerY) / dims.height;

  // Adjust camera position
  camera.animate(
    {
      x: currentState.x + offsetX * currentState.ratio,
      y: currentState.y + offsetY * currentState.ratio,
      ratio: 0.3,
    },
    { duration: 500 }
  );

  graph.setNodeAttribute(selectedNodeId, "highlighted", true);
  sigmaInstance.refresh();
}, [sigmaInstance, selectedNodeId]);
```

### Selected Solution: Option A

Option A is simpler and works reliably with Sigma's coordinate system. It normalizes the node position within the graph's bounding box, which directly maps to camera coordinates.

## File to Modify

**File:** [zoom-controls.tsx](../edgequake_webui/src/components/graph/zoom-controls.tsx)

**Lines:** 102-121

## Complete Fixed Function

```tsx
const handleFocusOnNode = useCallback(() => {
  if (!sigmaInstance || !selectedNodeId) return;

  const graph = sigmaInstance.getGraph();
  if (!graph.hasNode(selectedNodeId)) return;

  try {
    // Get node position in graph coordinates
    const nodeX = graph.getNodeAttribute(selectedNodeId, "x") as number;
    const nodeY = graph.getNodeAttribute(selectedNodeId, "y") as number;

    // Get graph bounding box (extent of all nodes)
    const bbox = sigmaInstance.getBBox();

    // Calculate normalized position (0-1 range)
    const graphWidth = bbox.x[1] - bbox.x[0];
    const graphHeight = bbox.y[1] - bbox.y[0];

    // Handle edge case of zero dimensions (single node)
    const normalizedX = graphWidth > 0 ? (nodeX - bbox.x[0]) / graphWidth : 0.5;
    const normalizedY =
      graphHeight > 0 ? (nodeY - bbox.y[0]) / graphHeight : 0.5;

    // Animate camera to focus on node with smooth easing
    sigmaInstance.getCamera().animate(
      {
        x: normalizedX,
        y: normalizedY,
        ratio: 0.4, // Good zoom level for focus
      },
      {
        duration: 500,
        easing: "quadraticInOut",
      }
    );

    // Highlight the selected node
    graph.setNodeAttribute(selectedNodeId, "highlighted", true);
    sigmaInstance.refresh();
  } catch (error) {
    console.error("Error focusing on node:", error);
  }
}, [sigmaInstance, selectedNodeId]);
```

## Testing Checklist

- [ ] Select a node in the center of the graph → Camera should center on it
- [ ] Select a node at the edge of the graph → Camera should smoothly pan to it
- [ ] Select a node and focus multiple times → Should be idempotent
- [ ] Focus on node then reset view → Both should work correctly
- [ ] Test with single-node graph → Should center without errors
