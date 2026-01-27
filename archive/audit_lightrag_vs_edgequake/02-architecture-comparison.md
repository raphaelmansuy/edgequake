# Architecture Comparison: LightRAG vs EdgeQuake

> **Document:** 02-architecture-comparison.md  
> **Last Updated:** 2025-12-30

---

## 1. Technology Stack

### LightRAG WebUI

| Layer         | Technology               | Notes                   |
| ------------- | ------------------------ | ----------------------- |
| Framework     | React 18 + Vite          | Fast HMR, no SSR        |
| Graph Library | @react-sigma/core        | Wrapper around sigma.js |
| Graph Data    | graphology DirectedGraph | Part of sigma ecosystem |
| State         | Zustand                  | With selectors pattern  |
| Styling       | Tailwind CSS             | Utility-first           |
| Routing       | React Router             | Client-side routing     |
| HTTP Client   | Axios                    | Promise-based           |
| i18n          | react-i18next            | Multi-language          |

### EdgeQuake WebUI

| Layer         | Technology               | Notes                  |
| ------------- | ------------------------ | ---------------------- |
| Framework     | Next.js 15 (App Router)  | SSR/SSG ready          |
| Graph Library | sigma + graphology       | Direct integration     |
| Graph Data    | graphology Graph         | Manual instantiation   |
| State         | Zustand                  | Simpler flat structure |
| Styling       | Tailwind CSS + shadcn/ui | Component library      |
| Routing       | Next.js App Router       | File-based             |
| HTTP Client   | Custom fetch wrapper     | Type-safe              |
| i18n          | react-i18next            | Multi-language         |
| Data Fetching | TanStack Query           | Caching, deduplication |

---

## 2. Component Architecture

### 2.1 LightRAG Graph Components

```
lightrag_webui/src/
├── features/
│   └── GraphViewer.tsx          # Main container with SigmaContainer
├── components/graph/
│   ├── GraphControl.tsx         # Event handling, sigma binding
│   ├── LayoutsControl.tsx       # 6 layout algorithms
│   ├── ZoomControl.tsx          # Zoom in/out/reset
│   ├── FullScreenControl.tsx    # Fullscreen toggle
│   ├── GraphSearch.tsx          # MiniSearch integration
│   ├── GraphLabels.tsx          # Node label rendering
│   ├── PropertiesView.tsx       # Node/edge properties panel
│   ├── EditablePropertyRow.tsx  # Inline editing
│   ├── Legend.tsx               # Color-coded type display
│   ├── LegendButton.tsx         # Legend toggle
│   ├── Settings.tsx             # Graph settings panel
│   ├── SettingsDisplay.tsx      # Display settings
│   ├── FocusOnNode.tsx          # Camera focus utility
│   ├── MergeDialog.tsx          # Entity merge UI
│   └── PropertyEditDialog.tsx   # Property editor modal
├── stores/
│   ├── graph.ts                 # RawGraph class, indexed maps
│   └── settings.ts              # User preferences
└── hooks/
    └── useLightragGraph.tsx     # Graph operations hook
```

### 2.2 EdgeQuake Graph Components

```
edgequake_webui/src/
├── components/graph/
│   ├── graph-viewer.tsx         # Main container with panels
│   ├── graph-renderer.tsx       # Sigma + graphology renderer
│   ├── graph-controls.tsx       # Control bar
│   ├── layout-control.tsx       # 3 layout algorithms
│   ├── zoom-controls.tsx        # Zoom in/out/reset
│   ├── graph-search.tsx         # MiniSearch with popover
│   ├── graph-filters.tsx        # Entity type checkboxes
│   ├── graph-legend.tsx         # Interactive type toggle
│   ├── graph-events.tsx         # Event handling
│   ├── graph-export.tsx         # Export to PNG/JSON
│   ├── graph-tour-wrapper.tsx   # Guided tour
│   ├── entity-browser-panel.tsx # Left sidebar browser
│   ├── node-details.tsx         # Right sidebar details
│   ├── node-context-menu.tsx    # Right-click menu
│   ├── entity-edit-dialog.tsx   # Entity editor modal
│   ├── relationship-edit-dialog.tsx # Relationship editor
│   ├── graph-context-menu.tsx   # Background context menu
│   └── keyboard-shortcuts-help.tsx # Shortcuts modal
├── stores/
│   └── use-graph-store.ts       # Flat state structure
├── hooks/
│   └── use-graph-keyboard-navigation.ts # Keyboard nav
└── lib/graph/
    ├── camera-utils.ts          # Camera focus helpers
    └── clustering.ts            # Community detection
```

---

## 3. Data Structures

### 3.1 LightRAG RawGraph Class

```typescript
// lightrag_webui/src/stores/graph.ts

export class RawGraph {
  nodes: RawNodeType[] = [];
  edges: RawEdgeType[] = [];

  // O(1) lookups via indexed maps
  nodeIdMap: Record<string, number> = {}; // nodeId → array index
  edgeIdMap: Record<string, number> = {}; // edgeId → array index
  edgeDynamicIdMap: Record<string, number> = {}; // sigma key → array index

  getNode(nodeId: string): RawNodeType | undefined {
    const index = this.nodeIdMap[nodeId];
    return index !== undefined ? this.nodes[index] : undefined;
  }

  getEdge(edgeId: string, dynamicId = true): RawEdgeType | undefined {
    const index = dynamicId
      ? this.edgeDynamicIdMap[edgeId]
      : this.edgeIdMap[edgeId];
    return index !== undefined ? this.edges[index] : undefined;
  }

  buildDynamicMap(): void {
    this.edgeDynamicIdMap = {};
    for (let i = 0; i < this.edges.length; i++) {
      this.edgeDynamicIdMap[this.edges[i].dynamicId] = i;
    }
  }
}
```

**Advantages:**

- O(1) node/edge lookups
- Separate index for sigma's dynamic IDs
- Efficient for large graphs (1000+ nodes)

### 3.2 EdgeQuake Flat Arrays

```typescript
// edgequake_webui/src/stores/use-graph-store.ts

interface GraphState {
  graph: KnowledgeGraph | null;
  nodes: GraphNode[]; // Flat array
  edges: GraphEdge[]; // Flat array
  selectedNodeId: string | null;
  visibleEntityTypes: Set<string>;
  visibleRelationshipTypes: Set<string>;
  searchQuery: string;
  colorMode: "entity-type" | "community";
  sigmaInstance: Sigma | null;
}
```

**Issues:**

- O(n) lookups via Array.find()
- No index structures
- Inefficient for large graphs

**Recommendation:** Add indexed maps:

```typescript
interface GraphState {
  nodeMap: Map<string, GraphNode>; // O(1) lookup
  edgeMap: Map<string, GraphEdge>; // O(1) lookup
  // ... existing fields
}
```

---

## 4. API Query Patterns

### 4.1 LightRAG API

```typescript
// Label-centric querying
queryGraphs(label, maxDepth, maxNodes)
→ GET /graphs?label=X&max_depth=Y&max_nodes=Z

// Supporting endpoints
getGraphLabels() → GET /graph/label/list
getPopularLabels(limit) → GET /graph/label/popular?limit=N
searchLabels(query, limit) → GET /graph/label/search?q=X&limit=N
```

**Characteristics:**

- Queries by specific entity label
- Depth-limited traversal (controls result size)
- Popular/search labels for discovery
- Server enforces limits

### 4.2 EdgeQuake API

```typescript
// Type-based querying
getGraph({ limit, entity_types, include_orphans })
→ GET /graph?limit=N&entity_types=X,Y&include_orphans=bool

// Supporting endpoints
getGraphLabels() → GET /graph/labels
getGraphStats() → GET /graph/stats
```

**Characteristics:**

- Queries by entity type categories
- Global node limit
- No depth control
- Client-side filtering for fine-grained control

### 4.3 Comparison Table

| Capability              | LightRAG  | EdgeQuake          |
| ----------------------- | --------- | ------------------ |
| Label-specific query    | ✅        | ❌                 |
| Depth-limited traversal | ✅        | ❌                 |
| Popular labels          | ✅        | ❌                 |
| Label search            | ✅        | ❌                 |
| Type filtering          | ❌ Server | ✅ Server + Client |
| Orphan control          | ❌        | ✅                 |
| Stats endpoint          | ❌        | ✅                 |

---

## 5. Layout Algorithms

### 5.1 LightRAG Layouts (6)

| Layout         | Implementation             | Worker | Animation |
| -------------- | -------------------------- | ------ | --------- |
| Circular       | useLayoutCircular          | ❌     | ❌        |
| Circlepack     | useLayoutCirclepack        | ❌     | ❌        |
| Random         | useLayoutRandom            | ❌     | ❌        |
| Noverlaps      | useLayoutNoverlap          | ✅     | ✅ 300ms  |
| Force Directed | useWorkerLayoutForce       | ✅     | ✅ 300ms  |
| Force Atlas    | useWorkerLayoutForceAtlas2 | ✅     | ✅ 300ms  |

**Key Feature:** Web Worker execution for heavy layouts

```typescript
// Worker-based layout with animation
const { positions } = useWorkerLayoutForceAtlas2({ iterations: 100 });
animateNodes(graph, positions, { duration: 300 });
```

### 5.2 EdgeQuake Layouts (3)

| Layout   | Implementation       | Worker | Animation |
| -------- | -------------------- | ------ | --------- |
| Force    | forceAtlas2.assign() | ❌     | ❌        |
| Circular | circular.assign()    | ❌     | ❌        |
| Random   | random.assign()      | ❌     | ❌        |

**Issue:** Synchronous execution blocks main thread

```typescript
// Current: Synchronous, blocks UI
forceAtlas2.assign(graph, {
  iterations: 100,
  settings: { gravity: 1, barnesHutOptimize: graph.order > 100 },
});
```

**Recommendation:** Use Web Worker supervisor

```typescript
import { FA2LayoutSupervisor } from "graphology-layout-forceatlas2/worker";

const layout = new FA2LayoutSupervisor(graph, {
  settings: { gravity: 1 },
});
layout.start();
// ... when done
layout.stop();
```

---

## 6. Sigma Rendering Programs

### 6.1 LightRAG Configuration

```typescript
// GraphViewer.tsx
const createSigmaSettings = (isDarkTheme: boolean): Partial<SigmaSettings> => ({
  defaultNodeType: "default",
  defaultEdgeType: "curvedNoArrow",

  nodeProgramClasses: {
    default: NodeBorderProgram, // ← Premium bordered nodes
    circel: NodeCircleProgram,
    point: NodePointProgram,
  },

  edgeProgramClasses: {
    arrow: EdgeArrowProgram,
    curvedArrow: EdgeCurvedArrowProgram, // ← Curved arrows
    curvedNoArrow: createEdgeCurveProgram(), // ← Curved lines
  },

  labelGridCellSize: 60, // Label optimization
  labelRenderedSizeThreshold: 12, // Size threshold
  enableEdgeEvents: true,

  labelColor: {
    color: isDarkTheme ? labelColorDarkTheme : labelColorLightTheme,
  },
});
```

### 6.2 EdgeQuake Configuration

```typescript
// graph-renderer.tsx
const sigma = new Sigma(graph, containerRef.current, {
  renderLabels: showLabels,
  renderEdgeLabels: showEdgeLabels,
  labelSize: 12,
  labelColor: { color: "#374151" }, // ← Hardcoded, not theme-aware
  labelFont: "Inter, sans-serif",
  defaultNodeColor: "#64748b",
  defaultEdgeColor: "#94a3b8",
  minCameraRatio: 0.1,
  maxCameraRatio: 10,
});
```

**Missing in EdgeQuake:**

- NodeBorderProgram (bordered nodes)
- EdgeCurvedArrowProgram (curved arrows)
- createEdgeCurveProgram (curved lines)
- Theme-aware label colors
- Label grid optimization

---

## 7. State Management Comparison

### 7.1 LightRAG Store Features

```typescript
interface GraphState {
  // Graph data with versioning
  rawGraph: RawGraph | null;
  sigmaGraph: DirectedGraph | null;
  graphDataVersion: number;
  incrementGraphDataVersion: () => void;

  // Search engine integration
  searchEngine: MiniSearch | null;
  setSearchEngine: (engine: MiniSearch | null) => void;
  resetSearchEngine: () => void;

  // Node operations
  triggerNodeExpand: (nodeId: string | null) => void;
  triggerNodePrune: (nodeId: string | null) => void;
  nodeToExpand: string | null;
  nodeToPrune: string | null;

  // Graph updates with UI sync
  updateNodeAndSelect: (nodeId, entityId, prop, value) => Promise<void>;
  updateEdgeAndSelect: (
    edgeId,
    dynamicId,
    source,
    target,
    prop,
    value
  ) => Promise<void>;

  // Type color mapping
  typeColorMap: Map<string, string>;
}
```

### 7.2 EdgeQuake Store Features

```typescript
interface GraphState {
  // Graph data
  graph: KnowledgeGraph | null;
  nodes: GraphNode[];
  edges: GraphEdge[];

  // Selection
  selectedNodeId: string | null;
  focusedNodeId: string | null;
  hoveredNodeId: string | null;
  selectedNodes: Set<string>;

  // Filtering (client-side)
  visibleEntityTypes: Set<string>;
  visibleRelationshipTypes: Set<string>;
  searchQuery: string;

  // Display modes
  colorMode: "entity-type" | "community";
  showClustering: boolean;

  // UI state
  showNodeDetails: boolean;
  rightPanelCollapsed: boolean;

  // Instance reference
  sigmaInstance: Sigma | null;
}
```

**Key Differences:**

| Feature                | LightRAG            | EdgeQuake              |
| ---------------------- | ------------------- | ---------------------- |
| Indexed data           | ✅ RawGraph class   | ❌ Flat arrays         |
| Graph versioning       | ✅ graphDataVersion | ❌ None                |
| Search engine in store | ✅ Yes              | ❌ Per-component       |
| Node expand/prune      | ✅ Yes              | ❌ None                |
| Type color map         | ✅ Centralized      | ❌ Hardcoded constants |
| Community detection    | ❌ None             | ✅ colorMode           |
| Multi-select           | ❌ None             | ✅ selectedNodes Set   |

---

## 8. Recommendations

### 8.1 Data Structure Improvements

```typescript
// Add to use-graph-store.ts
interface GraphState {
  // Add indexed maps for O(1) lookups
  nodeMap: Map<string, GraphNode>;
  edgeMap: Map<string, GraphEdge>;

  // Add graph versioning
  graphVersion: number;
  incrementGraphVersion: () => void;
}
```

### 8.2 Layout Architecture

```typescript
// Add Web Worker support
import { FA2LayoutSupervisor } from "graphology-layout-forceatlas2/worker";

const layoutSupervisor = useRef<FA2LayoutSupervisor | null>(null);

const startLayout = () => {
  layoutSupervisor.current = new FA2LayoutSupervisor(graph, {
    settings: { gravity: 1, barnesHutOptimize: true },
  });
  layoutSupervisor.current.start();
};

const stopLayout = () => {
  layoutSupervisor.current?.stop();
};
```

### 8.3 Sigma Programs

```typescript
// Add to graph-renderer.tsx
import { NodeBorderProgram } from "@sigma/node-border";
import {
  EdgeCurvedArrowProgram,
  createEdgeCurveProgram,
} from "@sigma/edge-curve";

const sigma = new Sigma(graph, container, {
  nodeProgramClasses: {
    default: NodeBorderProgram,
  },
  edgeProgramClasses: {
    arrow: EdgeArrowProgram,
    curved: EdgeCurvedArrowProgram,
    curvedNoArrow: createEdgeCurveProgram(),
  },
  defaultEdgeType: "curved",
});
```

---

_Related Documents:_

- [01-executive-summary.md](./01-executive-summary.md)
- [04-feature-parity-analysis.md](./04-feature-parity-analysis.md)
- [05-performance-report.md](./05-performance-report.md)
