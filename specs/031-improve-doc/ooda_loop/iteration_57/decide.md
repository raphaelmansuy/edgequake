# OODA Loop Iteration 57: Decide - WebUI Documentation Plan

## Documentation Structure Decision

Based on the Orient phase analysis, I've decided on the following documentation structure and content plan.

### 1. Primary Documentation Files to Create

| File                                   | Purpose                      | Priority    | Estimated Size |
| -------------------------------------- | ---------------------------- | ----------- | -------------- |
| `docs/0011-webui-architecture.md`      | WebUI system architecture    | 🔴 Critical | ~2500 lines    |
| `docs/0012-webui-components.md`        | Component catalog & usage    | 🔴 Critical | ~2000 lines    |
| `docs/0013-webui-api-integration.md`   | API client patterns          | 🔴 Critical | ~1500 lines    |
| `docs/0014-webui-state-management.md`  | Zustand stores & React Query | 🟡 High     | ~1200 lines    |
| `docs/0015-webui-development-guide.md` | Dev setup & workflows        | 🟡 High     | ~1000 lines    |
| `docs/0016-webui-testing-guide.md`     | Vitest & Playwright          | 🟠 Medium   | ~800 lines     |
| `docs/0017-webui-deployment.md`        | Production deployment        | 🟠 Medium   | ~600 lines     |

### 2. Documentation Hierarchy

```
docs/
├── README.md                              # Update with WebUI section
├── 0001-quick-start.md                    # Add WebUI setup
├── 0002-architecture-overview.md          # Add WebUI layer diagram
├── 0003-api-reference.md                  # Add React Query examples
│
├── 0011-webui-architecture.md             # NEW - WebUI core doc
│   ├── Technology Stack
│   ├── Next.js App Router Structure
│   ├── Component Architecture
│   ├── State Management Overview
│   ├── Performance Optimizations
│   └── Design Decisions & Trade-offs
│
├── 0012-webui-components.md               # NEW - Component catalog
│   ├── UI Primitives (shadcn/ui)
│   ├── Graph Visualization Components
│   ├── Document Management Components
│   ├── Query Interface Components
│   ├── Layout Components
│   └── Shared Utilities
│
├── 0013-webui-api-integration.md          # NEW - API patterns
│   ├── API Client Architecture
│   ├── React Query Integration
│   ├── Streaming Responses (SSE)
│   ├── WebSocket Connections
│   ├── Error Handling
│   └── Type Safety
│
├── 0014-webui-state-management.md         # NEW - State guide
│   ├── Zustand Store Patterns
│   ├── React Query Server State
│   ├── Local Component State
│   ├── Context Providers
│   └── State Flow Diagrams
│
├── 0015-webui-development-guide.md        # NEW - Dev guide
│   ├── Local Setup
│   ├── Development Workflow
│   ├── Hot Reload & Debugging
│   ├── Code Style & Linting
│   └── Common Tasks
│
├── 0016-webui-testing-guide.md            # NEW - Testing
│   ├── Unit Tests (Vitest)
│   ├── E2E Tests (Playwright)
│   ├── Component Testing
│   └── API Mocking
│
└── 0017-webui-deployment.md               # NEW - Deployment
    ├── Production Build
    ├── Docker Configuration
    ├── Environment Variables
    └── Performance Monitoring
```

### 3. Content Template for 0011-webui-architecture.md

**Planned sections** (2500 lines):

#### Section 1: Overview (200 lines)

- Introduction to EdgeQuake WebUI
- Technology stack summary table
- Architecture diagram (ASCII art)
- Core design principles

#### Section 2: Next.js App Router (400 lines)

- Route structure explanation
- Route groups (`(auth)`, `(dashboard)`)
- Server vs. Client Components
- Dynamic routes (`[id]`)
- API routes
- Code splitting strategy
- ASCII diagram of route tree

#### Section 3: Component Architecture (500 lines)

- Component hierarchy (Atomic Design)
- UI primitives (shadcn/ui + Radix)
- Smart vs. Presentational pattern
- Component file organization
- Props typing patterns
- Composition examples

#### Section 4: State Management (600 lines)

- Three-tier architecture diagram
- Zustand global state (10 stores overview)
- React Query server state
- Local component state
- Context providers
- State flow ASCII diagrams
- Example: Graph store walkthrough

#### Section 5: Graph Visualization (400 lines)

- Sigma.js + Graphology stack
- Rendering pipeline diagram
- Layout algorithms (ForceAtlas2)
- Web Worker usage
- Performance optimizations
- Virtual scrolling for entity browser

#### Section 6: API Integration (300 lines)

- Type-safe API client
- Streaming responses (SSE)
- WebSocket for progress
- Error handling patterns
- Request/response flow diagram

#### Section 7: Performance Optimizations (200 lines)

- Code splitting strategy
- React.memo usage
- Debouncing patterns
- Virtual scrolling
- Bundle size analysis

#### Section 8: Accessibility (100 lines)

- Keyboard navigation
- ARIA attributes
- Screen reader support
- WCAG 2.1 AA compliance

#### Section 9: Internationalization (100 lines)

- react-i18next setup
- Translation file structure
- Adding new locales

#### Section 10: Design Decisions (200 lines)

- Why Zustand over Redux?
- Why React Query?
- Why Sigma.js over D3?
- Why App Router over Pages Router?
- Why shadcn/ui over Material UI?
- Trade-offs analysis

### 4. Content Template for 0012-webui-components.md

**Planned sections** (2000 lines):

#### Section 1: Component Catalog Overview (100 lines)

- Total component count (156+)
- Component categories
- Usage conventions

#### Section 2: UI Primitives (shadcn/ui) (400 lines)

- 35 base components documented
- Button, Input, Select, Dialog, etc.
- Props reference tables
- Usage examples

#### Section 3: Graph Components (500 lines)

- GraphViewer (main component)
- GraphControls (zoom, reset, layout)
- GraphFilters (type, date, search)
- GraphSearch
- GraphLegend
- NodeDetails panel
- EntityBrowser panel
- Props + examples for each

#### Section 4: Document Components (300 lines)

- DocumentManager
- DocumentTable
- UploadDialog
- DocumentDetail
- LineageTree
- ChunkExplorer
- Props + examples

#### Section 5: Query Components (300 lines)

- QueryInterface
- QueryResultStream
- SourceCitations
- ConversationHistory
- QueryModeSelector
- Props + examples

#### Section 6: Layout Components (200 lines)

- Sidebar
- Header
- DynamicBreadcrumb
- TenantSelector
- Props + examples

#### Section 7: Shared Components (200 lines)

- DataTable (generic)
- MarkdownRenderer
- EmptyState
- LoadingSpinner
- Props + examples

### 5. Content Template for 0013-webui-api-integration.md

**Planned sections** (1500 lines):

#### Section 1: API Client Overview (200 lines)

- Architecture diagram
- Type safety approach
- Error handling strategy

#### Section 2: REST API Client (400 lines)

- `lib/api/edgequake.ts` structure
- Function signatures
- Request/response types
- Example: Document upload
- Example: Query execution

#### Section 3: React Query Integration (400 lines)

- Query hooks pattern
- Mutation hooks pattern
- Cache invalidation strategy
- Optimistic updates
- Loading/error states
- Example: Document list query
- Example: Delete document mutation

#### Section 4: Streaming Responses (300 lines)

- SSE (Server-Sent Events) protocol
- `streamClient` implementation
- Usage in query interface
- Cancellation handling
- Example: Streaming LLM response

#### Section 5: WebSocket Integration (200 lines)

- WebSocket client (`progress-websocket.ts`)
- Reconnection strategy
- Progress event handling
- Example: Document ingestion progress

### 6. ASCII Diagrams to Include

#### Diagram 1: System Architecture (3-tier)

```
┌─────────────────────────────────────────────┐
│         EdgeQuake WebUI (Next.js 16)        │
│  ┌─────────────────────────────────────┐   │
│  │  Client (Browser)                   │   │
│  │  - React 19 Components              │   │
│  │  - Zustand State                    │   │
│  │  - React Query Cache                │   │
│  └─────────────────────────────────────┘   │
│                    ↕                        │
│  ┌─────────────────────────────────────┐   │
│  │  Server Components (Next.js)        │   │
│  │  - SSR for initial load             │   │
│  │  - API Route handlers               │   │
│  └─────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
                    ↕ HTTP/SSE/WebSocket
┌─────────────────────────────────────────────┐
│       EdgeQuake Backend (Rust/Axum)         │
│  - REST API (/api/v1/*)                     │
│  - SSE streaming                            │
│  - WebSocket (/ws/progress)                 │
└─────────────────────────────────────────────┘
                    ↕
┌─────────────────────────────────────────────┐
│       Storage Layer                         │
│  - PostgreSQL (KV, Vector, Graph)           │
│  - Memory (testing)                         │
└─────────────────────────────────────────────┘
```

#### Diagram 2: Component Hierarchy

```
App Router (Next.js 16)
├── Layout (Root)
│   ├── QueryProvider (TanStack Query)
│   ├── ThemeProvider (Dark/Light)
│   └── TenantProvider (Multi-tenancy)
│
├── (auth) Route Group
│   ├── /login → LoginPage
│   └── /select-tenant → TenantSelector
│
└── (dashboard) Route Group
    ├── Layout (Sidebar + Header)
    │
    ├── /graph → GraphPage
    │   └── GraphViewer (Client Component)
    │       ├── SigmaContainer
    │       ├── GraphControls
    │       ├── GraphFilters
    │       └── NodeDetails Panel
    │
    ├── /documents → DocumentsPage
    │   └── DocumentManager
    │       ├── DocumentTable
    │       ├── UploadDialog
    │       └── FilterPanel
    │
    └── /query → QueryPage
        └── QueryInterface
            ├── QueryInput
            ├── StreamingResponse
            └── SourceCitations
```

#### Diagram 3: State Management Flow

```
User Action
    ↓
Component Event Handler
    ↓
┌─────────────────────────────────────┐
│ Decision: Which state layer?        │
└─────────────────────────────────────┘
    ↓           ↓              ↓
    │           │              │
Ephemeral?   Global?       Server Data?
    ↓           ↓              ↓
useState   Zustand Store  React Query
    │           │              │
    │           ├─> Derived State
    │           │   (selectors)
    │           │
    │           └─> Actions
    │               (mutations)
    ↓           ↓              ↓
Re-render affected components
```

### 7. Documentation Standards

#### Code Example Format

````typescript
// ✅ GOOD: Type-safe function with JSDoc
/**
 * Fetch knowledge graph data with optional filters.
 *
 * @param workspace - Workspace ID
 * @param filters - Optional filters (entity types, date range)
 * @returns Knowledge graph with nodes and edges
 * @throws {APIError} If request fails
 *
 * @example
 * ```tsx
 * const graph = await getKnowledgeGraph('ws_123', {
 *   entityTypes: ['PERSON', 'ORGANIZATION'],
 *   startDate: '2024-01-01'
 * });
 * ```
 */
export async function getKnowledgeGraph(
  workspace: string,
  filters?: GraphFilters
): Promise<KnowledgeGraph> {
  return api.get(`/workspaces/${workspace}/graph`, { params: filters });
}
````

#### Component Documentation Format

````tsx
/**
 * Graph visualization component using Sigma.js.
 *
 * @component
 * @implements FEAT0601 - Interactive graph visualization
 * @see GraphStore for state management
 *
 * @example
 * ```tsx
 * <GraphViewer
 *   initialLayout="force"
 *   showControls={true}
 *   showMinimap={false}
 * />
 * ```
 */
export function GraphViewer({
  initialLayout = "force",
  showControls = true,
  showMinimap = false,
}: GraphViewerProps) {
  // ...
}
````

### 8. Cross-Reference Strategy

#### Link to Features Registry

Every component/API function should reference FEAT codes:

```tsx
/**
 * @implements FEAT0601 - Knowledge Graph Visualization
 * @implements FEAT0101 - Entity Neighborhood Exploration
 */
export function GraphViewer() { ... }
```

#### Link to Use Cases

User-facing flows should reference UC codes:

```tsx
/**
 * Document upload dialog.
 *
 * @implements UC0001 - Upload Document
 * @implements UC0031 - Track Upload Progress
 */
export function UploadDialog() { ... }
```

#### Link to Business Rules

Validation/constraints should reference BR codes:

```tsx
/**
 * @enforces BR0009 - Max 1000 nodes per graph
 * @enforces BR0201 - Tenant isolation
 */
export function GraphFilters() { ... }
```

### 9. Features to Add to features.md

**New WebUI features** (FEAT0601-0620):

| ID       | Feature                               | Component           | Status |
| -------- | ------------------------------------- | ------------------- | ------ |
| FEAT0601 | Interactive Graph Visualization       | GraphViewer         | ✅     |
| FEAT0602 | Graph Filtering (Type, Date, Search)  | GraphFilters        | ✅     |
| FEAT0603 | Graph Layout Algorithms               | GraphControls       | ✅     |
| FEAT0604 | Entity Browser with Virtual Scrolling | EntityBrowser       | ✅     |
| FEAT0605 | Node Details Panel                    | NodeDetails         | ✅     |
| FEAT0606 | Graph Bookmarks                       | GraphBookmarks      | ✅     |
| FEAT0607 | Document Upload UI                    | UploadDialog        | ✅     |
| FEAT0608 | Document Management Table             | DocumentManager     | ✅     |
| FEAT0609 | Document Detail View                  | DocumentDetail      | ✅     |
| FEAT0610 | Data Lineage Visualization            | LineageTree         | ✅     |
| FEAT0611 | Query Interface with Streaming        | QueryInterface      | ✅     |
| FEAT0612 | Source Citations Display              | SourceCitations     | ✅     |
| FEAT0613 | Conversation History                  | ConversationHistory | ✅     |
| FEAT0614 | Cost Tracking Dashboard               | CostDashboard       | ✅     |
| FEAT0615 | Token Usage Analytics                 | TokenUsageTable     | ✅     |
| FEAT0616 | API Explorer UI                       | APIExplorer         | ✅     |
| FEAT0617 | Settings Panel                        | SettingsPanel       | ✅     |
| FEAT0618 | Dark/Light Theme Toggle               | ThemeToggle         | ✅     |
| FEAT0619 | Multi-Tenant Workspace Selector       | TenantSelector      | ✅     |
| FEAT0620 | Keyboard Shortcuts                    | KeyboardShortcuts   | ✅     |

### 10. Use Cases to Add to use_cases.md

**New WebUI workflows** (UC0031-0050):

| ID     | Use Case                     | User Journey                                      | Components                     |
| ------ | ---------------------------- | ------------------------------------------------- | ------------------------------ |
| UC0031 | Upload Document via UI       | Login → Documents → Upload → Track Progress       | UploadDialog, DocumentManager  |
| UC0032 | Explore Graph Visualization  | Login → Graph → Filter → Select Node              | GraphViewer, GraphFilters      |
| UC0033 | Execute Query with Streaming | Login → Query → Type Question → View Stream       | QueryInterface                 |
| UC0034 | View Document Detail         | Documents → Click Row → View Lineage              | DocumentDetail, LineageTree    |
| UC0035 | Track Ingestion Cost         | Documents → Upload → View Cost Breakdown          | CostDashboard, TokenUsageTable |
| UC0036 | Save Graph View as Bookmark  | Graph → Configure View → Save Bookmark            | GraphBookmarks                 |
| UC0037 | Switch Workspace             | Header → Tenant Selector → Select Workspace       | TenantSelector                 |
| UC0038 | View Entity Neighborhood     | Graph → Select Node → View Neighbors              | GraphViewer, NodeDetails       |
| UC0039 | Test API Endpoint            | API Explorer → Select Endpoint → Execute          | APIExplorer                    |
| UC0040 | Change Graph Layout          | Graph → Controls → Select Layout (Force/Circular) | GraphControls                  |

### 11. Implementation Plan

#### Phase 1: Core Documentation (Iteration 57)

- [x] Observe - Audit codebase (DONE)
- [x] Orient - Analyze patterns (DONE)
- [x] Decide - Plan structure (THIS DOCUMENT)
- [ ] Act - Create `docs/0011-webui-architecture.md`

#### Phase 2: Component Catalog (Iteration 58)

- [ ] Create `docs/0012-webui-components.md`
- [ ] Document 156+ components with examples

#### Phase 3: API Integration (Iteration 59)

- [ ] Create `docs/0013-webui-api-integration.md`
- [ ] Document React Query patterns

#### Phase 4: State Management (Iteration 60)

- [ ] Create `docs/0014-webui-state-management.md`
- [ ] Document 10 Zustand stores

#### Phase 5: Dev Guide (Iteration 61)

- [ ] Create `docs/0015-webui-development-guide.md`
- [ ] Setup instructions, workflows

#### Phase 6: Testing & Deployment (Iteration 62-63)

- [ ] Create `docs/0016-webui-testing-guide.md`
- [ ] Create `docs/0017-webui-deployment.md`

#### Phase 7: Update Existing Docs (Iteration 64-65)

- [ ] Update `docs/README.md` with WebUI section
- [ ] Update `docs/0001-quick-start.md` with WebUI setup
- [ ] Update `docs/0002-architecture-overview.md` with WebUI layer
- [ ] Update `docs/0003-api-reference.md` with React Query examples

#### Phase 8: Features & Use Cases (Iteration 66-67)

- [ ] Add FEAT0601-0620 to `docs/features.md`
- [ ] Add UC0031-0050 to `docs/use_cases.md`

#### Phase 9: Diagrams & Cleanup (Iteration 68-70)

- [ ] Add ASCII diagrams to architecture docs
- [ ] Clean up `docs/archive` directory
- [ ] Final cross-reference audit

### 12. Success Metrics

| Metric                     | Target                         | Measurement                    |
| -------------------------- | ------------------------------ | ------------------------------ |
| **Documentation coverage** | 100% of WebUI modules          | Files documented / Total files |
| **Code examples**          | 50+ working examples           | Count in docs                  |
| **ASCII diagrams**         | 10+ high-signal diagrams       | Visual aids count              |
| **Cross-references**       | 100% FEAT/UC links             | Grep for @implements           |
| **Accuracy**               | 0 broken file paths            | Automated check script         |
| **Completeness**           | All 156+ components documented | Component catalog              |

## Decision Summary

**Decided to create 7 new documentation files** covering:

1. ✅ Architecture (0011)
2. ✅ Components (0012)
3. ✅ API Integration (0013)
4. ✅ State Management (0014)
5. ✅ Development (0015)
6. ✅ Testing (0016)
7. ✅ Deployment (0017)

**Decided on documentation standards**:

- TypeScript code examples with JSDoc
- ASCII diagrams for architecture
- FEAT/BR/UC cross-references
- Props tables for components
- Usage examples for every API function

**Next**: Act phase will implement `docs/0011-webui-architecture.md` (first of 7).

---

**Files Referenced**: N/A (planning document)

**Codebase State**: January 9, 2026 (feat/documentation branch)
