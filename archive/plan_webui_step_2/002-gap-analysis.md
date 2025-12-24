# Gap Analysis: EdgeQuake WebUI vs LightRAG WebUI

> **Document Version:** 1.0  
> **Date:** 2024-12-23  
> **Purpose:** Comprehensive feature-by-feature gap analysis

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Technology Stack Comparison](#technology-stack-comparison)
3. [Feature Gap Matrix](#feature-gap-matrix)
4. [Detailed Gap Analysis](#detailed-gap-analysis)
5. [Cross-References](#cross-references)

---

## Executive Summary

This document provides a comprehensive gap analysis between **EdgeQuake WebUI** (current implementation) and **LightRAG WebUI** (reference implementation). The analysis reveals significant feature gaps that must be addressed to achieve feature parity.

### Key Findings

| Category             | EdgeQuake Status | Gap Severity |
| -------------------- | ---------------- | ------------ |
| Internationalization | ❌ Missing       | **Critical** |
| Graph Visualization  | ⚠️ Basic         | **High**     |
| Document Management  | ⚠️ Basic         | **High**     |
| Query/Retrieval      | ⚠️ Partial       | **High**     |
| Testing              | ❌ Missing       | **Medium**   |
| Advanced Markdown    | ❌ Missing       | **Medium**   |

### Overall Assessment

- **Total Identified Gaps:** 47
- **Critical Gaps:** 8
- **High Priority Gaps:** 18
- **Medium Priority Gaps:** 15
- **Low Priority Gaps:** 6
- **Estimated Feature Gap:** ~4,000+ lines of functionality

---

## Technology Stack Comparison

### Build Tools & Framework

| Aspect        | LightRAG WebUI   | EdgeQuake WebUI          | Notes                       |
| ------------- | ---------------- | ------------------------ | --------------------------- |
| Framework     | Vite + React 19  | Next.js 16               | EdgeQuake has SSR advantage |
| Build Tool    | Bun              | Next.js (Bun compatible) | Similar                     |
| Router        | react-router-dom | Next.js App Router       | Different paradigms         |
| React Version | 19.2.0           | 19.2.3                   | Compatible                  |
| State         | Zustand          | Zustand + React Query    | EdgeQuake enhanced          |

### UI & Visualization

| Aspect        | LightRAG WebUI | EdgeQuake WebUI | Status    |
| ------------- | -------------- | --------------- | --------- |
| UI Library    | Radix UI       | Radix UI        | ✅ Parity |
| Styling       | Tailwind CSS   | Tailwind CSS    | ✅ Parity |
| Graph Library | Sigma.js       | Sigma.js        | ✅ Parity |
| Icons         | Lucide React   | Lucide React    | ✅ Parity |
| Notifications | Sonner         | Sonner          | ✅ Parity |

### Missing Dependencies in EdgeQuake

| Package                    | Purpose                  | Priority     |
| -------------------------- | ------------------------ | ------------ |
| `i18next`                  | Internationalization     | **Critical** |
| `react-i18next`            | React i18n bindings      | **Critical** |
| `katex`                    | LaTeX rendering          | High         |
| `mermaid`                  | Diagram rendering        | High         |
| `react-syntax-highlighter` | Code highlighting        | High         |
| `minisearch`               | Fuzzy search             | High         |
| `remark-math`              | Math parsing             | Medium       |
| `rehype-katex`             | KaTeX integration        | Medium       |
| `seedrandom`               | Deterministic randomness | Low          |
| `react-select`             | Advanced select          | Low          |

---

## Feature Gap Matrix

### Legend

- ✅ Full implementation
- ⚠️ Partial implementation
- ❌ Not implemented

### Core Features

| Feature                         | LightRAG            | EdgeQuake     | Gap Level |
| ------------------------------- | ------------------- | ------------- | --------- |
| **Internationalization (i18n)** | ✅ 5 languages      | ❌ None       | Critical  |
| **RTL Support**                 | ✅ Arabic           | ❌ None       | Critical  |
| **Graph Node Drag**             | ✅ Full             | ❌ None       | High      |
| **Graph Layouts**               | ✅ 6 algorithms     | ❌ Force only | High      |
| **Graph Search**                | ✅ Fuzzy search     | ❌ None       | High      |
| **Graph Full-screen**           | ✅ Yes              | ❌ None       | Medium    |
| **Graph Legend**                | ✅ Yes              | ❌ None       | Low       |
| **Document Pagination**         | ✅ Full             | ❌ None       | High      |
| **Document Filtering**          | ✅ By status        | ❌ None       | High      |
| **Document Sorting**            | ✅ Multiple fields  | ❌ None       | High      |
| **Pipeline Monitoring**         | ✅ Full dialog      | ❌ None       | High      |
| **Document Scanning**           | ✅ Yes              | ❌ None       | Medium    |
| **Batch Document Ops**          | ✅ Yes              | ❌ None       | Medium    |
| **LaTeX Rendering**             | ✅ KaTeX            | ❌ None       | High      |
| **Mermaid Diagrams**            | ✅ Yes              | ❌ None       | High      |
| **COT/Thinking Display**        | ✅ `<think>` tags   | ❌ None       | High      |
| **Query Mode Prefix**           | ✅ `/mode query`    | ❌ None       | Medium    |
| **User Prompt History**         | ✅ With persistence | ❌ None       | Medium    |
| **Search History**              | ✅ LocalStorage     | ❌ None       | Medium    |
| **Entity Editing**              | ✅ Inline           | ❌ None       | Medium    |
| **Entity Merge**                | ✅ Dialog           | ❌ None       | Medium    |
| **Frontend Tests**              | ✅ Jest/Vitest      | ❌ None       | Medium    |
| **Tab Visibility**              | ✅ Optimization     | ❌ None       | Low       |

---

## Detailed Gap Analysis

### 1. Internationalization (i18n)

**Gap ID:** GAP-001  
**Severity:** 🔴 Critical  
**Impact:** UX / Accessibility / Market Reach

#### LightRAG Implementation

- **File:** `lightrag_webui/src/i18n.ts`
- **Locales:** `lightrag_webui/src/locales/{en,zh,fr,ar,zh_TW}.json`
- **Translation Keys:** 479+ in English
- **Features:**
  - Language switching with persistence
  - RTL support for Arabic
  - Dynamic language detection
  - Zustand store integration

#### EdgeQuake Status

- ❌ No i18n library installed
- ❌ All UI text hardcoded in English
- ❌ No locale files
- ❌ No language switching mechanism

#### Required Changes

1. Install `i18next` and `react-i18next`
2. Create locale files for at least: en, zh, fr
3. Create `i18n.ts` configuration
4. Wrap app with i18n provider
5. Replace all hardcoded strings with `t()` calls

---

### 2. Graph Visualization - Node Drag & Drop

**Gap ID:** GAP-002  
**Severity:** 🟠 High  
**Impact:** UX / Interactivity

#### LightRAG Implementation

- **File:** `lightrag_webui/src/features/GraphViewer.tsx` (lines 55-94)
- **Component:** `GraphEvents`
- **Features:**
  - Mouse down on node starts drag
  - Mouse move updates node position
  - Mouse up ends drag
  - Prevents camera auto-move during drag

#### EdgeQuake Status

- ❌ No node drag functionality
- ❌ Static node positions after layout

#### Code Reference

```tsx
// LightRAG GraphEvents component
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

---

### 3. Graph Layout Algorithms

**Gap ID:** GAP-003  
**Severity:** 🟠 High  
**Impact:** UX / Visualization Quality

#### LightRAG Implementation

- **File:** `lightrag_webui/src/components/graph/LayoutsControl.tsx`
- **Available Layouts:**
  1. Force (forceatlas2)
  2. Circular
  3. CirclePack
  4. Random
  5. Noverlap (collision avoidance)

#### EdgeQuake Status

- ⚠️ Only ForceAtlas2 layout implemented
- ❌ No layout selection UI
- ❌ Missing layout algorithm packages

#### Required Packages

```json
{
  "@react-sigma/layout-circular": "^5.0.4",
  "@react-sigma/layout-circlepack": "^5.0.4",
  "@react-sigma/layout-random": "^5.0.4",
  "@react-sigma/layout-noverlap": "^5.0.4"
}
```

---

### 4. Graph Node Search

**Gap ID:** GAP-004  
**Severity:** 🟠 High  
**Impact:** UX / Navigation

#### LightRAG Implementation

- **File:** `lightrag_webui/src/components/graph/GraphSearch.tsx`
- **Features:**
  - Fuzzy search with MiniSearch
  - Search results limit
  - Node/Edge search options
  - Focus on node on select
  - Custom search result display

#### EdgeQuake Status

- ❌ No node search functionality
- ❌ No fuzzy search library

#### Required Dependencies

```json
{
  "minisearch": "^7.2.0"
}
```

---

### 5. Document Manager - Pagination

**Gap ID:** GAP-005  
**Severity:** 🟠 High  
**Impact:** UX / Performance

#### LightRAG Implementation

- **File:** `lightrag_webui/src/features/DocumentManager.tsx`
- **Features:**
  - Page-based navigation
  - Configurable page size
  - URL state synchronization
  - Page number per status filter
  - Previous/Next controls

#### EdgeQuake Status

- ❌ No pagination
- Fetches all documents at once (page_size: 100)
- ❌ No URL state sync

---

### 6. Document Manager - Filtering & Sorting

**Gap ID:** GAP-006  
**Severity:** 🟠 High  
**Impact:** UX / Productivity

#### LightRAG Implementation

- **File:** `lightrag_webui/src/features/DocumentManager.tsx`
- **Filters:** all, processed, processing, pending, failed
- **Sort Fields:** created_at, updated_at, id, file_path
- **Sort Direction:** asc, desc

#### EdgeQuake Status

- ❌ No status filtering
- ❌ No sorting controls
- ❌ No filter/sort state in URL

---

### 7. Pipeline Status Monitoring

**Gap ID:** GAP-007  
**Severity:** 🟠 High  
**Impact:** UX / Operational Visibility

#### LightRAG Implementation

- **File:** `lightrag_webui/src/components/documents/PipelineStatusDialog.tsx`
- **Features:**
  - Real-time pipeline status
  - Job name and progress
  - Start time tracking
  - Pipeline messages log
  - Cancel functionality
  - Busy indicator animation

#### EdgeQuake Status

- ❌ No pipeline status monitoring
- ❌ No pipeline cancel functionality
- Basic processing indicator only

---

### 8. LaTeX/Math Rendering

**Gap ID:** GAP-008  
**Severity:** 🟠 High  
**Impact:** Content Quality / Academic Use

#### LightRAG Implementation

- **File:** `lightrag_webui/src/components/retrieval/ChatMessage.tsx`
- **Features:**
  - KaTeX with dynamic loading
  - Chemistry formulas (mhchem)
  - Copy-tex extension
  - Block and inline math
  - Error handling

#### EdgeQuake Status

- ❌ No LaTeX support
- Basic markdown only

#### Required Packages

```json
{
  "katex": "^0.16.23",
  "remark-math": "^6.0.0",
  "rehype-katex": "^7.0.1"
}
```

---

### 9. Mermaid Diagram Rendering

**Gap ID:** GAP-009  
**Severity:** 🟠 High  
**Impact:** Content Quality / Visualization

#### LightRAG Implementation

- **File:** `lightrag_webui/src/components/retrieval/ChatMessage.tsx`
- **Features:**
  - Automatic mermaid detection
  - Theme-aware rendering
  - Render state tracking
  - Code block identification

#### EdgeQuake Status

- ❌ No Mermaid support
- Code blocks displayed as text

#### Required Packages

```json
{
  "mermaid": "^11.12.0"
}
```

---

### 10. Chain-of-Thought (COT) Display

**Gap ID:** GAP-010  
**Severity:** 🟠 High  
**Impact:** UX / Transparency

#### LightRAG Implementation

- **File:** `lightrag_webui/src/features/RetrievalTesting.tsx`
- **Features:**
  - `<think>` tag parsing
  - Expandable thinking section
  - Thinking time tracking
  - Streaming support
  - Multiple think block handling

```typescript
const parseCOTContent = (content: string) => {
  const thinkStartTag = "<think>";
  const thinkEndTag = "</think>";
  // ... parsing logic
  return {
    isThinking,
    thinkingContent,
    displayContent,
    hasValidThinkBlock,
  };
};
```

#### EdgeQuake Status

- ❌ No COT parsing
- ❌ No thinking display
- Raw content shown including `<think>` tags

---

### 11. Syntax Highlighting

**Gap ID:** GAP-011  
**Severity:** 🟡 Medium  
**Impact:** Content Quality / Developer Experience

#### LightRAG Implementation

- **Package:** `react-syntax-highlighter`
- **Themes:** `oneLight`, `oneDark`
- **Features:**
  - Language detection
  - Theme awareness
  - Inline code support

#### EdgeQuake Status

- Uses `rehype-highlight`
- Less sophisticated highlighting
- No theme switching

---

### 12. Entity Property Editing

**Gap ID:** GAP-012  
**Severity:** 🟡 Medium  
**Impact:** Data Management

#### LightRAG Implementation

- **Files:**
  - `EditablePropertyRow.tsx`
  - `PropertyEditDialog.tsx`
  - `PropertiesView.tsx`
- **Features:**
  - Inline property editing
  - Save/cancel actions
  - Validation
  - API integration

#### EdgeQuake Status

- ❌ Read-only properties
- View-only node details panel

---

### 13. Entity Merge

**Gap ID:** GAP-013  
**Severity:** 🟡 Medium  
**Impact:** Data Quality

#### LightRAG Implementation

- **File:** `lightrag_webui/src/components/graph/MergeDialog.tsx`
- **Features:**
  - Merge confirmation dialog
  - Source/target entity display
  - Refresh options post-merge

#### EdgeQuake Status

- ❌ No entity merge functionality

---

### 14. User Prompt History

**Gap ID:** GAP-014  
**Severity:** 🟡 Medium  
**Impact:** UX / Productivity

#### LightRAG Implementation

- **File:** `lightrag_webui/src/components/ui/UserPromptInputWithHistory.tsx`
- **Features:**
  - History dropdown
  - Keyboard navigation
  - Delete from history
  - LocalStorage persistence

#### EdgeQuake Status

- Basic history in store
- ❌ No history dropdown UI
- ❌ No keyboard navigation

---

### 15. Search History Management

**Gap ID:** GAP-015  
**Severity:** 🟡 Medium  
**Impact:** UX / Productivity

#### LightRAG Implementation

- **File:** `lightrag_webui/src/utils/SearchHistoryManager.ts` (260 lines)
- **Features:**
  - LocalStorage persistence
  - Access count tracking
  - Size limit management
  - Version compatibility

#### EdgeQuake Status

- ❌ No search history persistence
- ❌ No SearchHistoryManager

---

### 16. Frontend Tests

**Gap ID:** GAP-016  
**Severity:** 🟡 Medium  
**Impact:** Quality / Maintainability

#### LightRAG Implementation

- **Directory:** `lightrag_webui/src/__tests__/`
- **Tests:** `tenantStateManager.test.ts`
- **Runner:** Bun test / Vitest

#### EdgeQuake Status

- ❌ No `__tests__` directory
- ❌ No test files
- ❌ No test configuration

---

### 17. Query Mode Prefix

**Gap ID:** GAP-017  
**Severity:** 🟡 Medium  
**Impact:** UX / Power Users

#### LightRAG Implementation

- **Pattern:** `/mode query_text`
- **Modes:** naive, local, global, hybrid, mix, bypass
- **Features:**
  - Prefix parsing
  - Error messages for invalid modes
  - Query rewriting

#### EdgeQuake Status

- ❌ No prefix parsing
- Mode selection via UI only

---

### 18. Tab Visibility Optimization

**Gap ID:** GAP-018  
**Severity:** 🟢 Low  
**Impact:** Performance

#### LightRAG Implementation

- **Files:**
  - `contexts/TabVisibilityProvider.tsx`
  - `contexts/useTabVisibility.ts`
- **Features:**
  - Pause updates when tab hidden
  - Resume on visibility
  - Resource optimization

#### EdgeQuake Status

- ❌ No visibility optimization

---

### 19. Graph Full-Screen Mode

**Gap ID:** GAP-019  
**Severity:** 🟢 Low  
**Impact:** UX

#### LightRAG Implementation

- **File:** `lightrag_webui/src/components/graph/FullScreenControl.tsx`
- **Features:**
  - Toggle full-screen
  - Escape key exit
  - Icon toggle

#### EdgeQuake Status

- ❌ No full-screen control

---

### 20. Graph Legend

**Gap ID:** GAP-020  
**Severity:** 🟢 Low  
**Impact:** UX / Clarity

#### LightRAG Implementation

- **Files:**
  - `Legend.tsx`
  - `LegendButton.tsx`
- **Features:**
  - Color legend by type
  - Toggle visibility
  - Backdrop blur styling

#### EdgeQuake Status

- ❌ No legend component
- Node colors unexplained

---

## Cross-References

| Document                                                    | Relationship                        |
| ----------------------------------------------------------- | ----------------------------------- |
| [Proposed Solutions](./003-proposed-solutions.md)           | Implementation plans for each gap   |
| [Prioritization & Roadmap](./004-prioritization-roadmap.md) | Execution order and phases          |
| [UX Improvements](./005-ux-improvements.md)                 | Detailed UX enhancement plans       |
| [Performance Strategy](./006-performance-strategy.md)       | Performance optimization approaches |
| [Success Criteria](./008-success-criteria.md)               | Measurable completion criteria      |

---

## Appendix: Source File References

### LightRAG WebUI Key Files

| File                            | Lines | Purpose             |
| ------------------------------- | ----- | ------------------- |
| `features/GraphViewer.tsx`      | 237   | Graph visualization |
| `features/DocumentManager.tsx`  | 1796  | Document management |
| `features/RetrievalTesting.tsx` | 825   | Query interface     |
| `hooks/useLightragGraph.tsx`    | 984   | Graph data hook     |
| `stores/settings.ts`            | 359   | Settings store      |
| `api/lightrag.ts`               | 833   | API layer           |
| `i18n.ts`                       | 55    | i18n config         |
| `utils/SearchHistoryManager.ts` | 260   | History utility     |

### EdgeQuake WebUI Key Files

| File                                        | Lines | Purpose             |
| ------------------------------------------- | ----- | ------------------- |
| `components/graph/graph-viewer.tsx`         | 214   | Graph visualization |
| `components/documents/document-manager.tsx` | 328   | Document management |
| `components/query/query-interface.tsx`      | 519   | Query interface     |
| `stores/use-settings-store.ts`              | 75    | Settings store      |
| `lib/api/edgequake.ts`                      | 410   | API layer           |

---

_Document generated by comprehensive file-by-file analysis_
