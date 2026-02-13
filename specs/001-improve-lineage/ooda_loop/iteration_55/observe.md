# Observation - Iteration 55

## Mobile Drawer Analysis

The graph page has mobile-specific drawers that replace the resizable panels on small screens.

### Desktop vs Mobile Layout (graph-viewer.tsx)

```tsx
// Desktop: Two side panels with ResizablePanel
<div className="hidden md:flex">
  <ResizablePanel side="left"... >
    <EntityBrowserPanel />
  </ResizablePanel>
  <GraphCanvas />
  <ResizablePanel side="right"... >
    <NodeDetails />
  </ResizablePanel>
</div>

// Mobile: Full-width canvas with sheet/drawer
<div className="flex md:hidden">
  <GraphCanvas />
  <Sheet>
    <EntityBrowserPanel />  // Uses same ScrollArea
  </Sheet>
  <Sheet>
    <NodeDetails />         // Uses same ScrollArea
  </Sheet>
</div>
```

### Mobile ScrollArea Behavior

The same components (`EntityBrowserPanel`, `NodeDetails`) are used in both desktop panels and mobile sheets. The Radix wrapper `!block` override is applied at the ScrollArea className level inside `graph-viewer.tsx`, so it applies regardless of whether the panel is in a ResizablePanel or a Sheet.

### Observation

Mobile drawers inherit the same fix because the ScrollArea className is set inside the graph content div, not on the ResizablePanel wrapper.
