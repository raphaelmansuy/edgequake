# Observe Phase - Iteration 14: WebUI Experience

## Research Focus

This iteration explores the EdgeQuake WebUI—a modern React 19 + Next.js 16 application that provides the user interface for interacting with the EdgeQuake Graph-RAG framework.

## Technical Stack

| Technology     | Version | Purpose               |
| -------------- | ------- | --------------------- |
| Next.js        | 16.1.0  | App Router framework  |
| React          | 19.2.3  | UI library            |
| Tailwind CSS   | 4.1.18  | Styling               |
| shadcn/ui      | Latest  | Component library     |
| Zustand        | 5.0.9   | State management      |
| TanStack Query | 5.90.12 | Data fetching         |
| Sigma.js       | 3.0.2   | Graph visualization   |
| Graphology     | Latest  | Graph data structures |
| Lucide React   | Latest  | Icons                 |

## Key Components

### 1. Query Interface (`query-interface.tsx`)

**Lines**: 897 lines
**Features**:

- Natural language query input
- Streaming responses with chain-of-thought display
- Query mode selector (local, global, hybrid, naive)
- Conversation history panel
- Source citations display
- Provider/model selector

**Implements**:

- UC0201: User submits natural language query
- UC0202: System retrieves relevant context from knowledge graph
- UC0203: System generates augmented response with citations
- FEAT0007: Natural Language Query Processing
- FEAT0101-0106: Query mode selection
- FEAT0734: Streaming responses with chain-of-thought display

### 2. Query Mode Selector (`query-mode-selector.tsx`)

**Lines**: 111 lines
**Modes**:
| Mode | Icon | Description |
|------|------|-------------|
| Local | Target | Neighborhood search |
| Global | Globe | Full graph search |
| Hybrid | Layers | Combined local+global |
| Naive | Zap | Direct LLM, no graph context |

### 3. Graph Viewer (`graph-viewer.tsx`)

**Lines**: 785 lines
**Features**:

- Interactive Sigma.js visualization
- Entity browser panel
- Node details panel
- Filtering by entity type and relationship
- Minimap for large graphs
- Time-based filtering
- Keyboard shortcuts
- Context menus
- Graph export
- Streaming indicator for progressive loading
- Layout controls (force-directed, circular, grid)

**Implements**:

- UC0101: User explores knowledge graph visually
- UC0104: User filters entities by type, date, or relationship
- UC0107: User exports graph for analysis
- FEAT0601: Interactive graph visualization with Sigma.js
- BR0009: Graph must handle 1000+ nodes performantly

### 4. Document Manager (`document-manager.tsx`)

**Lines**: 1492 lines
**Features**:

- Drag-and-drop file upload
- Batch document processing
- Progress tracking per document
- Status badges (pending, processing, completed, failed)
- Reprocess failed documents
- Delete documents (cascade to entities)
- Cost tracking per document
- Document preview panel
- Pipeline status monitoring

**Implements**:

- UC0001: User uploads documents for ingestion
- UC0007: User monitors document processing progress
- UC0008: User reprocesses failed documents
- UC0009: User deletes documents from knowledge graph
- FEAT0001: Document ingestion with entity extraction
- BR0302: Failed documents can be reprocessed
- BR0305: Cost tracking per document ingestion

### 5. Streaming Markdown Renderer

**Lines**: 442 lines
**Features**:

- Token-based rendering using marked.js
- LLM streaming normalization (fixes tokenizer artifacts)
- Lazy-loaded components (code, math, diagrams)
- Table buffering to prevent broken rendering
- Throttled auto-scroll for 60fps experience
- KaTeX math support
- Mermaid diagram rendering
- GitHub-style alerts
- Syntax-highlighted code blocks

**Challenge**: LLM tokenizers add leading spaces that break markdown syntax during streaming. The component includes sophisticated normalization logic.

## Directory Structure

```
edgequake_webui/src/
├── app/                      # Next.js App Router
│   ├── (dashboard)/          # Main pages
│   │   ├── graph/           # Graph viewer page
│   │   ├── documents/       # Document management page
│   │   ├── query/           # Query interface page
│   │   └── settings/        # Settings page
├── components/
│   ├── query/               # 17 components
│   │   ├── query-interface.tsx
│   │   ├── query-mode-selector.tsx
│   │   ├── chat-message.tsx
│   │   ├── source-citations.tsx
│   │   ├── thinking-display.tsx
│   │   └── markdown/        # 11 sub-components
│   ├── graph/               # 26 components
│   │   ├── graph-viewer.tsx
│   │   ├── graph-renderer.tsx
│   │   ├── node-details.tsx
│   │   └── ...
│   ├── documents/           # 18 components
│   │   ├── document-manager.tsx
│   │   ├── ingestion-progress-panel.tsx
│   │   └── ...
│   └── ui/                  # shadcn/ui components
├── hooks/                   # Custom React hooks
├── stores/                  # Zustand state stores
└── lib/                     # Utilities and API client
```

## Component Count

| Category    | Components |
| ----------- | ---------- |
| Query       | 17+        |
| Graph       | 26+        |
| Documents   | 18+        |
| UI (shadcn) | 40+        |
| Layout      | 5+         |
| **Total**   | **100+**   |

## Responsive Design

- **Desktop**: Full layout with sidebars
- **Tablet**: Collapsed panels, optimized touch targets
- **Mobile**: Drawer-based navigation, mobile-specific panels

Breakpoints:

- Mobile: ≤640px
- Tablet: 641px-1024px
- Desktop: >1024px

## Key UX Patterns

1. **Streaming Responses**: Real-time token streaming with thinking indicators
2. **Progressive Loading**: Graph loads incrementally with streaming indicator
3. **Optimistic Updates**: UI updates before API confirmation
4. **Error Recovery**: Retry buttons, clear error messages
5. **Accessibility**: ARIA labels, keyboard navigation
6. **Internationalization**: i18n support via react-i18next

## Code Quality

All major components include:

- JSDoc documentation with `@implements` tags
- Use case references (UC0001, etc.)
- Feature references (FEAT0001, etc.)
- Business rule enforcement (BR0001, etc.)

Example from `query-interface.tsx`:

```tsx
/**
 * @implements UC0201 - User submits a natural language query
 * @implements UC0202 - System retrieves relevant context
 * @implements FEAT0007 - Natural Language Query Processing
 * @enforces BR0104 - Query response must include source citations
 */
```

## Technology Highlights

1. **React 19**: Latest React with concurrent features
2. **Next.js 16 App Router**: Server components, streaming
3. **Zustand**: Minimal state management (no boilerplate)
4. **TanStack Query**: Automatic caching, background refetch
5. **Sigma.js 3.0**: WebGL-accelerated graph rendering
6. **shadcn/ui**: Copy-paste components, fully customizable
