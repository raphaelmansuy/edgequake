# Scratchpad: EdgeQuake WebUI vs LightRAG WebUI Analysis

> **Note:** This is an append-only document capturing raw findings during analysis.

---

## Analysis Session: 2024-12-23

### Framework Comparison

#### LightRAG WebUI

- **Framework:** Vite + React 19
- **Router:** react-router-dom
- **State:** Zustand
- **Build:** Bun
- **UI:** Radix UI + Tailwind CSS
- **Graph:** Sigma.js (@react-sigma/core)
- **i18n:** i18next with 5 languages (en, zh, fr, ar, zh_TW)

#### EdgeQuake WebUI

- **Framework:** Next.js 16 (App Router)
- **Router:** Next.js built-in routing
- **State:** Zustand + React Query
- **Build:** Next.js (can use Bun)
- **UI:** Radix UI + Tailwind CSS
- **Graph:** Sigma.js (@react-sigma/core)
- **i18n:** Not implemented

---

### Directory Structure Comparison

#### LightRAG WebUI src/

```
- App.tsx
- AppRouter.tsx
- __tests__/
- api/
- components/
- contexts/
- features/
- hooks/
- i18n.ts
- locales/
- lib/
- services/
- stores/
- types/
- utils/
```

#### EdgeQuake WebUI src/

```
- app/ (Next.js App Router)
- components/
- hooks/
- lib/
- providers/
- stores/
- types/
```

**Missing in EdgeQuake:**

- `__tests__/` - No test directory
- `locales/` - No internationalization
- `i18n.ts` - No i18n setup
- `services/` - No dedicated services layer
- `contexts/` - No React contexts
- `utils/` - No utility functions directory

---

### Feature-by-Feature Gap Analysis

#### 1. Internationalization (i18n)

**LightRAG:** Full i18n support with 5 languages

- File: `src/i18n.ts`
- Locales: `src/locales/{en,zh,fr,ar,zh_TW}.json`
- 479+ translation keys in English alone
- RTL support (Arabic)

**EdgeQuake:** ❌ NO i18n

- All text hardcoded in English
- Impact: Cannot support international users

---

#### 2. Graph Viewer

**LightRAG Features:**

- `GraphViewer.tsx` (237 lines)
- Node drag-and-drop with GraphEvents
- Multiple layout controls (circular, force, forceatlas2, noverlap, random)
- Full-screen control
- Node search bar with MiniSearch
- Legend display
- Settings panel
- Zoom controls
- Properties panel (editable)
- Theme-aware rendering
- Loading overlay with spinner
- Graph labels display

**EdgeQuake Features:**

- `graph-viewer.tsx` (214 lines)
- Basic zoom controls
- Node selection
- Node context menu
- Graph filters sidebar
- Node details panel
- Empty state with upload prompt
- Refresh functionality

**Missing in EdgeQuake:**

- ❌ Node drag-and-drop
- ❌ Multiple layout algorithms
- ❌ Full-screen mode
- ❌ Node search with fuzzy matching
- ❌ Legend display
- ❌ Advanced graph settings panel
- ❌ Theme switching protection
- ❌ Graph labels toggle

---

#### 3. Graph Components (LightRAG)

Files in `lightrag_webui/src/components/graph/`:

1. `EditablePropertyRow.tsx` - Inline property editing
2. `FocusOnNode.tsx` - Camera focus on selected node
3. `FullScreenControl.tsx` - Full-screen toggle
4. `GraphControl.tsx` - Main graph controller
5. `GraphLabels.tsx` - Label display settings
6. `GraphSearch.tsx` - Fuzzy search with MiniSearch
7. `LayoutsControl.tsx` - Layout algorithm selector
8. `Legend.tsx` - Color legend
9. `LegendButton.tsx` - Toggle legend
10. `MergeDialog.tsx` - Entity merge confirmation
11. `PropertiesView.tsx` - Property panel (editable)
12. `PropertyEditDialog.tsx` - Property edit modal
13. `PropertyRowComponents.tsx` - Property row UI
14. `Settings.tsx` - Graph settings
15. `SettingsDisplay.tsx` - Settings indicator
16. `ZoomControl.tsx` - Zoom buttons

**EdgeQuake has:**

1. `graph-controls.tsx`
2. `graph-filters.tsx`
3. `graph-renderer.tsx`
4. `graph-viewer.tsx`
5. `node-context-menu.tsx`
6. `node-details.tsx`

**Missing:** 10+ specialized graph components

---

#### 4. Document Manager

**LightRAG Features:**

- `DocumentManager.tsx` (1796 lines - very comprehensive)
- Pagination with URL sync
- Status filtering (all, processed, processing, pending, failed)
- Sorting (created_at, updated_at, id, file_path)
- Pipeline status monitoring
- Document scanning
- Batch operations
- Reset document status
- File name display toggle
- Metadata tooltips with formatting
- Progress animation for busy pipeline
- Select all / deselect
- Multi-tenant support

**EdgeQuake Features:**

- `document-manager.tsx` (328 lines)
- Basic upload with drag-and-drop
- Simple table display
- Delete single/all
- Reprocess single
- Status badges
- Auto-refresh polling

**Missing in EdgeQuake:**

- ❌ Pagination controls
- ❌ URL state sync
- ❌ Status filtering
- ❌ Sorting controls
- ❌ Pipeline status dialog
- ❌ Batch select/delete
- ❌ Document scanning
- ❌ Reset document status
- ❌ Metadata display

---

#### 5. Retrieval/Query Interface

**LightRAG Features:**

- `RetrievalTesting.tsx` (825 lines)
- Streaming responses with COT (Chain-of-Thought) parsing
- `<think>` tag processing for reasoning display
- Thinking time tracking
- Query mode prefix (e.g., `/naive query`)
- Conversation history support
- User prompt history
- LaTeX rendering with KaTeX
- Mermaid diagram support
- Syntax highlighting
- Copy to clipboard
- Multiple query modes
- Query settings panel
- Throttled updates

**EdgeQuake Features:**

- `query-interface.tsx` (519 lines)
- Streaming support
- Basic message display
- Query mode selector
- Settings sheet
- History sidebar
- Favorites
- Temperature/TopK sliders

**Missing in EdgeQuake:**

- ❌ COT/thinking display with `<think>` tags
- ❌ Thinking time tracking
- ❌ Query mode prefix parsing
- ❌ LaTeX/KaTeX rendering
- ❌ Mermaid diagram rendering
- ❌ Syntax highlighting (Prism)
- ❌ User prompt templates
- ❌ Advanced streaming with error recovery

---

#### 6. Chat Message Rendering

**LightRAG:**

- `ChatMessage.tsx` (515 lines)
- Dynamic KaTeX loading with extensions
- Mermaid diagram rendering
- Thinking content expansion
- Syntax highlighting with theme awareness
- Multiple markdown plugins
- Footnote support
- Error state styling

**EdgeQuake:**

- Uses basic ReactMarkdown with rehype-highlight
- No KaTeX
- No Mermaid
- No thinking display
- Basic syntax highlighting

---

#### 7. UI Components Comparison

**LightRAG has (that EdgeQuake is missing):**

- `AsyncSearch.tsx` - Async search dropdown
- `AsyncSelect.tsx` - Async select component
- `DataTable.tsx` - Data table with features
- `EmptyCard.tsx` - Empty state card
- `FileUploader.tsx` - Advanced file upload
- `NumberInput.tsx` - Number input component
- `PaginationControls.tsx` - Pagination UI
- `Progress.tsx` - Progress bar
- `TabContent.tsx` - Tab content wrapper
- `Text.tsx` - Text component
- `UserPromptInputWithHistory.tsx` - Input with history

---

#### 8. API Layer

**LightRAG:**

- `api/client.ts` - Axios instance with interceptors
- `api/lightrag.ts` - 800+ lines of typed API functions
- `api/tenant.ts` - Tenant-specific APIs
- Proper error handling
- Typed responses

**EdgeQuake:**

- `lib/api/client.ts` - Axios wrapper
- `lib/api/edgequake.ts` - 410 lines of API functions
- Stream client for SSE
- Similar structure but fewer endpoints

---

#### 9. State Management

**LightRAG Stores:**

- `stores/graph.ts` - Graph state with selectors
- `stores/settings.ts` - App settings (359 lines)
- `stores/state.ts` - Backend state
- `stores/tenant.ts` - Tenant state

**EdgeQuake Stores:**

- `stores/use-auth-store.ts` - Auth state
- `stores/use-backend-store.ts` - Backend state
- `stores/use-graph-store.ts` - Graph state
- `stores/use-query-store.ts` - Query state
- `stores/use-settings-store.ts` - Settings
- `stores/use-tenant-store.ts` - Tenant state

EdgeQuake has similar coverage but less feature-rich settings.

---

#### 10. Testing

**LightRAG:**

- `__tests__/tenantStateManager.test.ts`
- Test infrastructure exists

**EdgeQuake:**

- ❌ No tests in webui

---

#### 11. Services Layer

**LightRAG:**

- `services/debounce.ts` - Debounce utility
- `services/navigation.ts` - Navigation service
- `services/tenantStateManager.ts` - Tenant state management

**EdgeQuake:**

- ❌ No services layer

---

#### 12. Hooks

**LightRAG Hooks:**

- `useDebounce.tsx`
- `useLightragGraph.tsx` (984 lines - very complex)
- `useRandomGraph.tsx`
- `useRouteState.ts`
- `useTenantContext.ts`
- `useTenantInitialization.ts`
- `useTheme.tsx`

**EdgeQuake Hooks:**

- `use-keyboard-shortcuts.ts`

**Missing:** 6 hooks

---

#### 13. Utility Functions

**LightRAG Utils:**

- `utils/SearchHistoryManager.ts` (260 lines)
- `utils/clipboard.ts`
- `utils/graphColor.ts`
- `utils/remarkFootnotes.ts`

**EdgeQuake:**

- `lib/utils.ts` (basic cn function)
- Missing utilities

---

#### 14. Contexts

**LightRAG:**

- `contexts/TabVisibilityProvider.tsx`
- `contexts/context.ts`
- `contexts/types.ts`
- `contexts/useTabVisibility.ts`

**EdgeQuake:**

- ❌ No contexts directory

---

#### 15. Features (Page Components)

**LightRAG:**

- `features/ApiSite.tsx` - API documentation site
- `features/DocumentManager.tsx` - Document management
- `features/GraphViewer.tsx` - Graph visualization
- `features/LoginPage.tsx` - Login page
- `features/RetrievalTesting.tsx` - Query interface
- `features/SiteHeader.tsx` - Header component
- `features/TenantSelectionPage.tsx` - Tenant selection

**EdgeQuake:**

- Uses Next.js app router pages instead
- Similar coverage but different architecture

---

### Status Components (LightRAG)

- `StatusCard.tsx` - Status display card
- `StatusDialog.tsx` - Status details dialog
- `StatusIndicator.tsx` - Status dot indicator

EdgeQuake has status in header but less detailed.

---

### Tenant/Multi-tenant Features

**LightRAG:**

- `TenantSelector.tsx` - Tenant dropdown
- `useTenantContext.ts` - Tenant context hook
- `tenantStateManager.ts` - Tenant state service
- URL-based tenant routing

**EdgeQuake:**

- `use-tenant-store.ts` - Basic tenant store
- Less sophisticated tenant handling

---

### Theme Support

**LightRAG:**

- `ThemeProvider.tsx` - Theme context
- `ThemeToggle.tsx` - Theme switch button
- `useTheme.tsx` - Theme hook
- Proper dark/light theme support

**EdgeQuake:**

- `theme-provider.tsx` - Using next-themes
- Theme support exists but less integrated

---

### Documents Dialogs (LightRAG)

- `ClearDocumentsDialog.tsx` - Clear all confirmation
- `DeleteDocumentsDialog.tsx` - Delete selected
- `PipelineStatusDialog.tsx` - Pipeline monitoring
- `UploadDocumentsDialog.tsx` - Upload modal

EdgeQuake has basic AlertDialog for delete but missing:

- ❌ Pipeline status monitoring
- ❌ Advanced upload dialog
- ❌ Delete with options (delete files, clear cache)

---

### Query Settings (LightRAG)

- `QuerySettings.tsx` - Full settings panel
- Includes: mode, top_k, chunk_top_k, max_entity_tokens, max_relation_tokens, max_total_tokens, stream, history_turns, user_prompt, enable_rerank

EdgeQuake QuerySettings in sheet is simpler.

---

## Summary of Critical Gaps

### High Priority

1. **Internationalization** - No i18n support
2. **Advanced Graph Features** - Missing layouts, search, drag
3. **Document Pagination** - No pagination or filtering
4. **LaTeX/Mermaid Rendering** - Not supported
5. **COT/Thinking Display** - Not implemented
6. **Pipeline Monitoring** - No pipeline status

### Medium Priority

7. **Tests** - No frontend tests
8. **Graph Entity Editing** - Not supported
9. **Search History** - No persistence
10. **User Prompt History** - Not implemented
11. **Entity Merge** - Not supported

### Lower Priority

12. **Tab Visibility Optimization** - Not implemented
13. **Advanced Status Indicators** - Basic only
14. **Services Layer** - No abstraction

---

## File Reference Links

### LightRAG Source Files

- Graph: `lightrag_webui/src/features/GraphViewer.tsx`
- Documents: `lightrag_webui/src/features/DocumentManager.tsx`
- Retrieval: `lightrag_webui/src/features/RetrievalTesting.tsx`
- i18n: `lightrag_webui/src/i18n.ts`
- Settings: `lightrag_webui/src/stores/settings.ts`
- API: `lightrag_webui/src/api/lightrag.ts`

---

## Architecture Notes

### LightRAG Advantages

1. Clean feature-based organization
2. Comprehensive i18n from the start
3. Advanced graph visualization
4. Rich markdown rendering
5. URL state synchronization
6. Services layer for business logic

### EdgeQuake Advantages

1. Next.js SSR capabilities
2. React Query for data fetching
3. More modern Next.js 16 App Router
4. Better folder structure for Next.js

### Recommendation

Keep Next.js architecture but port LightRAG features:

1. Add i18next for internationalization
2. Port graph components for feature parity
3. Add pagination to document manager
4. Implement KaTeX and Mermaid for query
5. Add pipeline status monitoring
6. Create test suite

---

## Appendix: Line Count Comparison

| Feature          | LightRAG | EdgeQuake | Delta |
| ---------------- | -------- | --------- | ----- |
| GraphViewer      | 237      | 214       | -23   |
| DocumentManager  | 1796     | 328       | -1468 |
| RetrievalTesting | 825      | 519       | -306  |
| ChatMessage      | 515      | N/A       | -515  |
| Settings Store   | 359      | 75        | -284  |
| API Layer        | 833      | 410       | -423  |
| Graph Hook       | 984      | N/A       | -984  |

Total estimated feature gap: ~4,000+ lines of functionality

---

_End of Analysis Session: 2024-12-23_
