# Scratchpad: EdgeQuake WebUI vs LightRAG WebUI - Deep Analysis

> **Note:** This is an append-only document capturing raw findings during analysis.
> **Date:** 2024-12-23
> **Phase:** Step 3 - Comprehensive Gap Analysis & Enhancement Plan

---

## Analysis Session: 2024-12-23

### Framework Comparison

#### LightRAG WebUI Stack

- **Framework:** Vite + React 19.2.0
- **Router:** react-router-dom (HashRouter)
- **State Management:** Zustand with persist middleware
- **Build Tool:** Bun
- **UI Components:** Radix UI + Tailwind CSS
- **Graph Visualization:** Sigma.js (@react-sigma/core v5.0.4)
- **i18n:** i18next with 5 languages (en, zh, fr, ar, zh_TW)
- **HTTP Client:** Axios
- **Notifications:** Sonner

#### EdgeQuake WebUI Stack

- **Framework:** Next.js 16.1.0 (App Router)
- **Router:** Next.js built-in App Router
- **State Management:** Zustand + React Query (@tanstack/react-query)
- **Build Tool:** Next.js (Bun compatible)
- **UI Components:** Radix UI + Tailwind CSS
- **Graph Visualization:** Sigma.js (@react-sigma/core v5.0.6)
- **i18n:** i18next (partially implemented - 3 languages)
- **HTTP Client:** Fetch API with custom wrapper
- **Notifications:** Sonner v2.0.7

---

### Directory Structure Comparison

#### LightRAG WebUI src/

```
src/
├── App.tsx (286 lines - main app)
├── AppRouter.tsx (88 lines - routing)
├── __tests__/
├── api/
│   ├── client.ts
│   ├── lightrag.ts (833 lines - comprehensive)
│   └── tenant.ts
├── components/
│   ├── ApiKeyAlert.tsx
│   ├── AppSettings.tsx
│   ├── LanguageToggle.tsx
│   ├── Root.tsx
│   ├── TenantSelector.tsx
│   ├── ThemeProvider.tsx
│   ├── ThemeToggle.tsx
│   ├── documents/ (4 components)
│   ├── graph/ (16 components)
│   ├── retrieval/ (2 components)
│   ├── status/ (3 components)
│   └── ui/ (28 components)
├── contexts/
├── features/ (7 feature pages)
├── hooks/ (7 hooks)
├── i18n.ts
├── lib/
├── locales/ (5 language files)
├── services/
├── stores/ (4 stores)
├── types/
└── utils/
```

#### EdgeQuake WebUI src/

```
src/
├── app/
│   ├── (auth)/
│   ├── (dashboard)/
│   │   ├── api-explorer/
│   │   ├── documents/
│   │   ├── graph/
│   │   ├── query/
│   │   ├── settings/
│   │   └── layout.tsx
│   ├── layout.tsx
│   └── page.tsx
├── components/
│   ├── client-only.tsx
│   ├── documents/ (5 components)
│   ├── graph/ (10 components)
│   ├── layout/ (3 components)
│   ├── query/ (5 components)
│   ├── shared/ (3 components)
│   └── ui/ (29 components)
├── hooks/ (2 hooks)
├── lib/
│   └── api/ (2 files)
├── locales/ (3 language files)
├── providers/
├── stores/ (6 stores)
└── types/
```

---

### Feature-by-Feature Deep Dive

#### 1. Internationalization (i18n)

**LightRAG Implementation:**

- Full i18n with i18next
- 5 languages: en, zh, fr, ar, zh_TW
- RTL support for Arabic
- Language persisted in settings store
- Comprehensive translation keys (~500+)
- File: `src/i18n.ts`, `src/locales/*.json`

**EdgeQuake Implementation:**

- i18next installed and configured
- 3 languages: en, zh, fr
- Language selector in header
- Translation keys present (~150)
- Missing RTL support
- Files: `src/locales/*.json`

**Gap Analysis:**

- ✅ i18n framework present
- ⚠️ Missing Arabic and Traditional Chinese
- ⚠️ Missing RTL support
- ⚠️ Incomplete translation coverage
- Action: Expand translation coverage

---

#### 2. Graph Viewer - LightRAG (261 lines)

**Features Present:**

- SigmaContainer with memoized settings
- Theme-aware rendering (dark/light)
- Node drag-and-drop (GraphEvents component)
- Multiple layout controls (circular, force, forceatlas2, etc.)
- Full-screen control
- Node search with MiniSearch fuzzy matching
- Legend display with toggle
- Properties panel (editable)
- Settings panel
- Zoom controls
- Graph labels display
- Focus on node camera control
- Loading overlay with spinner
- Theme switching protection

**Components (16):**

1. EditablePropertyRow.tsx - Inline property editing
2. FocusOnNode.tsx - Camera focus control
3. FullScreenControl.tsx - Full-screen toggle
4. GraphControl.tsx - Main controller
5. GraphLabels.tsx - Label display
6. GraphSearch.tsx - Fuzzy search
7. LayoutsControl.tsx - Layout selector
8. Legend.tsx - Color legend
9. LegendButton.tsx - Legend toggle
10. MergeDialog.tsx - Entity merge confirmation
11. PropertiesView.tsx - Property panel
12. PropertyEditDialog.tsx - Edit modal
13. PropertyRowComponents.tsx - Property row UI
14. Settings.tsx - Graph settings
15. SettingsDisplay.tsx - Settings indicator
16. ZoomControl.tsx - Zoom buttons

---

#### 3. Graph Viewer - EdgeQuake (249 lines)

**Features Present:**

- SigmaContainer with react-query integration
- Basic zoom controls (in/out/reset)
- Node selection
- Node context menu (right-click)
- Node details panel
- Graph filters sidebar
- Layout control (basic)
- Graph search
- Empty state with upload prompt
- Refresh functionality
- Loading skeleton

**Components (10):**

1. graph-context-menu.tsx
2. graph-controls.tsx
3. graph-events.tsx
4. graph-filters.tsx
5. graph-renderer.tsx
6. graph-search.tsx
7. graph-viewer.tsx
8. layout-control.tsx
9. node-context-menu.tsx
10. node-details.tsx

**Missing:**

- ❌ Node drag-and-drop (dedicated GraphEvents)
- ❌ Full-screen mode
- ❌ Legend display
- ❌ Inline property editing
- ❌ Entity merge dialog
- ❌ Advanced settings panel
- ❌ Settings display indicator
- ⚠️ Partial layout algorithms

---

#### 4. Document Manager - LightRAG (1796 lines)

**Features:**

- Comprehensive pagination with page size control
- URL state synchronization
- Status filtering (all, processed, processing, pending, failed)
- Multi-field sorting (created_at, updated_at, id, file_path)
- Pipeline status monitoring with dialog
- Document scanning trigger
- Batch operations (select all, reset status)
- Single document reprocess/delete
- Clear all documents
- File name display toggle
- Metadata tooltips with formatted timestamps
- Progress animation for busy pipeline
- Multi-tenant context support
- Error state handling

**Sub-components:**

- UploadDocumentsDialog.tsx
- ClearDocumentsDialog.tsx
- DeleteDocumentsDialog.tsx
- PipelineStatusDialog.tsx

---

#### 5. Document Manager - EdgeQuake (434 lines)

**Features Present:**

- Drag-and-drop upload with react-dropzone
- Basic table display
- Status badges with icons
- Delete single document
- Delete all with confirmation
- Reprocess single document
- Auto-refresh polling (5s interval)
- Pipeline status query
- Pagination controls
- Document filters (status, sort)

**Missing:**

- ❌ URL state sync
- ❌ Batch selection operations
- ❌ Document scanning trigger
- ❌ Reset document status
- ❌ File name display toggle
- ⚠️ Pipeline status dialog (basic)

---

#### 6. Query/Retrieval Interface - LightRAG (825 lines)

**Features:**

- Multi-mode query (naive, local, global, hybrid, mix, bypass)
- Query mode prefix parsing (/mode query)
- Real-time streaming with NDJSON
- COT (Chain of Thought) display with <think> tags
- Thinking time measurement
- LaTeX rendering (KaTeX with extensions)
- Mermaid diagram rendering
- Markdown with GFM support
- Code syntax highlighting
- User prompt history with persistence
- Conversation history turns
- Scroll-to-bottom auto-follow
- Copy response to clipboard
- Message error state handling
- Tab visibility optimization

**Sub-components:**

- ChatMessage.tsx (515 lines - comprehensive)
- QuerySettings.tsx

---

#### 7. Query Interface - EdgeQuake (496 lines)

**Features Present:**

- Multi-mode query (naive, local, global, hybrid, mix)
- Streaming support (AsyncGenerator)
- COT display (ThinkingDisplay component)
- Markdown rendering (ReactMarkdown)
- LaTeX support (rehype-katex, lazy loaded)
- Mermaid diagram rendering
- Source citations panel
- Query history sidebar
- Favorite queries
- Settings sheet (top_k, temperature, max_tokens)
- Message badges (mode, tokens, duration)

**Missing:**

- ❌ Query mode prefix parsing (/mode)
- ❌ Thinking time measurement display
- ❌ User prompt template history
- ⚠️ Conversation history turns (limited)
- ⚠️ Tab visibility optimization

---

#### 8. API Client Comparison

**LightRAG (833 lines in lightrag.ts):**

- Axios-based with interceptors
- Comprehensive type definitions
- Streaming with NDJSON parsing
- Tenant context headers
- API key support
- Token authentication
- Error handling with typed errors
- All CRUD operations for entities/relationships
- Graph query with depth/nodes limits
- Document scanning/reprocessing
- Pipeline status monitoring

**EdgeQuake (464 lines in edgequake.ts):**

- Fetch API with custom wrapper
- TypeScript types
- Streaming with AsyncGenerator
- Tenant/workspace context
- Token refresh logic
- React Query integration
- Entity/relationship CRUD
- Graph operations
- Document operations
- Task/pipeline management

**Gap:** EdgeQuake API client is well-structured but some endpoints may differ from LightRAG.

---

#### 9. Stores Comparison

**LightRAG Stores:**

- `graph.ts` - Graph state, sigma instance, selection
- `settings.ts` (359 lines) - All app settings, theme, language, query settings
- `state.ts` - Backend state, auth state
- `tenant.ts` - Multi-tenant state

**EdgeQuake Stores:**

- `use-auth-store.ts` - Authentication state
- `use-backend-store.ts` - Backend connection
- `use-graph-store.ts` - Graph state
- `use-query-store.ts` - Query history
- `use-settings-store.ts` (75 lines) - Basic settings
- `use-tenant-store.ts` - Tenant/workspace

**Gap:** EdgeQuake settings store is simpler; needs expansion for graph settings, query settings persistence.

---

#### 10. Chat Streaming Implementation

**LightRAG:**

- Uses axios with `responseType: 'stream'`
- NDJSON parsing
- Chunk-by-chunk processing
- Error callback
- COT parsing in real-time
- Thinking time measurement

**EdgeQuake:**

- Uses Fetch with Response.body.getReader()
- AsyncGenerator pattern
- Chunk types (token, context, done, error)
- React Query mutation integration

**Assessment:** EdgeQuake streaming is well-implemented but needs COT thinking time tracking.

---

#### 11. Testing

**LightRAG:**

- Jest/Vitest setup
- Test directory: `src/__tests__/`
- Unit tests for utilities
- Component tests (implied)

**EdgeQuake:**

- Playwright E2E tests
- Test directory: `e2e/`
- Single test file: `gap-features.spec.ts` (238 lines)
- Tests for navigation, i18n, documents, graph, query

**Gap:** EdgeQuake has E2E tests but no unit tests for components/hooks.

---

#### 12. UI Components Comparison

**LightRAG UI (28 components):**

- Alert, AlertDialog, AsyncSearch, AsyncSelect
- Badge, Button, Card, Checkbox, Command
- DataTable, Dialog, EmptyCard, FileUploader
- Input, NumberInput, PaginationControls
- Popover, Progress, ScrollArea, Select
- Separator, TabContent, Table, Tabs
- Text, Textarea, Tooltip, UserPromptInputWithHistory

**EdgeQuake UI (29 components):**

- alert-dialog, alert, badge, breadcrumb
- button, card, checkbox, collapsible
- command, context-menu, dialog, dropdown-menu
- hover-card, input, label, popover
- progress, scroll-area, select, separator
- sheet, skeleton, slider, sonner
- switch, table, tabs, textarea, tooltip

**Observations:**

- EdgeQuake has more modern component set (sheet, skeleton, slider, switch)
- EdgeQuake missing: AsyncSearch, AsyncSelect, DataTable, NumberInput, UserPromptInputWithHistory
- LightRAG missing: breadcrumb, collapsible, context-menu, dropdown-menu, hover-card

---

### UX/UI Deep Dive

#### Navigation

**LightRAG:**

- Tab-based navigation (Tabs component)
- 4 main tabs: Documents, Knowledge Graph, Retrieval, API
- Status indicator in header
- Tenant selector dropdown

**EdgeQuake:**

- Sidebar navigation (5 items)
- Routes: Graph, Documents, Query, API Explorer, Settings
- Mobile-responsive sidebar (Sheet)
- Breadcrumb navigation
- Connection status indicator

**EdgeQuake Advantages:**

- ✅ Dedicated settings page
- ✅ Breadcrumb navigation
- ✅ Mobile-responsive sidebar

**Gaps:**

- ⚠️ No tab-based view option
- ⚠️ Status indicator less prominent

---

#### Keyboard Shortcuts

**LightRAG:**

- Not explicitly implemented
- Basic browser shortcuts only

**EdgeQuake:**

- Dedicated hook: `use-keyboard-shortcuts.ts`
- Keyboard shortcuts dialog (?)
- Navigation shortcuts implied

**EdgeQuake Advantage:** ✅ Better keyboard accessibility

---

#### Theme Support

**LightRAG:**

- ThemeProvider with system/light/dark
- Theme-aware graph rendering
- Theme switching protection

**EdgeQuake:**

- next-themes integration
- Theme toggle in header
- Graph theme detection

**Parity:** ✅ Both well-implemented

---

### Performance Considerations

**LightRAG:**

- Tab visibility provider (optimization)
- Memoized sigma settings
- Debounced search
- Throttled scroll handling
- Graph cleanup on unmount

**EdgeQuake:**

- React Query caching
- Dynamic imports for heavy components
- Skeleton loading states
- Lazy KaTeX loading
- Mermaid lazy initialization

**EdgeQuake Advantages:**

- ✅ Better loading states with skeletons
- ✅ React Query for data caching
- ✅ SSR support with dynamic imports

**Gaps:**

- ❌ No tab visibility optimization
- ❌ Less aggressive memoization

---

### Security & Authentication

**LightRAG:**

- API key support (X-API-Key header)
- JWT token authentication
- Login page
- Token storage in localStorage
- Navigation service for redirects

**EdgeQuake:**

- JWT token with refresh
- Auth store with Zustand
- Token interceptor in API client
- Multi-tenant context headers

**Parity:** ✅ Similar security model

---

### Multi-Tenant Support

**LightRAG:**

- Tenant selection page
- KB (Knowledge Base) selection
- Tenant context headers
- Single/multi-tenant mode detection

**EdgeQuake:**

- Tenant and workspace context
- API endpoints for tenant/workspace CRUD
- Context headers in API client

**EdgeQuake Advantage:** ✅ More complete tenant/workspace model

---

### Streaming Chat Reliability

**Current EdgeQuake Implementation:**

```typescript
export async function* queryStream(
  request: QueryRequest
): AsyncGenerator<QueryStreamChunk> {
  yield* streamClient<QueryStreamChunk>("/query/stream", {
    method: "POST",
    body: JSON.stringify({ ...request, stream: true }),
  });
}
```

**LightRAG Implementation:**

```typescript
export const queryTextStream = async (
  request: QueryRequest,
  onChunk: (chunk: string) => void,
  onError?: (error: string) => void
) => {
  // Uses NDJSON parsing with detailed error handling
  // Includes tenant context headers
  // Handles connection errors gracefully
};
```

**Required Improvements for EdgeQuake:**

1. Add reconnection logic on network failure
2. Add timeout handling
3. Add progress indication
4. Add COT thinking time measurement
5. Better error recovery

---

## Summary of Critical Gaps

### High Priority (Must Have)

1. ~~i18n~~ - Partially implemented, needs expansion
2. Node drag-and-drop in graph
3. Full-screen graph mode
4. Document pagination with URL sync
5. Pipeline status dialog
6. LaTeX rendering - ✅ Implemented
7. Mermaid diagrams - ✅ Implemented
8. COT thinking time display
9. Entity merge functionality
10. Inline property editing

### Medium Priority (Should Have)

1. Graph legend display
2. Advanced graph settings panel
3. Query mode prefix parsing
4. User prompt history
5. Document scanning trigger
6. Batch document operations
7. Tab visibility optimization
8. Arabic/Traditional Chinese locales

### Low Priority (Nice to Have)

1. Graph settings indicator
2. RTL layout support
3. Async search components
4. Number input component
5. Unit tests for components

---

## Next Steps

1. Create comprehensive gap analysis document
2. Create proposed solutions with code examples
3. Create prioritized roadmap
4. Create UX improvements plan
5. Create performance optimization strategy
6. Create QA plan
7. Create success criteria
8. Create developer quick start guide

---

_End of Scratchpad - Analysis Complete_
