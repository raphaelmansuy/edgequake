# EdgeQuake WebUI Architecture

> Comprehensive guide to the EdgeQuake WebUI client architecture, technology stack, and design patterns.

**Version**: 1.0.0 | **Last Updated**: 2026-01-09

---

## 1. Overview

EdgeQuake WebUI is a high-performance, responsive React application built with **Next.js 16**. It serves as the primary interface for the EdgeQuake RAG platform, providing capabilities for document management, knowledge graph exploration, and natural language querying.

### 1.1 Architecture Goals

1. **Performance**: Handle large graphs (5000+ nodes) and long interactions without lag.
2. **Type Safety**: End-to-end type safety from backend API to UI components.
3. **Resilience**: Graceful error handling and offline-capable state management.
4. **Accessibility**: WCAG 2.1 AA compliance for all interactive elements.
5. **Developer Experience**: Fast iteration loops, strict strict linting, and automated testing.

### 1.2 Technology Stack

| Category | Technology | Version | Purpose |
|----------|-----------|---------|---------|
| **Framework** | Next.js | 16.1.0 | App Router, Server Components, Routing |
| **Language** | TypeScript | 5.3.3 | Static Typing |
| **UI Library** | React | 19.2.3 | Component Model, Hooks |
| **State** | Zustand | 5.0.9 | Client-Side Global State |
| **Data Fetching** | TanStack Query | 5.90.12 | Server State Management, Caching |
| **Visualization** | Sigma.js | 3.0.2 | WebGL Graph Rendering |
| **Styling** | Tailwind CSS | 4.1.18 | Utility-First Styling |
| **Components** | Radix UI | Latest | Accessible Headless Primitives |
| **Testing** | Vitest + Playwright | Latest | Unit and E2E Testing |

---

## 2. System Architecture

EdgeQuake WebUI follows a **Three-Tier State Architecture** designed to separate ephemeral UI state, global application state, and server data.

```
┌─────────────────────────────────────────────┐
│              Presentation Layer             │
│  (Next.js Server & Client Components)       │
└──────────────────────┬──────────────────────┘
                       │
         ┌─────────────▼─────────────┐
         │      State Layer (Client) │
         │                           │
         │  ┌─────────────────────┐  │
         │  │   Zustand Stores    │  │
         │  │ (Session, UI Prefs) │  │
         │  └──────────┬──────────┘  │
         │             │             │
         │  ┌──────────▼──────────┐  │
         │  │  React Query Cache  │  │
         │  │    (Server Data)    │  │
         │  └──────────┬──────────┘  │
         └─────────────┼─────────────┘
                       │
         ┌─────────────▼─────────────┐
         │   EdgeQuake Backend API   │
         │   (REST + SSE + WS)       │
         └───────────────────────────┘
```

### 2.1 State Management Strategy

We strictly categorize state to choose the right tool:

| State Type | Tool | Examples |
|------------|------|----------|
| **Server State** | **TanStack Query** | Document lists, Graph data, User profile |
| **Global Client State** | **Zustand** | Auth token, Theme, Collapsed sidebar, Filters |
| **Local UI State** | **useState/useReducer** | Form inputs, Modal visibility, Hover state |
| **URL State** | **Next.js Router** | Current document ID, Search params, Active tab |

> **Design Rule**: Always prefer URL state for shareable UI states (filters, active views), then Server State for data. Use Zustand only for truly global client session state.

---

## 3. Next.js App Router Structure

The application uses the Next.js App Router (`app/` directory) for modern routing capabilities including Layouts, Loading UI, and Error Boundaries.

### 3.1 Directory Layout

```
src/app/
├── (auth)/                   # Route Group: Authentication context
│   ├── login/                # Path: /login
│   └── select-tenant/        # Path: /select-tenant
│
├── (dashboard)/              # Route Group: Main application context
│   ├── layout.tsx            # Authenticated Shell (Sidebar + Header)
│   ├── page.tsx              # Path: / (Dashboard Home)
│   ├── graph/                # Knowledge Graph Feature
│   │   └── page.tsx          # Path: /graph
│   ├── documents/            # Document Management Feature
│   │   ├── page.tsx          # Path: /documents
│   │   └── [id]/             # Dynamic Route: Document Detail
│   │       └── page.tsx      # Path: /documents/123
│   ├── query/                # Query Interface
│   │   └── page.tsx          # Path: /query
│   └── settings/             # Settings
│       └── page.tsx          # Path: /settings
│
├── api/                      # API Route Handlers
│   └── copilotkit/           # Backend-for-Frontend endpoints
│
├── layout.tsx                # Root Layout (Providers)
└── not-found.tsx             # 404 Page
```

### 3.2 Client vs. Server Components

We follow the **"Leaf Client Components"** pattern to maximize performance:

- **Server Components (Default)**: Used for data fetching, layouts, and static content.
- **Client Components (`'use client'`)**: Used only for interactivity (forms, graph visualization, listeners).

**Example: Graph Page Architecture**

```tsx
// app/(dashboard)/graph/page.tsx (Server Component)
export default function GraphPage() {
  return (
    <div className="h-full w-full">
      {/* Client Component loaded dynamically */}
      <GraphViewer /> 
    </div>
  );
}

// components/graph/graph-viewer.tsx (Client Component)
'use client';

import { useGraphStore } from '@/stores/use-graph-store';

export function GraphViewer() {
  const { nodes, edges } = useGraphStore(); // Access client state
  return <SigmaContainer ... />;
}
```

---

## 4. Component Architecture

EdgeQuake UI follows **Atomic Design** principles integrated with **shadcn/ui**.

### 4.1 Component Layers

1. **Primitives (`components/ui/`)**: Low-level, accessible atoms (Button, Input, Card). Copied from shadcn/ui.
2. **Shared Molecules (`components/shared/`)**: Reusable combinations (DataTable, MarkdownRenderer).
3. **Domain Organisms (`components/documents/`, `components/graph/`)**: Feature-specific complex components.
4. **Layout Templates (`components/layout/`)**: Page structures (Sidebar, Header).
5. **Pages (`app/**/page.tsx`)**: Entry points connecting data to templates.

### 4.2 Pattern: Smart Container / Dumb Presenter

To keep logic testable and components reusable, we separate data fetching/state logic from rendering.

**Smart Container**:
```tsx
// components/documents/document-manager.tsx
export function DocumentManager() {
  const { data: documents } = useQuery(...); // Fetching
  const deleteMutation = useMutation(...);   // Logic

  return (
    <DocumentTable 
      data={documents} 
      onDelete={deleteMutation.mutate} 
    />
  );
}
```

**Dumb Presenter**:
```tsx
// components/documents/document-table.tsx
export function DocumentTable({ data, onDelete }: Props) {
  // Pure rendering logic
  return (
    <Table>
      {data.map(doc => ...)}
    </Table>
  );
}
```

---

## 5. API Integration Pattern

All API interactions are centralized in `src/lib/api/` to ensure type safety and consistent error handling.

### 5.1 Type-Safe Client

We export a typed API client wrapper around `fetch`:

```typescript
// lib/api/client.ts
export const api = {
  get: <T>(url: string) => request<T>(url, { method: 'GET' }),
  post: <T>(url: string, data: unknown) => request<T>(url, { 
    method: 'POST', 
    body: JSON.stringify(data) 
  }),
  // ...
};
```

### 5.2 Streaming Responses (Server-Sent Events)

For AI responses and long-running queries, we use SSE:

```typescript
// lib/api/edgequake.ts
export async function queryStream(
  req: QueryRequest, 
  onChunk: (chunk: QueryStreamChunk) => void
) {
  await streamClient('/query', { body: JSON.stringify(req) }, onChunk);
}
```

### 5.3 WebSocket Integration

WebSockets are used for real-time background job progress (e.g., ingestion status).

- **Manager**: `lib/websocket/websocket-manager.ts` handles connection/reconnection.
- **Hook**: `useWebSocketProgress(workspaceId)` exposes progress state to components.
- **Visuals**: `components/progress/stage-indicator.tsx` renders live updates.

---

## 6. Graph Visualization Engine

The Knowledge Graph visualizer is the most complex component, utilizing **Sigma.js** (WebGL) for rendering up to 10,000+ nodes.

### 6.1 Rendering Pipeline

1. **Data Load**: `useGraphStore` fetches graph JSON (nodes/edges).
2. **Transformation**: Data mapped to `graphology` structure.
3. **Layout**: ForceAtlas2 algorithm runs in a **Web Worker** to prevent UI thread blocking.
4. **Rendering**: Sigma.js renders frames via WebGL canvas.
5. **Interaction**: Event listeners handle hover/click/drag interaction.

### 6.2 Performance Optimizations

- **Web Workers**: Heavy layout calculations offloaded to worker threads.
- **Throttling**: Rerenders throttled to 60fps.
- **Component Memoization**: `React.memo` prevents sidebar rerenders during graph animation.
- **Index-based Lookups**: Zustand store maintains `Map<Id, Node>` for O(1) access.

---

## 7. Performance & Optimization

### 7.1 Code Splitting

Next.js automatically splits code by route. We additionally use `next/dynamic` for heavy client interactive components:

```tsx
const GraphViewer = dynamic(() => import('@/components/graph/graph-viewer'), {
  ssr: false,
  loading: () => <GraphSkeleton />,
});
```

### 7.2 Core Web Vitals Focus

- **LCP (Largest Contentful Paint)**: Main headers/text load instantly via Server Components.
- **CLS (Cumulative Layout Shift)**: Skeletons used during data loading to reserve space.
- **INP (Interaction to Next Paint)**: Heavy computations (graph layout) moved to workers.

---

## 8. Internationalization (i18n)

We use `react-i18next` with a namespace strategy.

- **Translation Files**: `/public/locales/{en,fr,zh}/*.json`
- **Hook**: `const { t } = useTranslation('namespace');`
- **Provider**: `src/providers/i18n-provider.tsx` handles locale loading and switching.

**Why public folder?** Next.js middleware or client-side fetching can easily access static JSON files without bundling them into the main JS bundle.

---

## 9. Design Decisions & Trade-offs

| Decision | Alternative | Rationale |
|----------|-------------|-----------|
| **Zustand** | Redux/Context | Simpler API, no provider hell, smaller bundle size (3KB). |
| **Tailwind CSS** | CSS-in-JS | Zero runtime overhead, colocated styles, consistent design system. |
| **App Router** | Pages Router | Server Components reduce client JS size significantly (-30%). |
| **Sigma.js** | D3.js | WebGL rendering required for <1000 nodes performance. |
| **Vitest** | Jest | Native ESM support, faster execution, Vite compatibility. |

---

## 10. Related Documentation

- [WebUI Component Catalog](0012-webui-components.md) - Detailed component reference.
- [API Integration Patterns](0013-webui-api-integration.md) - Deep dive into caching and streaming.
- [State Management Guide](0014-webui-state-management.md) - Store definitions and flows.
- [Development Guide](0015-webui-development-guide.md) - Setup, commands, and workflow.

