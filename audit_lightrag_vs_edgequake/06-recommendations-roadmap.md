# Recommendations Roadmap: EdgeQuake Graph Improvements

> **Document:** 06-recommendations-roadmap.md  
> **Last Updated:** 2025-12-30

---

## 1. Improvement Phases Overview

| Phase       | Focus                 | Duration | Priority |
| ----------- | --------------------- | -------- | -------- |
| **Phase 1** | Critical Fixes        | Week 1   | 🔴 P0    |
| **Phase 2** | Visual Quality        | Week 2   | 🟠 P1    |
| **Phase 3** | Feature Parity        | Week 3-4 | 🟡 P2    |
| **Phase 4** | Performance Hardening | Week 5   | 🟢 P3    |
| **Phase 5** | SOTA Features         | Week 6+  | 🔵 P4    |

---

## 2. Phase 1: Critical Fixes (Week 1)

### 2.1 Fix Responsive Layout Bug

**Problem:** Graph canvas invisible on tablet (768px) and mobile (375px)

**Location:** [graph-viewer.tsx](../edgequake_webui/src/components/graph/graph-viewer.tsx)

**Root Cause Analysis:**

- Entity browser panel and details panel consume 100% width
- Graph canvas receives 0px width
- CSS flexbox/grid layout issues at responsive breakpoints

**Acceptance Criteria:**

- [ ] Graph visible at 1440px, 1024px, 768px, 375px
- [ ] Panels collapse properly on smaller screens
- [ ] Touch interactions work on mobile
- [ ] Graph fills available space after panel collapse

**Implementation:**

```typescript
// Add responsive panel behavior
const isMobile = useMediaQuery('(max-width: 768px)');
const isTablet = useMediaQuery('(max-width: 1024px)');

// Auto-collapse panels on smaller screens
useEffect(() => {
  if (isMobile || isTablet) {
    setLeftPanelCollapsed(true);
    setRightPanelCollapsed(true);
  }
}, [isMobile, isTablet]);

// Ensure graph container has minimum width
<div className={cn(
  "flex-1 relative min-w-0",  // min-w-0 prevents overflow
  "h-full",
  // Ensure visible even when panels are open
  leftPanelCollapsed && rightPanelCollapsed && "w-full"
)}>
  <GraphRenderer ... />
</div>
```

**Effort:** 4-6 hours

---

### 2.2 Add Web Worker for ForceAtlas2

**Problem:** Layout algorithm blocks main thread, causing 2-5s UI freeze

**Location:** [graph-renderer.tsx](../edgequake_webui/src/components/graph/graph-renderer.tsx)

**Acceptance Criteria:**

- [ ] Layout runs in Web Worker (non-blocking)
- [ ] UI remains responsive during layout
- [ ] Layout auto-stops after 3 seconds
- [ ] Play/pause toggle available

**Implementation:**

```typescript
// graph-renderer.tsx
import { FA2LayoutSupervisor } from "graphology-layout-forceatlas2/worker";

interface GraphRendererProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  onLayoutStart?: () => void;
  onLayoutEnd?: () => void;
}

export function GraphRenderer({
  nodes,
  edges,
  onLayoutStart,
  onLayoutEnd,
}: GraphRendererProps) {
  const layoutSupervisorRef = useRef<FA2LayoutSupervisor | null>(null);
  const [isLayouting, setIsLayouting] = useState(false);

  const startLayout = useCallback(() => {
    const graph = sigmaRef.current?.getGraph();
    if (!graph || graph.order === 0) return;

    // Stop existing layout
    layoutSupervisorRef.current?.stop();

    setIsLayouting(true);
    onLayoutStart?.();

    // Create Web Worker supervisor
    layoutSupervisorRef.current = new FA2LayoutSupervisor(graph, {
      settings: {
        gravity: 1,
        scalingRatio: 2,
        strongGravityMode: true,
        barnesHutOptimize: graph.order > 100,
        slowDown: 1 + Math.log(graph.order) / 4,
      },
    });

    layoutSupervisorRef.current.start();

    // Auto-stop after 3 seconds
    const timeout = setTimeout(() => {
      stopLayout();
    }, 3000);

    return () => clearTimeout(timeout);
  }, [onLayoutStart]);

  const stopLayout = useCallback(() => {
    layoutSupervisorRef.current?.stop();
    layoutSupervisorRef.current = null;
    setIsLayouting(false);
    onLayoutEnd?.();
  }, [onLayoutEnd]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      layoutSupervisorRef.current?.stop();
    };
  }, []);

  return (
    <div className="relative h-full w-full">
      {/* Graph canvas */}
      <div ref={containerRef} className="absolute inset-0" />

      {/* Layout indicator */}
      {isLayouting && (
        <div className="absolute top-2 left-2 flex items-center gap-2 bg-background/80 px-2 py-1 rounded-md text-xs">
          <Loader2 className="h-3 w-3 animate-spin" />
          <span>Optimizing layout...</span>
          <Button size="sm" variant="ghost" onClick={stopLayout}>
            Stop
          </Button>
        </div>
      )}
    </div>
  );
}
```

**Package Installation:**

```bash
cd edgequake_webui
pnpm add graphology-layout-forceatlas2
```

**Effort:** 6-8 hours

---

### 2.3 Implement Progressive Loading

**Problem:** Full graph fetched upfront, memory pressure on large datasets

**Acceptance Criteria:**

- [ ] Initial load limited to 100 popular entities
- [ ] "Load more" button for additional entities
- [ ] Neighborhood expansion on node selection
- [ ] Loading indicators for async operations

**Backend API Addition:**

```rust
// edgequake/crates/edgequake-api/src/routes/graph.rs

/// GET /graph/neighbors?node_id=X&depth=Y&limit=Z
async fn get_graph_neighbors(
    Query(params): Query<NeighborsParams>,
    State(state): State<AppState>,
) -> Result<Json<GraphNeighbors>, ApiError> {
    let neighbors = state.graph_service
        .get_neighbors(&params.node_id, params.depth.unwrap_or(1), params.limit.unwrap_or(50))
        .await?;

    Ok(Json(neighbors))
}
```

**Frontend Implementation:**

```typescript
// lib/api/edgequake.ts
export async function getGraphNeighbors(options: {
  nodeId: string;
  depth?: number;
  limit?: number;
}): Promise<{ nodes: GraphNode[]; edges: GraphEdge[] }> {
  const params = new URLSearchParams();
  params.set("node_id", options.nodeId);
  if (options.depth) params.set("depth", String(options.depth));
  if (options.limit) params.set("limit", String(options.limit));

  return api.get(`/graph/neighbors?${params}`);
}

// In graph-viewer.tsx
const expandNode = async (nodeId: string) => {
  const neighbors = await getGraphNeighbors({ nodeId, depth: 1, limit: 50 });

  // Merge new nodes (avoiding duplicates)
  const existingIds = new Set(nodes.map((n) => n.id));
  const newNodes = neighbors.nodes.filter((n) => !existingIds.has(n.id));

  setGraph({
    ...graph,
    nodes: [...nodes, ...newNodes],
    edges: [...edges, ...neighbors.edges],
  });

  toast.success(`Added ${newNodes.length} connected entities`);
};
```

**Effort:** 8-12 hours (includes backend work)

---

## 3. Phase 2: Visual Quality (Week 2)

### 3.1 Add Curved Edge Rendering

**Location:** [graph-renderer.tsx](../edgequake_webui/src/components/graph/graph-renderer.tsx)

**Package Installation:**

```bash
pnpm add @sigma/edge-curve
```

**Implementation:**

```typescript
import {
  EdgeCurvedArrowProgram,
  createEdgeCurveProgram,
} from "@sigma/edge-curve";

const sigma = new Sigma(graph, containerRef.current, {
  // Existing settings...

  edgeProgramClasses: {
    arrow: EdgeArrowProgram,
    curved: EdgeCurvedArrowProgram,
    curvedNoArrow: createEdgeCurveProgram(),
  },
  defaultEdgeType: "curved", // Change from 'arrow'
});
```

**Acceptance Criteria:**

- [ ] Edges render as smooth curves
- [ ] Arrow heads point correctly
- [ ] Self-loops render as circular arcs
- [ ] Performance maintained for 500+ edges

**Effort:** 2-3 hours

---

### 3.2 Add Node Border Program

**Package Installation:**

```bash
pnpm add @sigma/node-border
```

**Implementation:**

```typescript
import { NodeBorderProgram } from "@sigma/node-border";

const sigma = new Sigma(graph, containerRef.current, {
  // Existing settings...

  nodeProgramClasses: {
    default: NodeBorderProgram,
  },
  nodeReducer: (node, data) => ({
    ...data,
    borderColor: "#ffffff",
    borderSize: 2,
  }),
});
```

**Acceptance Criteria:**

- [ ] Nodes have visible white borders
- [ ] Borders scale with node size
- [ ] Selected node has highlighted border
- [ ] Works in both light and dark themes

**Effort:** 2-3 hours

---

### 3.3 Add Layout Animations

**Implementation:**

```typescript
import { animateNodes } from "sigma/utils";

const applyLayout = (positions: Record<string, { x: number; y: number }>) => {
  const graph = sigmaRef.current?.getGraph();
  if (!graph) return;

  // Smooth 300ms transition to new positions
  animateNodes(graph, positions, {
    duration: 300,
    easing: "quadraticInOut",
  });
};

// Use with layout changes
const switchToCircular = () => {
  const graph = sigmaRef.current?.getGraph();
  if (!graph) return;

  // Calculate new positions
  const positions: Record<string, { x: number; y: number }> = {};
  const nodes = graph.nodes();
  const angleStep = (2 * Math.PI) / nodes.length;
  const radius = 100;

  nodes.forEach((nodeId, index) => {
    positions[nodeId] = {
      x: Math.cos(index * angleStep) * radius,
      y: Math.sin(index * angleStep) * radius,
    };
  });

  applyLayout(positions);
};
```

**Acceptance Criteria:**

- [ ] Layout changes animate smoothly (300ms)
- [ ] Camera follows node movements
- [ ] No jarring jumps
- [ ] Animation can be interrupted

**Effort:** 3-4 hours

---

### 3.4 Theme-Aware Label Colors

**Implementation:**

```typescript
// In graph-renderer.tsx
import { useTheme } from "next-themes";

const { theme } = useTheme();
const isDark = theme === "dark";

const sigma = new Sigma(graph, containerRef.current, {
  labelColor: {
    color: isDark ? "#e2e8f0" : "#1e293b", // slate-200 / slate-800
  },
  edgeLabelColor: {
    color: isDark ? "#94a3b8" : "#475569", // slate-400 / slate-600
  },
});

// Update on theme change
useEffect(() => {
  if (sigmaRef.current) {
    sigmaRef.current.setSetting("labelColor", {
      color: isDark ? "#e2e8f0" : "#1e293b",
    });
    sigmaRef.current.refresh();
  }
}, [isDark]);
```

**Acceptance Criteria:**

- [ ] Labels readable in light theme
- [ ] Labels readable in dark theme
- [ ] Smooth transition on theme change
- [ ] Edge labels also theme-aware

**Effort:** 1-2 hours

---

## 4. Phase 3: Feature Parity (Week 3-4)

### 4.1 Node Expand/Prune Functionality

**Expand Node:**

```typescript
// hooks/use-graph-operations.ts
export function useGraphOperations() {
  const { nodes, edges, setGraph, sigmaInstance } = useGraphStore();

  const expandNode = async (nodeId: string) => {
    try {
      // Fetch neighbors from server
      const neighbors = await getGraphNeighbors({
        nodeId,
        depth: 1,
        limit: 50,
      });

      // Calculate radial positions for new nodes
      const existingNode = nodes.find((n) => n.id === nodeId);
      if (!existingNode || !sigmaInstance) return;

      const sourcePos = sigmaInstance.getNodeDisplayData(nodeId);
      if (!sourcePos) return;

      const newNodes = neighbors.nodes.filter(
        (n) => !nodes.some((existing) => existing.id === n.id)
      );

      // Position new nodes radially
      const angleStep = (2 * Math.PI) / newNodes.length;
      const radius = 150;

      newNodes.forEach((node, index) => {
        const angle = index * angleStep;
        node.x = sourcePos.x + Math.cos(angle) * radius;
        node.y = sourcePos.y + Math.sin(angle) * radius;
      });

      // Merge into graph
      setGraph({
        nodes: [...nodes, ...newNodes],
        edges: [...edges, ...neighbors.edges],
      });

      toast.success(`Expanded: +${newNodes.length} entities`);
    } catch (error) {
      toast.error("Failed to expand node");
    }
  };

  const pruneNode = (nodeId: string) => {
    const remainingNodes = nodes.filter((n) => n.id !== nodeId);
    const remainingEdges = edges.filter(
      (e) => e.source !== nodeId && e.target !== nodeId
    );

    setGraph({
      nodes: remainingNodes,
      edges: remainingEdges,
    });

    toast.success("Node removed from view");
  };

  return { expandNode, pruneNode };
}
```

**UI Integration:**

```typescript
// In node-details.tsx
import { GitBranchPlus, Scissors } from "lucide-react";

const { expandNode, pruneNode } = useGraphOperations();

<div className="flex gap-2">
  <Button
    variant="outline"
    size="sm"
    onClick={() => expandNode(node.id)}
    title="Expand neighborhood"
  >
    <GitBranchPlus className="h-4 w-4 mr-1" />
    Expand
  </Button>
  <Button
    variant="outline"
    size="sm"
    onClick={() => pruneNode(node.id)}
    title="Remove from view"
  >
    <Scissors className="h-4 w-4 mr-1" />
    Prune
  </Button>
</div>;
```

**Acceptance Criteria:**

- [ ] Expand button in node details panel
- [ ] New nodes positioned radially around source
- [ ] Duplicate nodes prevented
- [ ] Prune removes node and connected edges
- [ ] Undo available for prune action

**Effort:** 8-12 hours

---

### 4.2 Add Additional Layouts

**Noverlaps Layout:**

```typescript
import noverlap from "graphology-layout-noverlap";

const applyNoverlap = () => {
  const graph = sigmaRef.current?.getGraph();
  if (!graph) return;

  const positions = noverlap(graph, {
    maxIterations: 100,
    gridSize: 20,
    margin: 5,
    expansion: 1.1,
    ratio: 1.0,
  });

  animateNodes(graph, positions, { duration: 300 });
};
```

**Circlepack Layout:**

```typescript
import circlepack from "graphology-layout/circlepack";

const applyCirclepack = () => {
  const graph = sigmaRef.current?.getGraph();
  if (!graph) return;

  circlepack.assign(graph, {
    hierarchyAttributes: ["node_type"],
    scale: 100,
  });

  sigmaRef.current?.refresh();
};
```

**UI Updates:**

```typescript
// layout-control.tsx
const layouts = [
  { id: "force", label: "Force Directed", icon: Network },
  { id: "circular", label: "Circular", icon: Circle },
  { id: "random", label: "Random", icon: Shuffle },
  { id: "noverlap", label: "No Overlap", icon: Layers }, // New
  { id: "circlepack", label: "Circlepack", icon: Boxes }, // New
];
```

**Effort:** 4-6 hours

---

### 4.3 Depth-Limited API Query

**Backend Implementation:**

```rust
// Add to graph routes
#[derive(Deserialize)]
struct DepthQueryParams {
    label: String,
    max_depth: Option<u32>,
    max_nodes: Option<u32>,
}

async fn query_by_label(
    Query(params): Query<DepthQueryParams>,
    State(state): State<AppState>,
) -> Result<Json<GraphResponse>, ApiError> {
    let subgraph = state.graph_service
        .query_subgraph(
            &params.label,
            params.max_depth.unwrap_or(2),
            params.max_nodes.unwrap_or(100),
        )
        .await?;

    Ok(Json(subgraph))
}
```

**Frontend Integration:**

```typescript
// lib/api/edgequake.ts
export async function queryGraphByLabel(options: {
  label: string;
  maxDepth?: number;
  maxNodes?: number;
}): Promise<KnowledgeGraph> {
  const params = new URLSearchParams();
  params.set("label", options.label);
  if (options.maxDepth) params.set("max_depth", String(options.maxDepth));
  if (options.maxNodes) params.set("max_nodes", String(options.maxNodes));

  return api.get(`/graph/query?${params}`);
}
```

**Effort:** 8-12 hours (includes backend)

---

## 5. Phase 4: Performance Hardening (Week 5)

### 5.1 Indexed Data Structures

```typescript
// use-graph-store.ts
interface GraphState {
  // Existing
  nodes: GraphNode[];
  edges: GraphEdge[];

  // Add indexed maps
  nodeMap: Map<string, GraphNode>;
  edgeMap: Map<string, GraphEdge>;
  nodesByType: Map<string, Set<string>>; // type → node IDs
  edgesBySource: Map<string, Set<string>>; // nodeId → edge IDs
  edgesByTarget: Map<string, Set<string>>; // nodeId → edge IDs
}

// Optimized setGraph action
setGraph: (graph) => {
  const nodeMap = new Map<string, GraphNode>();
  const edgeMap = new Map<string, GraphEdge>();
  const nodesByType = new Map<string, Set<string>>();
  const edgesBySource = new Map<string, Set<string>>();
  const edgesByTarget = new Map<string, Set<string>>();

  // Build node indexes
  graph.nodes.forEach((node) => {
    nodeMap.set(node.id, node);

    const typeSet = nodesByType.get(node.node_type) || new Set();
    typeSet.add(node.id);
    nodesByType.set(node.node_type, typeSet);
  });

  // Build edge indexes
  graph.edges.forEach((edge) => {
    const edgeId = `${edge.source}-${edge.target}-${edge.relationship_type}`;
    edgeMap.set(edgeId, edge);

    const sourceSet = edgesBySource.get(edge.source) || new Set();
    sourceSet.add(edgeId);
    edgesBySource.set(edge.source, sourceSet);

    const targetSet = edgesByTarget.get(edge.target) || new Set();
    targetSet.add(edgeId);
    edgesByTarget.set(edge.target, targetSet);
  });

  set({
    nodes: graph.nodes,
    edges: graph.edges,
    nodeMap,
    edgeMap,
    nodesByType,
    edgesBySource,
    edgesByTarget,
  });
};
```

**Effort:** 4-6 hours

---

### 5.2 Virtual Scrolling for Entity Browser

```typescript
// entity-browser-panel.tsx
import { useVirtualizer } from "@tanstack/react-virtual";

const EntityList = ({ nodes, selectedNodeId, onNodeClick }) => {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: nodes.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 44,
    overscan: 5,
  });

  return (
    <div ref={parentRef} className="h-full overflow-auto">
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          position: "relative",
        }}
      >
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const node = nodes[virtualRow.index];
          return (
            <div
              key={node.id}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
              }}
            >
              <EntityItem
                node={node}
                isSelected={node.id === selectedNodeId}
                onClick={() => onNodeClick(node.id)}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
};
```

**Package Installation:**

```bash
pnpm add @tanstack/react-virtual
```

**Effort:** 4-6 hours

---

### 5.3 Incremental Graph Updates

```typescript
// Instead of recreating graph on filter changes
const updateNodeVisibility = useCallback(
  (visibleTypes: Set<string>) => {
    const graph = sigmaRef.current?.getGraph();
    if (!graph) return;

    graph.forEachNode((nodeId) => {
      const node = nodeMap.get(nodeId);
      if (!node) return;

      const isVisible = visibleTypes.has(node.node_type);
      graph.setNodeAttribute(nodeId, "hidden", !isVisible);
    });

    graph.forEachEdge((edgeId, attrs, source, target) => {
      const sourceHidden = graph.getNodeAttribute(source, "hidden");
      const targetHidden = graph.getNodeAttribute(target, "hidden");
      graph.setEdgeAttribute(edgeId, "hidden", sourceHidden || targetHidden);
    });
  },
  [nodeMap]
);
```

**Effort:** 3-4 hours

---

## 6. Phase 5: SOTA Features (Week 6+)

### 6.1 Graph Minimap

```typescript
import { MiniMap } from "@react-sigma/minimap";

// Add to graph-viewer.tsx
<div className="absolute bottom-4 left-4 w-40 h-32 border rounded-lg overflow-hidden bg-background/80">
  <MiniMap sigma={sigmaInstance} />
</div>;
```

### 6.2 Time-Based Filtering

```typescript
// Add created_at/updated_at filters
interface TimeFilter {
  startDate?: Date;
  endDate?: Date;
}

const filteredByTime = useMemo(() => {
  return nodes.filter((node) => {
    if (!timeFilter.startDate && !timeFilter.endDate) return true;
    const nodeDate = new Date(node.created_at);
    if (timeFilter.startDate && nodeDate < timeFilter.startDate) return false;
    if (timeFilter.endDate && nodeDate > timeFilter.endDate) return false;
    return true;
  });
}, [nodes, timeFilter]);
```

### 6.3 Subgraph Bookmarks

```typescript
// Save current view state
const saveBookmark = () => {
  const bookmark = {
    id: crypto.randomUUID(),
    name: bookmarkName,
    visibleNodeIds: Array.from(filteredNodes.map((n) => n.id)),
    cameraState: sigmaInstance?.getCamera().getState(),
    filters: {
      entityTypes: Array.from(visibleEntityTypes),
      searchQuery,
    },
    createdAt: new Date().toISOString(),
  };

  localStorage.setItem(
    `graph-bookmark-${bookmark.id}`,
    JSON.stringify(bookmark)
  );
};
```

---

## 7. Effort Summary

| Phase                | Items  | Total Effort     |
| -------------------- | ------ | ---------------- |
| Phase 1: Critical    | 3      | 18-26 hours      |
| Phase 2: Visual      | 4      | 8-12 hours       |
| Phase 3: Features    | 3      | 20-30 hours      |
| Phase 4: Performance | 3      | 11-16 hours      |
| Phase 5: SOTA        | 3      | 15-20 hours      |
| **Total**            | **16** | **72-104 hours** |

---

## 8. Dependencies & Packages

```bash
# Phase 1
pnpm add graphology-layout-forceatlas2

# Phase 2
pnpm add @sigma/edge-curve @sigma/node-border

# Phase 3
pnpm add graphology-layout-noverlap

# Phase 4
pnpm add @tanstack/react-virtual

# Phase 5
pnpm add @react-sigma/minimap
```

---

## 9. Testing Checklist

### Phase 1 Tests

- [ ] Graph visible at all breakpoints (375, 768, 1024, 1440px)
- [ ] Layout completes without UI freeze for 500 nodes
- [ ] Progressive loading shows initial 100 entities
- [ ] Expand loads additional neighbors

### Phase 2 Tests

- [ ] Edges render as curves
- [ ] Nodes have visible borders
- [ ] Layout transitions animate smoothly
- [ ] Labels update with theme change

### Phase 3 Tests

- [ ] Expand adds radially positioned nodes
- [ ] Prune removes node and edges
- [ ] Noverlap resolves node overlaps
- [ ] Depth-limited query limits results

### Phase 4 Tests

- [ ] Node lookup is O(1) (benchmark)
- [ ] Entity browser scrolls at 60fps with 1000 nodes
- [ ] Filter changes don't recreate sigma instance

---

_Related Documents:_

- [01-executive-summary.md](./01-executive-summary.md)
- [04-feature-parity-analysis.md](./04-feature-parity-analysis.md)
- [05-performance-report.md](./05-performance-report.md)
