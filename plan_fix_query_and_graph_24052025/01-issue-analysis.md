# Issue Analysis

← [Back to Index](./00-index.md)

## Issue 1: Runtime TypeError in MarkdownRenderer

### Symptoms

```
Runtime TypeError
Cannot use 'in' operator to search for 'children' in undefined
at src/components/query/markdown-renderer.tsx (313:9) @ MarkdownRenderer
```

### Root Cause Analysis

The error occurs in the `ReactMarkdown` component when processing streaming content. The component's custom `code` handler receives `undefined` or `null` props when:

1. The streaming sends empty chunks
2. The markdown parser encounters malformed content
3. The content includes incomplete code blocks during streaming

**Code Location:** [markdown-renderer.tsx](../edgequake_webui/src/components/query/markdown-renderer.tsx#L313)

```tsx
<ReactMarkdown
  remarkPlugins={remarkPlugins}
  rehypePlugins={rehypePlugins}
  components={components} // ← components may receive undefined props
>
  {safeContent}
</ReactMarkdown>
```

### Solution

The existing null checks in component handlers need to be more defensive. Add early returns and wrap the entire ReactMarkdown in additional error handling.

---

## Issue 2: Input Container Not Visible

### Symptoms

The query input textarea at the bottom of the Query page is not visible or scrolls out of view.

### Root Cause Analysis

**Investigated and Found:** During E2E testing, the input WAS visible. The issue may have been:

1. A transient state during hot-reload
2. CSS flexbox layout issue that was already fixed
3. The error from Issue #1 causing component render failure

**Current Layout Structure:**

```tsx
<div className="flex-1 flex flex-col h-full overflow-hidden">
  {/* Header */}
  <div className="flex-shrink-0">...</div>

  {/* ScrollArea - takes remaining space */}
  <ScrollArea className="flex-1">...</ScrollArea>

  {/* Input - fixed at bottom */}
  <div className="border-t p-4 bg-background flex-shrink-0">...</div>
</div>
```

**Code Location:** [query-interface.tsx](../edgequake_webui/src/components/query/query-interface.tsx#L991)

### Status: ✅ Already Fixed

The flexbox layout is correct. The input visibility issue was caused by the TypeError crashing the component.

---

## Issue 3: No New Conversation Button

### Symptoms

Users reported no way to clear the conversation and start fresh.

### Investigation

**Investigated and Found:** The "New" button EXISTS and WORKS.

**Location:** [query-interface.tsx](../edgequake_webui/src/components/query/query-interface.tsx#L755-L768)

```tsx
<Button
  variant="outline"
  size="sm"
  onClick={() => {
    setMessages([]);
    setInput("");
    setCurrentStreamingId(null);
    setStreamingState("idle");
  }}
  disabled={isLoading || messages.length === 0}
  className="gap-1"
>
  <Plus className="h-4 w-4" />
  {t("query.newConversation", "New")}
</Button>
```

### Status: ✅ Already Implemented

The button is disabled when there are no messages (correct UX behavior). Once a conversation starts, the button becomes active.

---

## Issue 4: Graph Camera Focus Broken

### Symptoms

When selecting a node in the Knowledge Graph and clicking "Focus on Selected Node", the camera zooms to empty space instead of centering on the selected node.

### Root Cause Analysis

**Location:** [zoom-controls.tsx](../edgequake_webui/src/components/graph/zoom-controls.tsx#L102-L121)

```tsx
const handleFocusOnNode = useCallback(() => {
  if (sigmaInstance && selectedNodeId) {
    const graph = sigmaInstance.getGraph();

    if (graph.hasNode(selectedNodeId)) {
      const nodePosition = {
        x: graph.getNodeAttribute(selectedNodeId, "x"), // ❌ Graph coordinates
        y: graph.getNodeAttribute(selectedNodeId, "y"), // ❌ Not camera coordinates
      };

      sigmaInstance.getCamera().animate(
        {
          x: nodePosition.x, // ❌ WRONG: Passing graph coords to camera
          y: nodePosition.y, // ❌ Camera expects normalized viewport coords
          ratio: 0.3,
        },
        { duration: 500 }
      );
    }
  }
}, [sigmaInstance, selectedNodeId]);
```

**The Bug:**

- `graph.getNodeAttribute(nodeId, 'x')` returns **graph coordinates** (can range from -100 to +100 or any value)
- `camera.animate({ x, y })` expects **normalized camera coordinates** (0.5, 0.5 is center)
- The camera is zooming to coordinates like `x: -50, y: 75` which are far outside the viewport

### Solution

Use Sigma.js `framedGraphToViewport()` to convert graph coordinates to viewport coordinates, then use that for camera animation:

```tsx
const nodePosition = {
  x: graph.getNodeAttribute(selectedNodeId, "x"),
  y: graph.getNodeAttribute(selectedNodeId, "y"),
};

// Convert graph coordinates to camera coordinates
const viewportCoords = sigmaInstance.graphToViewport(nodePosition);
const dimensions = sigmaInstance.getDimensions();

// Normalize to 0-1 range (camera coords)
const cameraX = viewportCoords.x / dimensions.width;
const cameraY = viewportCoords.y / dimensions.height;
```

**OR** use the simpler approach of animating to normalized graph center:

```tsx
// Get graph bounds
const graphExtent = sigmaInstance.getBBox();

// Calculate normalized position
const normalizedX =
  (nodePosition.x - graphExtent.x[0]) / (graphExtent.x[1] - graphExtent.x[0]);
const normalizedY =
  (nodePosition.y - graphExtent.y[0]) / (graphExtent.y[1] - graphExtent.y[0]);

sigmaInstance.getCamera().animate(
  {
    x: normalizedX,
    y: normalizedY,
    ratio: 0.3,
  },
  { duration: 500 }
);
```

→ [Detailed Fix: 03-camera-focus-fix.md](./03-camera-focus-fix.md)
