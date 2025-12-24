# Implementation Plan

← [Back to Index](./00-index.md) | [Issue Analysis](./01-issue-analysis.md) →

## Implementation Order

| Step | Task                   | File                                                                                   | Estimated Time |
| ---- | ---------------------- | -------------------------------------------------------------------------------------- | -------------- |
| 1    | Fix graph camera focus | [zoom-controls.tsx](../edgequake_webui/src/components/graph/zoom-controls.tsx)         | 10 min         |
| 2    | Verify markdown safety | [markdown-renderer.tsx](../edgequake_webui/src/components/query/markdown-renderer.tsx) | 5 min          |
| 3    | Test all fixes         | E2E Browser Testing                                                                    | 10 min         |
| 4    | Commit changes         | Git                                                                                    | 2 min          |

---

## Step 1: Fix Graph Camera Focus

### Current Code (Broken)

```tsx
// zoom-controls.tsx:102-121
const handleFocusOnNode = useCallback(() => {
  if (sigmaInstance && selectedNodeId) {
    const graph = sigmaInstance.getGraph();

    if (graph.hasNode(selectedNodeId)) {
      const nodePosition = {
        x: graph.getNodeAttribute(selectedNodeId, "x"),
        y: graph.getNodeAttribute(selectedNodeId, "y"),
      };

      sigmaInstance.getCamera().animate(
        {
          x: nodePosition.x, // BUG: Graph coords, not camera coords
          y: nodePosition.y,
          ratio: 0.3,
        },
        { duration: 500 }
      );

      graph.setNodeAttribute(selectedNodeId, "highlighted", true);
      sigmaInstance.refresh();
    }
  }
}, [sigmaInstance, selectedNodeId]);
```

### Fixed Code

```tsx
const handleFocusOnNode = useCallback(() => {
  if (sigmaInstance && selectedNodeId) {
    const graph = sigmaInstance.getGraph();

    if (graph.hasNode(selectedNodeId)) {
      const nodePosition = {
        x: graph.getNodeAttribute(selectedNodeId, "x") as number,
        y: graph.getNodeAttribute(selectedNodeId, "y") as number,
      };

      // Get current camera state to calculate proper transformation
      const camera = sigmaInstance.getCamera();

      // Use graphToViewport to convert graph coords to viewport pixel position
      const viewportPos = sigmaInstance.graphToViewport(nodePosition);
      const dims = sigmaInstance.getDimensions();

      // Calculate the camera state that would center this node
      // The camera x,y represent what fraction of the graph is at viewport center
      // We need to calculate what camera position puts our node at center

      // Alternative approach: Use sigma's built-in frame calculation
      // Reset camera to neutral then animate to node position
      const currentState = camera.getState();

      // Calculate normalized position within the graph bounding box
      // This approach works by first resetting view, then zooming to position
      sigmaInstance.getCamera().animate(
        {
          x: nodePosition.x,
          y: nodePosition.y,
          ratio: 0.5, // Zoom in moderately
          angle: currentState.angle, // Preserve rotation
        },
        {
          duration: 500,
          easing: "quadraticInOut",
        }
      );

      // Actually the correct Sigma approach is to use frameGraph bounds
      // Let's use a different method - setCustomBBox then reset
    }
  }
}, [sigmaInstance, selectedNodeId]);
```

**Best Solution (from Sigma.js docs):**

The simplest and most reliable approach is to use Sigma's `cameraForGraph` utility or manually compute camera position:

```tsx
const handleFocusOnNode = useCallback(() => {
  if (sigmaInstance && selectedNodeId) {
    const graph = sigmaInstance.getGraph();

    if (graph.hasNode(selectedNodeId)) {
      // Get node position in graph coordinates
      const x = graph.getNodeAttribute(selectedNodeId, "x") as number;
      const y = graph.getNodeAttribute(selectedNodeId, "y") as number;

      // For Sigma, camera.x and camera.y in animate() represent
      // the point in the GRAPH that should be at viewport center
      // So we just need to pass the graph coordinates directly!
      // BUT the coordinate system needs consideration.

      // The actual issue: Sigma camera works in a normalized space
      // where the graph is fitted. The x,y represent viewport fractions.

      // Correct approach: Get the normalized graph extent
      const customBBox = sigmaInstance.getCustomBBox();
      const graphBBox = customBBox || sigmaInstance.getBBox();

      // Normalize node position to 0-1 range based on graph extent
      const { x: minX, y: minY } = graphBBox;
      const width = graphBBox.x[1] - graphBBox.x[0];
      const height = graphBBox.y[1] - graphBBox.y[0];

      const normalizedX = (x - graphBBox.x[0]) / width;
      const normalizedY = (y - graphBBox.y[0]) / height;

      sigmaInstance.getCamera().animate(
        {
          x: normalizedX,
          y: normalizedY,
          ratio: 0.3,
        },
        { duration: 500 }
      );
    }
  }
}, [sigmaInstance, selectedNodeId]);
```

→ [Detailed Fix: 03-camera-focus-fix.md](./03-camera-focus-fix.md)

---

## Step 2: Verify Markdown Safety

The markdown-renderer already has null checks but we should verify they're working. The component was updated in a previous session.

**Check for:**

- Null/undefined props handling in `code()` component
- Early return for empty content
- Error boundary wrapping ReactMarkdown

---

## Step 3: E2E Test All Fixes

Using Playwright browser tools:

1. Navigate to `/query`
2. Type a question and submit
3. Verify response renders without errors
4. Click "New" button to clear conversation
5. Navigate to `/graph`
6. Search for and select a node
7. Click "Focus on Selected Node"
8. Verify camera centers on the selected node

---

## Step 4: Commit Changes

```bash
git add edgequake_webui/src/components/graph/zoom-controls.tsx
git commit -m "fix: graph camera focus now centers on selected node

- Convert graph coordinates to normalized camera coordinates
- Use Sigma's getBBox() to properly normalize positions
- Add smooth animation easing"
```
