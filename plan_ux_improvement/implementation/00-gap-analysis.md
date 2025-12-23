# Gap Analysis: UX Improvement Plan vs Current Implementation

## Overview

This document maps the gap between the UX improvement plan requirements and the current codebase implementation. Each item is categorized by implementation status and linked to relevant source files.

---

## Status Legend

- ✅ **Implemented** - Feature is fully implemented
- 🟡 **Partial** - Feature exists but needs enhancement
- ❌ **Missing** - Feature needs to be implemented
- 🔧 **Fix Needed** - Feature has bugs or issues

---

## 1. Navigation & Layout ([01-navigation-layout.md](../01-navigation-layout.md))

### 1.1 Logo Navigation Target

| Requirement                       | Status     | Current State         | Source File                                                                    |
| --------------------------------- | ---------- | --------------------- | ------------------------------------------------------------------------------ |
| Logo links to home/dashboard page | ❌ Missing | Links to `/graph`     | [sidebar.tsx#L27](../../edgequake_webui/src/components/layout/sidebar.tsx#L27) |
| Home/dashboard page exists        | ❌ Missing | Redirects to `/graph` | [page.tsx#L4](<../../edgequake_webui/src/app/(dashboard)/page.tsx#L4>)         |

**Action Required:**

- Create a proper home/dashboard page at `/` route
- Update logo link to point to `/`
- Add dashboard with stats (document count, entity count, graph stats)

### 1.2 Sidebar Features

| Requirement             | Status         | Current State       | Source File                                                                            |
| ----------------------- | -------------- | ------------------- | -------------------------------------------------------------------------------------- |
| Sidebar navigation      | ✅ Implemented | 5 nav items working | [sidebar.tsx](../../edgequake_webui/src/components/layout/sidebar.tsx)                 |
| Active state styling    | ✅ Implemented | Using `bg-primary`  | [sidebar.tsx#L47-L52](../../edgequake_webui/src/components/layout/sidebar.tsx#L47-L52) |
| Sidebar collapse toggle | ❌ Missing     | Fixed width 256px   | [sidebar.tsx#L68](../../edgequake_webui/src/components/layout/sidebar.tsx#L68)         |
| Mobile hamburger menu   | ✅ Implemented | Sheet component     | [sidebar.tsx#L74-L88](../../edgequake_webui/src/components/layout/sidebar.tsx#L74-L88) |
| Version display         | ✅ Implemented | Footer shows v0.1.0 | [sidebar.tsx#L57-L61](../../edgequake_webui/src/components/layout/sidebar.tsx#L57-L61) |

**Action Required:**

- Add collapsible sidebar with icon-only mode
- Store collapse preference in localStorage

### 1.3 Header

| Requirement           | Status         | Current State        | Source File                                                                                  |
| --------------------- | -------------- | -------------------- | -------------------------------------------------------------------------------------------- |
| API status indicator  | ✅ Implemented | Green/red/yellow dot | [header.tsx#L31-L45](../../edgequake_webui/src/components/layout/header.tsx#L31-L45)         |
| Theme toggle          | ✅ Implemented | Light/Dark/System    | [header.tsx#L84-L109](../../edgequake_webui/src/components/layout/header.tsx#L84-L109)       |
| Language selector     | ✅ Implemented | en/zh/ja/ko          | [language-selector.tsx](../../edgequake_webui/src/components/shared/language-selector.tsx)   |
| User menu             | ✅ Implemented | Login/logout         | [header.tsx#L112-L142](../../edgequake_webui/src/components/layout/header.tsx#L112-L142)     |
| Breadcrumb navigation | ✅ Implemented | Dynamic breadcrumb   | [dynamic-breadcrumb.tsx](../../edgequake_webui/src/components/layout/dynamic-breadcrumb.tsx) |

### 1.4 Breadcrumb

| Requirement         | Status     | Current State | Source File                                                                                  |
| ------------------- | ---------- | ------------- | -------------------------------------------------------------------------------------------- |
| All items clickable | 🟡 Partial | Some disabled | [dynamic-breadcrumb.tsx](../../edgequake_webui/src/components/layout/dynamic-breadcrumb.tsx) |

---

## 2. Documents Page ([02-documents-page.md](../02-documents-page.md))

### 2.1 Upload Zone

| Requirement                | Status         | Current State         | Source File                                                                                                     |
| -------------------------- | -------------- | --------------------- | --------------------------------------------------------------------------------------------------------------- |
| Drag & drop upload         | ✅ Implemented | react-dropzone        | [document-manager.tsx#L351-L360](../../edgequake_webui/src/components/documents/document-manager.tsx#L351-L360) |
| Click to upload            | ✅ Implemented | Working               | [document-manager.tsx#L476-L497](../../edgequake_webui/src/components/documents/document-manager.tsx#L476-L497) |
| Max file size display      | ❌ Missing     | Only shows file types | [document-manager.tsx#L493-L495](../../edgequake_webui/src/components/documents/document-manager.tsx#L493-L495) |
| Upload progress (per-file) | ✅ Implemented | Phase indicators      | [document-manager.tsx#L125-L230](../../edgequake_webui/src/components/documents/document-manager.tsx#L125-L230) |
| Cancel upload button       | ❌ Missing     | No cancel option      | N/A                                                                                                             |
| Duplicate detection        | ✅ Implemented | Shows warning         | [document-manager.tsx#L183-L200](../../edgequake_webui/src/components/documents/document-manager.tsx#L183-L200) |

### 2.2 Document Table

| Requirement                         | Status         | Current State                       | Source File                                                                                                 |
| ----------------------------------- | -------------- | ----------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Document list table                 | ✅ Implemented | Full table                          | [document-manager.tsx](../../edgequake_webui/src/components/documents/document-manager.tsx)                 |
| Status badges                       | ✅ Implemented | pending/processing/completed/failed | [document-manager.tsx#L66-L75](../../edgequake_webui/src/components/documents/document-manager.tsx#L66-L75) |
| Entity count                        | ✅ Implemented | Shows count                         | Document table                                                                                              |
| Relative timestamps                 | ✅ Implemented | "3 minutes ago"                     | Using date-fns                                                                                              |
| Row actions (view/delete/reprocess) | ✅ Implemented | Dropdown menu                       | [document-manager.tsx](../../edgequake_webui/src/components/documents/document-manager.tsx)                 |
| Pagination                          | ✅ Implemented | Page controls                       | [pagination-controls.tsx](../../edgequake_webui/src/components/documents/pagination-controls.tsx)           |
| Status filter                       | ✅ Implemented | Dropdown filter                     | [document-filters.tsx](../../edgequake_webui/src/components/documents/document-filters.tsx)                 |
| Sort controls                       | ✅ Implemented | Created/Updated                     | [document-filters.tsx](../../edgequake_webui/src/components/documents/document-filters.tsx)                 |

### 2.3 Document Details

| Requirement                   | Status         | Current State                       | Source File                                                                                             |
| ----------------------------- | -------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Document detail drawer/dialog | ✅ Implemented | Dialog with tabs                    | [document-detail-dialog.tsx](../../edgequake_webui/src/components/documents/document-detail-dialog.tsx) |
| Content preview               | ✅ Implemented | Content tab                         | [document-detail-dialog.tsx](../../edgequake_webui/src/components/documents/document-detail-dialog.tsx) |
| Entities list                 | 🟡 Partial     | Tab exists but may need enhancement | [document-detail-dialog.tsx](../../edgequake_webui/src/components/documents/document-detail-dialog.tsx) |
| "View in Graph" button        | ❌ Missing     | Not linked to graph                 | N/A                                                                                                     |

### 2.4 Empty State

| Requirement         | Status     | Current State      | Source File                     |
| ------------------- | ---------- | ------------------ | ------------------------------- |
| Helpful empty state | 🟡 Partial | Basic message only | Needs illustration and guidance |

### 2.5 Clear All Confirmation

| Requirement                       | Status         | Current State | Source File                                                                                                     |
| --------------------------------- | -------------- | ------------- | --------------------------------------------------------------------------------------------------------------- |
| Confirmation dialog for Clear All | ✅ Implemented | AlertDialog   | [document-manager.tsx#L462-L474](../../edgequake_webui/src/components/documents/document-manager.tsx#L462-L474) |

---

## 3. Knowledge Graph Page ([03-knowledge-graph-page.md](../03-knowledge-graph-page.md))

### 3.1 Graph Visualization

| Requirement                | Status         | Current State         | Source File                                                                                     |
| -------------------------- | -------------- | --------------------- | ----------------------------------------------------------------------------------------------- |
| Force-directed layout      | ✅ Implemented | ForceAtlas2           | [graph-renderer.tsx](../../edgequake_webui/src/components/graph/graph-renderer.tsx)             |
| Zoom controls              | ✅ Implemented | +/-/reset buttons     | [zoom-controls.tsx](../../edgequake_webui/src/components/graph/zoom-controls.tsx)               |
| Node coloring by type      | ✅ Implemented | Type-based colors     | [node-details.tsx#L35-L43](../../edgequake_webui/src/components/graph/node-details.tsx#L35-L43) |
| Graph legend               | ✅ Implemented | Collapsible panel     | [graph-legend.tsx](../../edgequake_webui/src/components/graph/graph-legend.tsx)                 |
| Multiple layout algorithms | ✅ Implemented | Force/Circular/Random | [layout-control.tsx](../../edgequake_webui/src/components/graph/layout-control.tsx)             |

### 3.2 Node Interaction

| Requirement              | Status         | Current State     | Source File                                                                               |
| ------------------------ | -------------- | ----------------- | ----------------------------------------------------------------------------------------- |
| Node click shows details | ✅ Implemented | NodeDetails panel | [node-details.tsx](../../edgequake_webui/src/components/graph/node-details.tsx)           |
| Right-click context menu | ✅ Implemented | Node actions      | [node-context-menu.tsx](../../edgequake_webui/src/components/graph/node-context-menu.tsx) |
| Node drag & drop         | ✅ Implemented | Draggable         | [graph-events.tsx](../../edgequake_webui/src/components/graph/graph-events.tsx)           |

### 3.3 Search & Filter

| Requirement                | Status         | Current State     | Source File                                                                     |
| -------------------------- | -------------- | ----------------- | ------------------------------------------------------------------------------- |
| Node search                | ✅ Implemented | Search input      | [graph-search.tsx](../../edgequake_webui/src/components/graph/graph-search.tsx) |
| Type filter (legend click) | 🟡 Partial     | Needs toggle      | [graph-legend.tsx](../../edgequake_webui/src/components/graph/graph-legend.tsx) |
| Search autocomplete        | ❌ Missing     | Basic search only | N/A                                                                             |

### 3.4 Edge Labels

| Requirement                    | Status         | Current State   | Source File           |
| ------------------------------ | -------------- | --------------- | --------------------- |
| Edge labels on hover           | 🟡 Partial     | Setting exists  | Graph settings toggle |
| Always show edge labels option | ✅ Implemented | Settings toggle | Settings page         |

### 3.5 Empty State

| Requirement          | Status         | Current State | Source File                                                                                         |
| -------------------- | -------------- | ------------- | --------------------------------------------------------------------------------------------------- |
| Empty state with CTA | ✅ Implemented | Upload button | [graph-viewer.tsx#L202-L216](../../edgequake_webui/src/components/graph/graph-viewer.tsx#L202-L216) |

### 3.6 Graph Export

| Requirement       | Status     | Current State   | Source File |
| ----------------- | ---------- | --------------- | ----------- |
| Export as PNG/SVG | ❌ Missing | Not implemented | N/A         |
| Export as JSON    | ❌ Missing | Not implemented | N/A         |

---

## 4. Query Page ([04-query-page.md](../04-query-page.md))

### 4.1 Query Modes

| Requirement                                | Status         | Current State     | Source File                                                                                                   |
| ------------------------------------------ | -------------- | ----------------- | ------------------------------------------------------------------------------------------------------------- |
| Mode selector (Local/Global/Hybrid/Simple) | ✅ Implemented | Segmented control | [query-mode-selector.tsx](../../edgequake_webui/src/components/query/query-mode-selector.tsx)                 |
| Mode tooltips with explanations            | ✅ Implemented | Rich descriptions | [query-mode-selector.tsx#L18-L39](../../edgequake_webui/src/components/query/query-mode-selector.tsx#L18-L39) |

### 4.2 Chat Interface

| Requirement          | Status         | Current State          | Source File                                                                                             |
| -------------------- | -------------- | ---------------------- | ------------------------------------------------------------------------------------------------------- |
| Chat-style messages  | ✅ Implemented | User/Assistant bubbles | [query-interface.tsx](../../edgequake_webui/src/components/query/query-interface.tsx)                   |
| Streaming responses  | ✅ Implemented | Token by token         | [query-interface.tsx](../../edgequake_webui/src/components/query/query-interface.tsx)                   |
| Loading indicator    | ✅ Implemented | Animated brain         | [query-interface.tsx#L70-L118](../../edgequake_webui/src/components/query/query-interface.tsx#L70-L118) |
| Thinking/COT display | ✅ Implemented | Collapsible section    | [thinking-display.tsx](../../edgequake_webui/src/components/query/thinking-display.tsx)                 |

### 4.3 Source Attribution

| Requirement              | Status         | Current State             | Source File                                                                                               |
| ------------------------ | -------------- | ------------------------- | --------------------------------------------------------------------------------------------------------- |
| Source citations         | ✅ Implemented | SourceCitations component | [source-citations.tsx](../../edgequake_webui/src/components/query/source-citations.tsx)                   |
| Link to source documents | ✅ Implemented | Clickable links           | [source-citations.tsx](../../edgequake_webui/src/components/query/source-citations.tsx)                   |
| Link to graph entities   | ✅ Implemented | Entity navigation         | [query-interface.tsx#L345-L350](../../edgequake_webui/src/components/query/query-interface.tsx#L345-L350) |

### 4.4 Input Area

| Requirement           | Status         | Current State          | Source File                                                                           |
| --------------------- | -------------- | ---------------------- | ------------------------------------------------------------------------------------- |
| Auto-expand textarea  | 🟡 Partial     | Fixed height initially | [query-interface.tsx](../../edgequake_webui/src/components/query/query-interface.tsx) |
| Character/token count | ❌ Missing     | Not shown              | N/A                                                                                   |
| Keyboard hints        | ✅ Implemented | Shift+Enter hint       | [query-interface.tsx](../../edgequake_webui/src/components/query/query-interface.tsx) |

### 4.5 History

| Requirement                | Status         | Current State     | Source File                                                                           |
| -------------------------- | -------------- | ----------------- | ------------------------------------------------------------------------------------- |
| Query history sidebar      | ✅ Implemented | Sheet component   | [query-interface.tsx](../../edgequake_webui/src/components/query/query-interface.tsx) |
| Favorite queries           | ✅ Implemented | Star toggle       | [use-query-store.ts](../../edgequake_webui/src/stores/use-query-store.ts)             |
| Clear history              | ✅ Implemented | Clear button      | [query-interface.tsx](../../edgequake_webui/src/components/query/query-interface.tsx) |
| Separate favorites section | ❌ Missing     | Mixed with recent | N/A                                                                                   |
| Full text on hover         | ❌ Missing     | Truncated only    | N/A                                                                                   |

### 4.6 Response Actions

| Requirement         | Status         | Current State     | Source File                                                                                               |
| ------------------- | -------------- | ----------------- | --------------------------------------------------------------------------------------------------------- |
| Copy response       | ✅ Implemented | Copy button       | [query-interface.tsx#L162-L174](../../edgequake_webui/src/components/query/query-interface.tsx#L162-L174) |
| Regenerate response | ✅ Implemented | Refresh button    | [query-interface.tsx#L297-L311](../../edgequake_webui/src/components/query/query-interface.tsx#L297-L311) |
| Stop generation     | 🟡 Partial     | Needs enhancement | N/A                                                                                                       |

---

## 5. Settings Page ([05-settings-page.md](../05-settings-page.md))

### 5.1 Appearance

| Requirement             | Status         | Current State     | Source File                                                                                          |
| ----------------------- | -------------- | ----------------- | ---------------------------------------------------------------------------------------------------- |
| Theme selector          | ✅ Implemented | Light/Dark/System | [settings/page.tsx#L65-L94](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx#L65-L94>)   |
| Language selector       | ✅ Implemented | 4 languages       | [settings/page.tsx#L98-L117](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx#L98-L117>) |
| Smooth theme transition | ❌ Missing     | May flash         | N/A                                                                                                  |

### 5.2 Graph Settings

| Requirement             | Status         | Current State      | Source File                                                                                            |
| ----------------------- | -------------- | ------------------ | ------------------------------------------------------------------------------------------------------ |
| Show node labels toggle | ✅ Implemented | Switch             | [settings/page.tsx#L136](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx#L136>)           |
| Show edge labels toggle | ✅ Implemented | Switch             | [settings/page.tsx#L149](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx#L149>)           |
| Node size selector      | ✅ Implemented | Small/Medium/Large | [settings/page.tsx#L161-L178](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx#L161-L178>) |
| Settings preview        | ❌ Missing     | No preview graph   | N/A                                                                                                    |

### 5.3 Query Settings

| Requirement        | Status         | Current State | Source File    |
| ------------------ | -------------- | ------------- | -------------- |
| Default query mode | ✅ Implemented | Mode selector | Settings store |
| Streaming toggle   | ✅ Implemented | Switch        | Settings store |

### 5.4 Data Management

| Requirement          | Status         | Current State                     | Source File                                                                        |
| -------------------- | -------------- | --------------------------------- | ---------------------------------------------------------------------------------- |
| Clear query history  | ✅ Implemented | Button                            | [settings/page.tsx](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx>) |
| Reset settings       | ✅ Implemented | Button                            | [settings/page.tsx](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx>) |
| Confirmation dialogs | 🟡 Partial     | Reset has dialog, clear needs one | [settings/page.tsx](<../../edgequake_webui/src/app/(dashboard)/settings/page.tsx>) |

### 5.5 Save Confirmation

| Requirement              | Status     | Current State | Source File |
| ------------------------ | ---------- | ------------- | ----------- |
| Toast on settings change | ❌ Missing | Silent save   | N/A         |

### 5.6 Import/Export

| Requirement               | Status     | Current State   | Source File |
| ------------------------- | ---------- | --------------- | ----------- |
| Export settings as JSON   | ❌ Missing | Not implemented | N/A         |
| Import settings from JSON | ❌ Missing | Not implemented | N/A         |

---

## 6. Global Components ([06-global-components.md](../06-global-components.md))

### 6.1 Toasts

| Requirement          | Status         | Current State           | Source File    |
| -------------------- | -------------- | ----------------------- | -------------- |
| Success toasts       | ✅ Implemented | sonner                  | Throughout app |
| Error toasts         | ✅ Implemented | sonner                  | Throughout app |
| Warning toasts       | ✅ Implemented | sonner                  | Throughout app |
| Toast stacking       | 🟡 Partial     | Default sonner behavior | N/A            |
| Toast action buttons | ❌ Missing     | Passive only            | N/A            |

### 6.2 Loading States

| Requirement           | Status     | Current State     | Source File |
| --------------------- | ---------- | ----------------- | ----------- |
| Page skeleton loaders | ❌ Missing | Only spinner      | N/A         |
| Button loading state  | 🟡 Partial | Some buttons only | Various     |
| Top progress bar      | ❌ Missing | Not implemented   | N/A         |

### 6.3 Error Handling

| Requirement                  | Status         | Current State     | Source File                                                                                 |
| ---------------------------- | -------------- | ----------------- | ------------------------------------------------------------------------------------------- |
| API connection error display | ✅ Implemented | Alert component   | [document-manager.tsx](../../edgequake_webui/src/components/documents/document-manager.tsx) |
| Retry buttons                | ✅ Implemented | "Try again" links | Various                                                                                     |
| Form validation errors       | 🟡 Partial     | Basic validation  | N/A                                                                                         |

### 6.4 Accessibility

| Requirement          | Status         | Current State       | Source File |
| -------------------- | -------------- | ------------------- | ----------- |
| Focus indicators     | ✅ Implemented | focus-visible rings | Throughout  |
| Skip navigation link | ❌ Missing     | Not implemented     | N/A         |
| Screen reader labels | ✅ Implemented | aria-labels         | Throughout  |
| Keyboard navigation  | 🟡 Partial     | Basic tab support   | N/A         |

### 6.5 Responsive Design

| Requirement          | Status         | Current State          | Source File                                                                                 |
| -------------------- | -------------- | ---------------------- | ------------------------------------------------------------------------------------------- |
| Mobile sidebar       | ✅ Implemented | Sheet drawer           | [sidebar.tsx](../../edgequake_webui/src/components/layout/sidebar.tsx)                      |
| Table responsiveness | 🟡 Partial     | No card view on mobile | [document-manager.tsx](../../edgequake_webui/src/components/documents/document-manager.tsx) |
| Touch targets (44px) | 🟡 Partial     | Most buttons adequate  | N/A                                                                                         |

### 6.6 Localization

| Requirement            | Status         | Current State       | Source File                                    |
| ---------------------- | -------------- | ------------------- | ---------------------------------------------- |
| Translation coverage   | ✅ Implemented | en/zh/fr JSON files | [locales/](../../edgequake_webui/src/locales/) |
| RTL support            | ❌ Missing     | Not implemented     | N/A                                            |
| Date/number formatting | 🟡 Partial     | date-fns used       | Various                                        |

---

## 7. Upload Flow ([07-upload-flow.md](../07-upload-flow.md))

### 7.1 Progress Tracking

| Requirement              | Status         | Current State     | Source File                                                                                       |
| ------------------------ | -------------- | ----------------- | ------------------------------------------------------------------------------------------------- |
| Per-file progress        | ✅ Implemented | Phase indicators  | [document-manager.tsx](../../edgequake_webui/src/components/documents/document-manager.tsx)       |
| Batch progress card      | ✅ Implemented | BatchProgressCard | [batch-progress-card.tsx](../../edgequake_webui/src/components/documents/batch-progress-card.tsx) |
| Processing time estimate | ❌ Missing     | No ETA            | N/A                                                                                               |

### 7.2 Error Handling

| Requirement        | Status         | Current State      | Source File                                                                                 |
| ------------------ | -------------- | ------------------ | ------------------------------------------------------------------------------------------- |
| File error display | ✅ Implemented | Red icon + message | [document-manager.tsx](../../edgequake_webui/src/components/documents/document-manager.tsx) |
| Retry failed files | ❌ Missing     | Must re-upload     | N/A                                                                                         |

### 7.3 Validation

| Requirement                   | Status         | Current State        | Source File                                                                                                     |
| ----------------------------- | -------------- | -------------------- | --------------------------------------------------------------------------------------------------------------- |
| File type validation          | ✅ Implemented | accept prop          | [document-manager.tsx#L354-L359](../../edgequake_webui/src/components/documents/document-manager.tsx#L354-L359) |
| File size validation          | ❌ Missing     | No client-side check | N/A                                                                                                             |
| Immediate feedback on invalid | ❌ Missing     | Delayed error        | N/A                                                                                                             |

---

## Priority Summary

### P0 - Critical (Must Fix)

1. ❌ Create home/dashboard page
2. ❌ Update logo to link to home

### P1 - High Priority (Sprint 1)

1. ❌ Add save confirmation toasts for settings
2. ❌ Add max file size display and validation
3. 🟡 Improve empty states with illustrations
4. ❌ Add sidebar collapse functionality

### P2 - Medium Priority (Sprint 2)

1. ❌ Add graph export (PNG/SVG/JSON)
2. ❌ Separate favorites section in query history
3. ❌ Add search autocomplete in graph
4. ❌ Settings import/export
5. 🟡 Improve toast action buttons

### P3 - Low Priority (Sprint 3)

1. ❌ RTL language support
2. ❌ Skip navigation link
3. ❌ Mobile card view for tables
4. ❌ Processing time estimate for uploads

---

## Next Steps

See implementation plans:

- [Phase 1: Core Fixes](./01-phase1-core-fixes.md)
- [Phase 2: Graph & Query](./02-phase2-graph-query.md)
- [Phase 3: Polish & Accessibility](./03-phase3-polish.md)
