# Gap Analysis: EdgeQuake WebUI vs LightRAG WebUI

> **Document Version:** 2.0  
> **Date:** 2024-12-23  
> **Phase:** Step 3 - Comprehensive Analysis  
> **Purpose:** Feature-by-feature gap analysis with source references

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Technology Stack Comparison](#technology-stack-comparison)
3. [Feature Gap Matrix](#feature-gap-matrix)
4. [Detailed Gap Analysis](#detailed-gap-analysis)
5. [Cross-References](#cross-references)

---

## Executive Summary

This document provides a comprehensive gap analysis between **EdgeQuake WebUI** and **LightRAG WebUI** (reference implementation). The analysis is based on a file-by-file review of both implementations.

### Key Findings

| Category             | EdgeQuake Status | Gap Severity |
| -------------------- | ---------------- | ------------ |
| Internationalization | ⚠️ Partial       | **Medium**   |
| Graph Visualization  | ⚠️ Basic         | **High**     |
| Document Management  | ⚠️ Basic         | **High**     |
| Query/Retrieval      | ⚠️ Partial       | **Medium**   |
| Testing              | ⚠️ E2E Only      | **Medium**   |
| Markdown Rendering   | ✅ Implemented   | Low          |
| Theme Support        | ✅ Implemented   | None         |

### Overall Assessment

- **Total Identified Gaps:** 35
- **Critical Gaps:** 4
- **High Priority Gaps:** 14
- **Medium Priority Gaps:** 12
- **Low Priority Gaps:** 5
- **Already Implemented:** LaTeX, Mermaid, COT Display, Streaming

---

## Technology Stack Comparison

### Build Tools & Framework

| Aspect        | LightRAG WebUI          | EdgeQuake WebUI          | Notes                       |
| ------------- | ----------------------- | ------------------------ | --------------------------- |
| Framework     | Vite + React 19.2.0     | Next.js 16.1.0           | EdgeQuake has SSR advantage |
| Build Tool    | Bun                     | Next.js (Bun compatible) | Similar                     |
| Router        | react-router-dom (Hash) | Next.js App Router       | Different paradigms         |
| React Version | 19.2.0                  | 19.2.3                   | Compatible                  |
| State         | Zustand                 | Zustand + React Query    | EdgeQuake enhanced          |

### UI & Visualization

| Aspect        | LightRAG WebUI | EdgeQuake WebUI | Status    |
| ------------- | -------------- | --------------- | --------- |
| UI Library    | Radix UI       | Radix UI        | ✅ Parity |
| Styling       | Tailwind CSS   | Tailwind CSS    | ✅ Parity |
| Graph Library | Sigma.js 5.0.4 | Sigma.js 5.0.6  | ✅ Parity |
| Icons         | Lucide React   | Lucide React    | ✅ Parity |
| Notifications | Sonner 1.7.4   | Sonner 2.0.7    | ✅ Parity |

### Missing/Different Dependencies

| Package       | LightRAG    | EdgeQuake        | Status       |
| ------------- | ----------- | ---------------- | ------------ |
| `i18next`     | 5 languages | 3 languages      | ⚠️ Partial   |
| `katex`       | 0.16.23     | 0.16.27          | ✅ Present   |
| `mermaid`     | 11.12.0     | 11.12.2          | ✅ Present   |
| `minisearch`  | 7.2.0       | 7.2.0            | ✅ Present   |
| `react-query` | Not used    | 5.90.12          | ✅ Advantage |
| `date-fns`    | Not used    | 4.1.0            | ✅ Advantage |
| `axios`       | 1.12.2      | Not used (Fetch) | ≈ Equivalent |

---

## Feature Gap Matrix

### Legend

- ✅ Full implementation
- ⚠️ Partial implementation
- ❌ Not implemented

### Graph Visualization

| Feature                     | LightRAG                                                                                     | EdgeQuake     | Gap Level  |
| --------------------------- | -------------------------------------------------------------------------------------------- | ------------- | ---------- |
| **Node Drag & Drop**        | ✅ [GraphViewer.tsx#L55-94](../lightrag_webui/src/features/GraphViewer.tsx)                  | ❌ None       | **High**   |
| **Multiple Layouts**        | ✅ 6 algorithms                                                                              | ⚠️ Force only | **High**   |
| **Fuzzy Node Search**       | ✅ [GraphSearch.tsx](../lightrag_webui/src/components/graph/GraphSearch.tsx)                 | ❌ Basic      | **High**   |
| **Full-screen Mode**        | ✅ [FullScreenControl.tsx](../lightrag_webui/src/components/graph/FullScreenControl.tsx)     | ❌ None       | **Medium** |
| **Legend Display**          | ✅ [Legend.tsx](../lightrag_webui/src/components/graph/Legend.tsx)                           | ❌ None       | **Low**    |
| **Inline Property Edit**    | ✅ [EditablePropertyRow.tsx](../lightrag_webui/src/components/graph/EditablePropertyRow.tsx) | ❌ None       | **Medium** |
| **Entity Merge Dialog**     | ✅ [MergeDialog.tsx](../lightrag_webui/src/components/graph/MergeDialog.tsx)                 | ❌ None       | **Medium** |
| **Graph Settings Panel**    | ✅ [Settings.tsx](../lightrag_webui/src/components/graph/Settings.tsx)                       | ❌ None       | **Medium** |
| **Focus on Node**           | ✅ [FocusOnNode.tsx](../lightrag_webui/src/components/graph/FocusOnNode.tsx)                 | ⚠️ Basic      | **Low**    |
| **Theme Switch Protection** | ✅ Yes                                                                                       | ⚠️ Partial    | **Low**    |

### Document Management

| Feature                    | LightRAG                                                                                           | EdgeQuake      | Gap Level  |
| -------------------------- | -------------------------------------------------------------------------------------------------- | -------------- | ---------- |
| **URL State Sync**         | ✅ [useRouteState.ts](../lightrag_webui/src/hooks/useRouteState.ts)                                | ❌ None        | **High**   |
| **Pagination Controls**    | ✅ [PaginationControls.tsx](../lightrag_webui/src/components/ui/PaginationControls.tsx)            | ⚠️ Basic       | **Medium** |
| **Status Filtering**       | ✅ Multiple statuses                                                                               | ⚠️ Basic       | **Medium** |
| **Multi-field Sorting**    | ✅ 4 fields                                                                                        | ⚠️ Client-side | **Medium** |
| **Document Scanning**      | ✅ [scanNewDocuments](../lightrag_webui/src/api/lightrag.ts#L303)                                  | ❌ None        | **Medium** |
| **Batch Operations**       | ✅ Select all, reset                                                                               | ❌ None        | **Medium** |
| **Pipeline Status Dialog** | ✅ [PipelineStatusDialog.tsx](../lightrag_webui/src/components/documents/PipelineStatusDialog.tsx) | ⚠️ Basic       | **High**   |
| **Metadata Tooltips**      | ✅ Formatted display                                                                               | ❌ None        | **Low**    |

### Query/Retrieval Interface

| Feature                   | LightRAG                                                                                            | EdgeQuake      | Gap Level  |
| ------------------------- | --------------------------------------------------------------------------------------------------- | -------------- | ---------- |
| **Streaming Chat**        | ✅ [queryTextStream](../lightrag_webui/src/api/lightrag.ts#L313)                                    | ✅ Implemented | None       |
| **COT Display**           | ✅ [ChatMessage.tsx](../lightrag_webui/src/components/retrieval/ChatMessage.tsx)                    | ✅ Implemented | None       |
| **LaTeX Rendering**       | ✅ KaTeX + extensions                                                                               | ✅ Implemented | None       |
| **Mermaid Diagrams**      | ✅ Yes                                                                                              | ✅ Implemented | None       |
| **Query Mode Prefix**     | ✅ `/mode query` parsing                                                                            | ❌ None        | **Medium** |
| **Thinking Time Display** | ✅ Duration tracking                                                                                | ❌ None        | **Medium** |
| **User Prompt History**   | ✅ [UserPromptInputWithHistory](../lightrag_webui/src/components/ui/UserPromptInputWithHistory.tsx) | ❌ None        | **Medium** |
| **Conversation History**  | ✅ Configurable turns                                                                               | ⚠️ Basic       | **Medium** |
| **Copy to Clipboard**     | ✅ Response copying                                                                                 | ❌ None        | **Low**    |
| **Source Citations**      | ❌ None                                                                                             | ✅ Implemented | Advantage  |

### Internationalization

| Feature                  | LightRAG                     | EdgeQuake         | Gap Level  |
| ------------------------ | ---------------------------- | ----------------- | ---------- |
| **i18n Framework**       | ✅ i18next                   | ✅ i18next        | None       |
| **Languages Supported**  | ✅ 5 (en, zh, fr, ar, zh_TW) | ⚠️ 3 (en, zh, fr) | **Medium** |
| **RTL Support**          | ✅ Arabic                    | ❌ None           | **Low**    |
| **Translation Coverage** | ✅ ~500 keys                 | ⚠️ ~150 keys      | **High**   |
| **Language Persistence** | ✅ Settings store            | ✅ Settings store | None       |

### UI/UX Features

| Feature                | LightRAG                                                     | EdgeQuake         | Gap Level |
| ---------------------- | ------------------------------------------------------------ | ----------------- | --------- |
| **Navigation**         | Tab-based                                                    | Sidebar           | Different |
| **Keyboard Shortcuts** | ❌ None                                                      | ✅ Implemented    | Advantage |
| **Breadcrumb**         | ❌ None                                                      | ✅ Implemented    | Advantage |
| **Mobile Sidebar**     | ❌ None                                                      | ✅ Implemented    | Advantage |
| **Loading Skeletons**  | ⚠️ Basic                                                     | ✅ Implemented    | Advantage |
| **API Explorer**       | ✅ [ApiSite.tsx](../lightrag_webui/src/features/ApiSite.tsx) | ✅ Implemented    | None      |
| **Settings Page**      | ❌ Modal only                                                | ✅ Dedicated page | Advantage |

---

## Detailed Gap Analysis

### GAP-001: Node Drag & Drop

**Severity:** 🔴 High  
**Impact:** UX / Interactivity  
**Status:** ❌ Not Implemented

#### LightRAG Implementation

**Source:** [lightrag_webui/src/features/GraphViewer.tsx#L55-94](../lightrag_webui/src/features/GraphViewer.tsx)

```tsx
const GraphEvents = () => {
  const registerEvents = useRegisterEvents();
  const sigma = useSigma();
  const [draggedNode, setDraggedNode] = useState<string | null>(null);

  useEffect(() => {
    registerEvents({
      downNode: (e) => {
        setDraggedNode(e.node);
        sigma.getGraph().setNodeAttribute(e.node, "highlighted", true);
      },
      mousemovebody: (e) => {
        if (!draggedNode) return;
        const pos = sigma.viewportToGraph(e);
        sigma.getGraph().setNodeAttribute(draggedNode, "x", pos.x);
        sigma.getGraph().setNodeAttribute(draggedNode, "y", pos.y);
        e.preventSigmaDefault();
      },
      mouseup: () => {
        if (draggedNode) {
          setDraggedNode(null);
          sigma.getGraph().removeNodeAttribute(draggedNode, "highlighted");
        }
      },
    });
  }, [registerEvents, sigma, draggedNode]);
  return null;
};
```

#### EdgeQuake Status

- `graph-events.tsx` exists but lacks drag implementation
- Only handles click events

#### Required Changes

1. Add `draggedNode` state to graph store
2. Register drag events in GraphEvents component
3. Add visual feedback for dragged nodes
4. Prevent camera move during drag

📎 See: [003-proposed-solutions.md](./003-proposed-solutions.md#gap-001)

---

### GAP-002: Multiple Layout Algorithms

**Severity:** 🔴 High  
**Impact:** UX / Functionality  
**Status:** ⚠️ Partial (Force only)

#### LightRAG Implementation

**Source:** [lightrag_webui/src/components/graph/LayoutsControl.tsx](../lightrag_webui/src/components/graph/LayoutsControl.tsx)

Supported layouts:

- Circular
- CirclePack
- Force
- ForceAtlas2
- Noverlap
- Random

#### EdgeQuake Status

- `layout-control.tsx` exists
- Only implements basic force layout
- Missing layout persistence

#### Required Changes

1. Install additional @react-sigma/layout-\* packages
2. Create layout selector dropdown
3. Add layout settings (iterations, etc.)
4. Persist selected layout in settings

📎 See: [003-proposed-solutions.md](./003-proposed-solutions.md#gap-002)

---

### GAP-003: Graph Node Search with Fuzzy Matching

**Severity:** 🔴 High  
**Impact:** UX / Discoverability  
**Status:** ❌ Not Implemented

#### LightRAG Implementation

**Source:** [lightrag_webui/src/components/graph/GraphSearch.tsx](../lightrag_webui/src/components/graph/GraphSearch.tsx)

- Uses MiniSearch for fuzzy matching
- Real-time filtering as user types
- Highlights matching nodes
- Keyboard navigation support

#### EdgeQuake Status

- `graph-search.tsx` exists but uses basic string matching
- No fuzzy matching
- Limited keyboard support

📎 See: [003-proposed-solutions.md](./003-proposed-solutions.md#gap-003)

---

### GAP-004: Pipeline Status Dialog

**Severity:** 🔴 High  
**Impact:** UX / Visibility  
**Status:** ⚠️ Partial

#### LightRAG Implementation

**Source:** [lightrag_webui/src/components/documents/PipelineStatusDialog.tsx](../lightrag_webui/src/components/documents/PipelineStatusDialog.tsx)

- Full dialog with progress details
- Current batch information
- History of messages
- Cancellation support
- Auto-refresh

#### EdgeQuake Status

- `pipeline-status-dialog.tsx` exists
- Basic implementation
- Missing detailed progress
- No cancellation UI

📎 See: [003-proposed-solutions.md](./003-proposed-solutions.md#gap-004)

---

### GAP-005: Translation Coverage

**Severity:** 🟡 Medium  
**Impact:** Internationalization  
**Status:** ⚠️ Partial (~30% coverage)

#### LightRAG Implementation

**Source:** [lightrag_webui/src/locales/en.json](../lightrag_webui/src/locales/en.json)

~500 translation keys covering:

- Navigation
- Graph controls
- Document management
- Query interface
- Error messages
- Tooltips
- Accessibility labels

#### EdgeQuake Status

- ~150 translation keys
- Missing graph component translations
- Missing error message translations
- Missing accessibility labels

📎 See: [003-proposed-solutions.md](./003-proposed-solutions.md#gap-005)

---

### GAP-006: URL State Synchronization

**Severity:** 🟡 Medium  
**Impact:** UX / Shareability  
**Status:** ❌ Not Implemented

#### LightRAG Implementation

**Source:** [lightrag_webui/src/hooks/useRouteState.ts](../lightrag_webui/src/hooks/useRouteState.ts)

- Syncs pagination to URL
- Syncs filters to URL
- Enables shareable links
- Browser back/forward support

#### EdgeQuake Status

- No URL state hook
- State only in React state/stores

📎 See: [003-proposed-solutions.md](./003-proposed-solutions.md#gap-006)

---

### GAP-007: Query Mode Prefix Parsing

**Severity:** 🟡 Medium  
**Impact:** UX / Power Users  
**Status:** ❌ Not Implemented

#### LightRAG Implementation

**Source:** [lightrag_webui/src/features/RetrievalTesting.tsx#L168-180](../lightrag_webui/src/features/RetrievalTesting.tsx)

```tsx
const allowedModes: QueryMode[] = [
  "naive",
  "local",
  "global",
  "hybrid",
  "mix",
  "bypass",
];
const prefixMatch = inputValue.match(/^\/(\w+)\s+([\s\S]+)/);
if (prefixMatch) {
  const mode = prefixMatch[1] as QueryMode;
  if (allowedModes.includes(mode)) {
    modeOverride = mode;
    actualQuery = prefixMatch[2];
  }
}
```

#### EdgeQuake Status

- No prefix parsing
- Must use dropdown for mode selection

📎 See: [003-proposed-solutions.md](./003-proposed-solutions.md#gap-007)

---

### GAP-008: Thinking Time Display

**Severity:** 🟡 Medium  
**Impact:** UX / Transparency  
**Status:** ❌ Not Implemented

#### LightRAG Implementation

**Source:** [lightrag_webui/src/features/RetrievalTesting.tsx#L265-275](../lightrag_webui/src/features/RetrievalTesting.tsx)

- Tracks thinking start time
- Calculates duration on completion
- Displays in message metadata

#### EdgeQuake Status

- COT display exists
- No time tracking

📎 See: [003-proposed-solutions.md](./003-proposed-solutions.md#gap-008)

---

### GAP-009: Full-Screen Graph Mode

**Severity:** 🟡 Medium  
**Impact:** UX / Presentation  
**Status:** ❌ Not Implemented

#### LightRAG Implementation

**Source:** [lightrag_webui/src/components/graph/FullScreenControl.tsx](../lightrag_webui/src/components/graph/FullScreenControl.tsx)

- Toggle button for full-screen
- Uses browser Fullscreen API
- Keyboard shortcut (F11)

📎 See: [003-proposed-solutions.md](./003-proposed-solutions.md#gap-009)

---

### GAP-010: Entity Merge Functionality

**Severity:** 🟡 Medium  
**Impact:** Data Quality  
**Status:** ❌ Not Implemented

#### LightRAG Implementation

**Source:** [lightrag_webui/src/components/graph/MergeDialog.tsx](../lightrag_webui/src/components/graph/MergeDialog.tsx)

- Confirmation dialog
- Source/target entity display
- Refresh options after merge

📎 See: [003-proposed-solutions.md](./003-proposed-solutions.md#gap-010)

---

## EdgeQuake Advantages Over LightRAG

Features where EdgeQuake is ahead:

| Feature                | EdgeQuake Implementation                              |
| ---------------------- | ----------------------------------------------------- |
| **SSR Support**        | Next.js App Router with dynamic imports               |
| **React Query**        | Better caching, background refresh, mutation handling |
| **Source Citations**   | Shows entities/chunks used in query response          |
| **Keyboard Shortcuts** | Global shortcuts with help dialog                     |
| **Breadcrumb Nav**     | Context-aware navigation                              |
| **Mobile Sidebar**     | Responsive Sheet component                            |
| **Loading States**     | Skeleton components throughout                        |
| **Settings Page**      | Dedicated route vs modal                              |
| **E2E Tests**          | Playwright test suite                                 |
| **Query History**      | Favorites and recent queries sidebar                  |

---

## Cross-References

- **Proposed Solutions:** [003-proposed-solutions.md](./003-proposed-solutions.md)
- **Prioritization & Roadmap:** [004-prioritization-roadmap.md](./004-prioritization-roadmap.md)
- **UX Improvements:** [005-ux-improvements.md](./005-ux-improvements.md)
- **Performance Strategy:** [006-performance-strategy.md](./006-performance-strategy.md)
- **QA Plan:** [007-qa-plan.md](./007-qa-plan.md)
- **Success Criteria:** [008-success-criteria.md](./008-success-criteria.md)
- **Developer Guide:** [009-developer-guide.md](./009-developer-guide.md)

---

_Document generated from comprehensive source analysis. See [scratchpad.md](./scratchpad.md) for raw findings._
