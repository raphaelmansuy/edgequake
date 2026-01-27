# EdgeQuake WebUI State Management

> Architecture of application state, store boundaries, and performance patterns.

**Version**: 1.1.0 | **Last Updated**: 2026-01-09

---

## 1. State Strategy

EdgeQuake WebUI manages complexity by strictly categorizing state into three buckets. We use **Zustand** for global client state due to its minimal boilerplate and transient update capabilities (essential for high-performance graph/canvas interactions).

| Type            | Library        | Purpose                                    | Persistence                |
| --------------- | -------------- | ------------------------------------------ | -------------------------- |
| **Server Data** | TanStack Query | API responses (Docs, User, Graph Data)     | Memory (with staleTime)    |
| **Global UI**   | Zustand        | Sidebar state, Auth tokens, Theme, Filters | `localStorage` (Selective) |
| **High-Freq**   | Zustand        | Graph camera position, Hover interaction   | Memory (Transient)         |

---

## 2. Store Catalog

All stores are located in `src/stores/`. Each store is a Zustand hook with clear domain boundaries.

### 2.1 Complete Store List

| Store Hook              | Lines | Features                                   | Persisted?   |
| ----------------------- | ----- | ------------------------------------------ | ------------ |
| `useAuthStore`          | 80    | Authentication tokens, user session        | ✅ (Tokens)  |
| `useBackendStore`       | 119   | Health checks, pipeline status             | ❌           |
| `useConversationStore`  | 284   | Conversation history, messages             | ✅           |
| `useCostStore`          | 100   | Token usage tracking, cost estimation      | ❌           |
| `useGraphStore`         | 950   | Graph visualization, filters, selection    | ❌           |
| `useIngestionStore`     | 150   | File upload queue, progress tracking       | ❌           |
| `useQueryStore`         | 202   | Query execution, streaming response        | ❌           |
| `useQueryUIStore`       | 314   | Streaming state, thinking indicator        | ✅ (Filters) |
| `useSettingsStore`      | 263   | App settings, graph layout, query defaults | ✅           |
| `useTenantStore`        | 120   | Workspace/tenant context                   | ✅           |
| `useUIPreferencesStore` | 80    | Sidebar, theme, density mode               | ✅           |

### 2.2 Store-Feature Mapping

> Cross-reference with [features.md](features.md) FEAT06XX

```
┌──────────────────────────┬───────────────────────────────────────────────┐
│        Zustand Store     │         Features Implemented                  │
├──────────────────────────┼───────────────────────────────────────────────┤
│ useAuthStore             │ FEAT0701 (API Key Auth)                       │
│                          │ FEAT0702 (JWT Token Support)                  │
├──────────────────────────┼───────────────────────────────────────────────┤
│ useBackendStore          │ FEAT0611 (Backend health monitoring)          │
│                          │ BR0608 (Health updates periodically)          │
├──────────────────────────┼───────────────────────────────────────────────┤
│ useConversationStore     │ FEAT0609 (Conversation Persistence)           │
│                          │ UC0401 (Create conversation)                  │
│                          │ UC0405 (View history)                         │
├──────────────────────────┼───────────────────────────────────────────────┤
│ useCostStore             │ FEAT0610 (Cost Tracking Display)              │
│                          │ FEAT0013 (LLM Cost Tracking)                  │
├──────────────────────────┼───────────────────────────────────────────────┤
│ useGraphStore            │ FEAT0601 (Knowledge Graph Visualization)      │
│                          │ FEAT0607 (Entity Type Filter)                 │
│                          │ FEAT0608 (Graph Bookmark Manager)             │
│                          │ FEAT0616 (Entity Search - MiniSearch)         │
│                          │ UC0101 (Explore Entity Neighborhood)          │
├──────────────────────────┼───────────────────────────────────────────────┤
│ useIngestionStore        │ FEAT0605 (Document Upload Interface)          │
│                          │ UC0001 (Upload Text Document)                 │
├──────────────────────────┼───────────────────────────────────────────────┤
│ useQueryStore            │ FEAT0602 (Chat Query Interface)               │
│                          │ FEAT0603 (Streaming Response Display)         │
│                          │ UC0201 (Execute Query)                        │
├──────────────────────────┼───────────────────────────────────────────────┤
│ useQueryUIStore          │ FEAT0604 (Query Mode Selector)                │
│                          │ FEAT0603 (Streaming state management)         │
├──────────────────────────┼───────────────────────────────────────────────┤
│ useSettingsStore         │ FEAT0613 (Dark/Light Theme)                   │
│                          │ FEAT0617 (User Preference Persistence)        │
│                          │ FEAT0618 (Graph Layout Settings)              │
│                          │ FEAT0619 (Ingestion Quality Settings)         │
├──────────────────────────┼───────────────────────────────────────────────┤
│ useTenantStore           │ FEAT0606 (Workspace Switcher)                 │
│                          │ FEAT0015 (Multi-Tenant Isolation)             │
├──────────────────────────┼───────────────────────────────────────────────┤
│ useUIPreferencesStore    │ FEAT0617 (UI preference persistence)          │
│                          │ BR0609 (Theme persists across sessions)       │
└──────────────────────────┴───────────────────────────────────────────────┘
```

---

## 3. The Graph Store (`use-graph-store.ts`)

This is the largest and most critical state module (~1000 lines). It orchestrates the capabilities of the Graph Explorer.

### 3.1 Architecture

The Graph Store is not just data; it is a **View Model** for the canvas. It bridges the raw API data (Nodes/Edges) with the visual requirements of Sigma.js.

**State Shape**:

```typescript
interface GraphState {
  // 1. Data Layer (Raw)
  rawNodes: GraphNode[];
  rawEdges: GraphEdge[];

  // 2. Visual Layer (Computed/filtered)
  graph: GraphologyGraph; // The actual graph object used by renderer

  // 3. Selection State
  selectedNodeId: string | null;
  hoveredNodeId: string | null;

  // 4. Filtering State
  searchQuery: string;
  activeCommunities: string[];
  minDegree: number;

  // 5. Actions
  setGraphData: (data: KnowledgeGraph) => void;
  focusNode: (id: string) => void;
}
```

### 3.2 Handling Large Datasets

To maintain 60FPS with 5,000+ nodes:

1.  **Transient Updates**: We avoid React re-renders for camera moves.
2.  **Worker Layout**: The store triggers layout computations in a Web Worker, updating the graph object reference only when stable.
3.  **Debounced Filtering**: Search inputs update a local state first, then debounce-update the heavy store filter.

---

## 4. Performance Patterns

### 4.1 Atomic Selectors

We strictly use atomic selectors to prevent unnecessary re-renders.

**❌ Bad (Causes re-render on _any_ store change)**:

```tsx
const { selectedNodeId, setGraphData } = useGraphStore();
```

**✅ Good (Re-renders only when specific slice changes)**:

```tsx
const selectedNodeId = useGraphStore((state) => state.selectedNodeId);
```

### 4.2 Computed State

We deliberately avoid derived state in the store _if_ it's cheap to compute. Costly derivations (like subgraph filtering) are handled via `useEffect` inside the store actions or custom hooks, updating a cached property in the store.

---

## 5. Persistence Middleware

We use Zustand's `persist` middleware for User Preferences to ensure the UI looks the same on reload.

```typescript
// src/stores/use-ui-preferences.ts
export const useUiPreferences = create<UiState>()(
  persist(
    (set) => ({
      sidebarOpen: true,
      toggleSidebar: () =>
        set((state) => ({ sidebarOpen: !state.sidebarOpen })),
    }),
    {
      name: "edgequake-ui-prefs", // localStorage key
      partialize: (state) => ({ sidebarOpen: state.sidebarOpen }), // Whitelist
    }
  )
);
```

---

## 6. Server State (React Query)

While Zustand holds the _active_ state, React Query fetches the _source_.

**Typical Flow**:

1.  React Query fetches `GET /graph`.
2.  `onSuccess` callback pushes data into `useGraphStore.setData()`.
3.  GraphStore processes raw data -> Graphology Object.
4.  UI renders from GraphStore.

**Why duplicate?**
The raw JSON from API isn't render-ready (needs position x/y, color mapping, size scaling). The Zustand store acts as the **Transformation Layer**.

---

## 7. Custom Hooks Catalog

All hooks are located in `src/hooks/`. They provide reusable logic patterns across components.

### 7.1 Data Fetching Hooks

| Hook               | Purpose                 | Features                  |
| ------------------ | ----------------------- | ------------------------- |
| `useConversations` | Fetch conversation list | React Query + cache       |
| `useCost`          | Fetch cost estimates    | Token tracking            |
| `useFolders`       | Fetch folder hierarchy  | Tree structure            |
| `useLineage`       | Fetch document lineage  | Source tracking, FEAT0615 |

### 7.2 Graph Hooks

| Hook                         | Purpose               | Features                   |
| ---------------------------- | --------------------- | -------------------------- |
| `useGraphExpansion`          | Expand node neighbors | Lazy loading               |
| `useGraphKeyboardNavigation` | Arrow key navigation  | FEAT0612                   |
| `useGraphStream`             | SSE graph streaming   | Progressive load, FEAT0603 |

### 7.3 UI Utility Hooks

| Hook                   | Purpose                | Features              |
| ---------------------- | ---------------------- | --------------------- |
| `useAutoResize`        | Auto-resize textarea   | Performance optimized |
| `useDebounce`          | Debounce values        | 300ms default         |
| `useKeyboardShortcuts` | Global shortcuts       | FEAT0612              |
| `useMediaQuery`        | Responsive breakpoints | SSR-safe              |
| `useStoreHydration`    | SSR hydration guard    | Next.js compatible    |
| `useUrlState`          | URL query params       | Shareable state       |

### 7.4 Context Hooks

| Hook                      | Purpose                  | Features           |
| ------------------------- | ------------------------ | ------------------ |
| `useIngestionProgress`    | Track upload progress    | SSE, FEAT0611      |
| `useMigrateConversations` | Migrate v1 → v2 schema   | One-time migration |
| `useQueryPageState`       | Query page coordination  | Cross-component    |
| `useTenantContext`        | Current tenant/workspace | FEAT0606           |
| `useWebsocket`            | WebSocket connection     | Auto-reconnect     |
| `useWorkspaceUrl`         | Workspace URL builder    | Route helpers      |

---

## 8. State Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         React Components                            │
│                                                                     │
│   ┌───────────────┐   ┌───────────────┐   ┌───────────────┐        │
│   │   GraphPage   │   │   QueryPage   │   │  DocumentPage │        │
│   └───────┬───────┘   └───────┬───────┘   └───────┬───────┘        │
└───────────┼───────────────────┼───────────────────┼────────────────┘
            │                   │                   │
            ▼                   ▼                   ▼
┌───────────────────────────────────────────────────────────────────┐
│                        Custom Hooks Layer                          │
│                                                                    │
│  useGraphStream    useConversations    useIngestionProgress        │
│  useLineage        useCost             useWorkspaceUrl             │
└──────────────────────────────┬─────────────────────────────────────┘
                               │
            ┌──────────────────┼──────────────────┐
            ▼                  ▼                  ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  Zustand Stores │  │  React Query    │  │  URL State      │
│                 │  │                 │  │                 │
│ useGraphStore   │  │ useQuery(...)   │  │ useSearchParams │
│ useQueryStore   │  │ useMutation(...)│  │ usePathname     │
│ useSettingsStore│  │ useInfinite...  │  │                 │
│ useTenantStore  │  │                 │  │                 │
└────────┬────────┘  └────────┬────────┘  └────────┬────────┘
         │                    │                    │
         │    localStorage    │    HTTP/SSE        │    URL
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│   Persistence   │  │ EdgeQuake API   │  │    Browser      │
│   (IndexedDB/   │  │ REST + SSE      │  │    History      │
│    localStorage)│  │                 │  │                 │
└─────────────────┘  └─────────────────┘  └─────────────────┘
```

---

## 9. Related Documents

- [Features Registry](features.md) - FEAT06XX WebUI features
- [Business Rules](business_rules.md) - BR06XX WebUI rules
- [WebUI Architecture](0011-webui-architecture.md) - System overview
- [WebUI Components](0012-webui-components.md) - Component catalog
