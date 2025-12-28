# WebUI Specification: Architecture Overview

> Document ID: WEBUI-001
> Version: 1.0
> Created: 2024-12-28
> Status: SPECIFICATION

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Design Principles](#2-design-principles)
3. [Architecture Overview](#3-architecture-overview)
4. [Data Flow Architecture](#4-data-flow-architecture)
5. [Component Hierarchy](#5-component-hierarchy)
6. [State Management](#6-state-management)
7. [Technology Stack](#7-technology-stack)

---

## 1. Executive Summary

This document defines the architecture for upgrading the EdgeQuake WebUI to leverage the new SOTA ingestion pipeline features. The update introduces real-time progress tracking, comprehensive lineage visualization, cost monitoring, and enhanced document management capabilities.

### 1.1 Key Objectives

| ID    | Objective                                  | Priority |
| ----- | ------------------------------------------ | -------- |
| O-001 | Real-time ingestion progress via WebSocket | P0       |
| O-002 | Interactive lineage visualization          | P0       |
| O-003 | Cost tracking and budget alerts            | P0       |
| O-004 | Enhanced document management UI            | P1       |
| O-005 | Stage-level progress indicators            | P0       |
| O-006 | Entity provenance drill-down               | P1       |
| O-007 | Ingestion job cancellation                 | P1       |
| O-008 | Dark/Light mode consistency                | P2       |

### 1.2 Design Tokens Reference

The UI follows existing design tokens from `design-tokens.css`:

- Primary colors: Blue accent for actions
- Success: Green for completed states
- Warning: Amber for pending/processing
- Error: Red for failures
- Background: Neutral gray scale

---

## 2. Design Principles

### 2.1 Core UX Principles

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          DESIGN PRINCIPLES                                  │
└─────────────────────────────────────────────────────────────────────────────┘

  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
  │   SLICK         │  │  INFORMATIVE    │  │  ACTIONABLE     │
  │   ───────────   │  │  ───────────    │  │  ───────────    │
  │ Minimal clutter │  │ Clear status    │  │ Every metric    │
  │ Clean hierarchy │  │ Real-time data  │  │ drives action   │
  │ Focused tasks   │  │ Progressive     │  │ Quick access    │
  │                 │  │ disclosure      │  │ to next step    │
  └─────────────────┘  └─────────────────┘  └─────────────────┘

  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
  │   RESPONSIVE    │  │  ACCESSIBLE     │  │  CONSISTENT     │
  │   ───────────   │  │  ───────────    │  │  ───────────    │
  │ Mobile-first    │  │ WCAG 2.1 AA    │  │ Unified tokens  │
  │ Adaptive layout │  │ Keyboard nav    │  │ Pattern library │
  │ Touch-friendly  │  │ Screen readers  │  │ Dark/Light      │
  └─────────────────┘  └─────────────────┘  └─────────────────┘
```

### 2.2 Information Hierarchy

| Level            | Content                                  | Interaction     |
| ---------------- | ---------------------------------------- | --------------- |
| **L1 Overview**  | Document counts, active jobs, total cost | Dashboard view  |
| **L2 List**      | Document table with status badges        | Click to select |
| **L3 Detail**    | Document metadata, extraction stats      | Side panel      |
| **L4 Deep Dive** | Chunk-level lineage, entity provenance   | Modal/page      |

### 2.3 Color Semantics

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          COLOR SEMANTICS                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐ │
│  │ 🟢 SUCCESS   │   │ 🟡 PENDING   │   │ 🔵 PROCESSING│   │ 🔴 ERROR     │ │
│  │              │   │              │   │              │   │              │ │
│  │ Completed    │   │ Queued       │   │ Extracting   │   │ Failed       │ │
│  │ Indexed      │   │ Waiting      │   │ Embedding    │   │ Cancelled    │ │
│  │ Ready        │   │ Scheduled    │   │ Merging      │   │ Timeout      │ │
│  │              │   │              │   │              │   │              │ │
│  │ green-500    │   │ yellow-500   │   │ blue-500     │   │ red-500      │ │
│  └──────────────┘   └──────────────┘   └──────────────┘   └──────────────┘ │
│                                                                             │
│  ┌──────────────┐   ┌──────────────┐                                       │
│  │ 🟣 INFO      │   │ ⚪ NEUTRAL   │                                       │
│  │              │   │              │                                       │
│  │ Metadata     │   │ Default      │                                       │
│  │ Statistics   │   │ Borders      │                                       │
│  │ Hints        │   │ Dividers     │                                       │
│  │              │   │              │                                       │
│  │ purple-500   │   │ gray-500     │                                       │
│  └──────────────┘   └──────────────┘                                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Architecture Overview

### 3.1 High-Level System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         WEBUI ARCHITECTURE                                  │
└─────────────────────────────────────────────────────────────────────────────┘

                              ┌─────────────────┐
                              │     User        │
                              │    Browser      │
                              └────────┬────────┘
                                       │
                         ┌─────────────┴─────────────┐
                         │                           │
              ┌──────────┴──────────┐     ┌─────────┴─────────┐
              │   Next.js App       │     │   WebSocket       │
              │   (HTTP/REST)       │     │   (Real-time)     │
              └──────────┬──────────┘     └─────────┬─────────┘
                         │                           │
                         │    ┌─────────────────┐    │
                         └────┤  React Query    ├────┘
                              │  (Cache Layer)  │
                              └────────┬────────┘
                                       │
              ┌────────────────────────┼────────────────────────┐
              │                        │                        │
              ▼                        ▼                        ▼
    ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
    │  Zustand Store   │    │   UI Components  │    │  Hooks Library   │
    │  (Global State)  │    │   (Presentation) │    │  (Data Fetching) │
    └──────────────────┘    └──────────────────┘    └──────────────────┘
              │                        │                        │
              └────────────────────────┼────────────────────────┘
                                       │
                              ┌────────┴────────┐
                              │  EdgeQuake API  │
                              │  (Rust Backend) │
                              └─────────────────┘
```

### 3.2 Feature Module Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         FEATURE MODULES                                     │
└─────────────────────────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────────────────────┐
│                              APP SHELL                                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │  Sidebar    │  │  Header     │  │  Content    │  │  Right Panel│      │
│  │  Navigation │  │  Breadcrumb │  │  Area       │  │  (Details)  │      │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘      │
└───────────────────────────────────────────────────────────────────────────┘

┌─────────────────────┐ ┌─────────────────────┐ ┌─────────────────────┐
│  DOCUMENTS MODULE   │ │   GRAPH MODULE      │ │   QUERY MODULE      │
│  ─────────────────  │ │  ─────────────────  │ │  ─────────────────  │
│ ┌─────────────────┐ │ │ ┌─────────────────┐ │ │ ┌─────────────────┐ │
│ │ Document List   │ │ │ │ Graph Canvas    │ │ │ │ Query Input     │ │
│ │ + Upload Zone   │ │ │ │ + Node Details  │ │ │ │ + Response      │ │
│ └─────────────────┘ │ │ └─────────────────┘ │ │ └─────────────────┘ │
│ ┌─────────────────┐ │ │ ┌─────────────────┐ │ │ ┌─────────────────┐ │
│ │ Progress Panel  │ │ │ │ Entity Panel    │ │ │ │ Context Panel   │ │
│ │ + WebSocket     │ │ │ │ + Lineage Link  │ │ │ │ + Citations     │ │
│ └─────────────────┘ │ │ └─────────────────┘ │ │ └─────────────────┘ │
│ ┌─────────────────┐ │ │ ┌─────────────────┐ │ │ ┌─────────────────┐ │
│ │ Lineage View    │ │ │ │ Relationship    │ │ │ │ Conversation    │ │
│ │ + Cost Display  │ │ │ │ Explorer        │ │ │ │ History         │ │
│ └─────────────────┘ │ │ └─────────────────┘ │ │ └─────────────────┘ │
└─────────────────────┘ └─────────────────────┘ └─────────────────────┘

┌─────────────────────┐ ┌─────────────────────┐
│  SETTINGS MODULE    │ │   ADMIN MODULE      │
│  ─────────────────  │ │  ─────────────────  │
│ ┌─────────────────┐ │ │ ┌─────────────────┐ │
│ │ LLM Config      │ │ │ │ Cost Dashboard  │ │
│ │ + Model Select  │ │ │ │ + Usage Charts  │ │
│ └─────────────────┘ │ │ └─────────────────┘ │
│ ┌─────────────────┐ │ │ ┌─────────────────┐ │
│ │ Pipeline Config │ │ │ │ Tenant/Workspace│ │
│ │ + Chunk Size    │ │ │ │ Management      │ │
│ └─────────────────┘ │ │ └─────────────────┘ │
└─────────────────────┘ └─────────────────────┘
```

---

## 4. Data Flow Architecture

### 4.1 Document Ingestion Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    DOCUMENT INGESTION DATA FLOW                             │
└─────────────────────────────────────────────────────────────────────────────┘

User Action                  Frontend                       Backend
───────────────────────────────────────────────────────────────────────────────

1. Upload File        ──►  POST /api/v1/documents   ──►  Create Document
                                     │                         │
                                     ▼                         ▼
                           Response: track_id        Queue for Processing
                                     │
                                     ▼
2. Connect WebSocket  ──►  WS /api/v1/ws/progress  ──►  Subscribe to Track
                                     │                         │
                                     │                         ▼
3. Receive Updates    ◄──  Progress Events         ◄──  Pipeline Stages
                           {stage, %, message}               │
                                     │                         │
                                     ▼                         │
4. Update UI          ──►  Local State Update                 │
   - Progress bar                    │                         │
   - Stage indicator                 │                         │
   - Cost display                    │                         ▼
                                     │              Complete/Error
5. Completion         ◄──  Completion Event        ◄──────────┘
                           {result, cost}
                                     │
                                     ▼
6. Fetch Details      ──►  GET /api/v1/documents/{id}  ──►  Document Data
                           GET /api/v1/documents/{id}/lineage ──► Lineage
```

### 4.2 WebSocket Event Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      WEBSOCKET EVENT SEQUENCE                               │
└─────────────────────────────────────────────────────────────────────────────┘

Client                              Server
───────────────────────────────────────────────────────────────────────────────

1. Connect & Auth
   ──►  { type: "auth", token: "..." }
   ◄──  { type: "auth_ok" }

2. Subscribe to Ingestion
   ──►  { type: "subscribe", channel: "ingestion", track_id: "track_xyz" }
   ◄──  { type: "subscribed", channel: "ingestion", track_id: "track_xyz" }

3. Progress Updates (repeating)
   ◄──  { type: "progress", track_id: "track_xyz",
         stage: "chunking", completion_percentage: 15.5,
         message: "Creating chunks..." }
   ◄──  { type: "progress", track_id: "track_xyz",
         stage: "extracting", completion_percentage: 45.0,
         message: "Extracting entities from chunk 5/10" }
   ◄──  { type: "cost_update", track_id: "track_xyz",
         cost: { total_usd: 0.0025, tokens: { input: 5000, output: 1000 } } }

4. Stage Completion
   ◄──  { type: "stage_completed", track_id: "track_xyz",
         stage: "extracting", result: { entities: 25, relationships: 15 } }

5. Job Completion
   ◄──  { type: "completed", track_id: "track_xyz",
         document_id: "doc_abc", result: {...}, cost: {...} }

6. Unsubscribe (optional)
   ──►  { type: "unsubscribe", channel: "ingestion", track_id: "track_xyz" }
```

---

## 5. Component Hierarchy

### 5.1 Documents Module Components

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     DOCUMENTS MODULE COMPONENT TREE                         │
└─────────────────────────────────────────────────────────────────────────────┘

DocumentsPage
├── DocumentPageHeader
│   ├── PageTitle
│   ├── DocumentFilters
│   │   ├── StatusFilter (dropdown)
│   │   ├── DateRangeFilter
│   │   └── SearchInput
│   └── ActionButtons
│       ├── UploadButton
│       ├── ScanButton
│       └── ReprocessFailedButton
│
├── DocumentUploadZone (NEW - enhanced)
│   ├── DropZone
│   ├── UploadProgressList
│   │   └── UploadProgressItem (per file)
│   │       ├── FileInfo
│   │       ├── ProgressBar
│   │       └── StatusBadge
│   └── BatchProgressCard (WebSocket)
│
├── DocumentTable
│   ├── TableHeader (sortable)
│   └── TableBody
│       └── DocumentRow (per doc)
│           ├── SelectCheckbox
│           ├── NameCell (+ icon)
│           ├── StatusCell (badge + animation)
│           ├── StatsCell (entities/relationships)
│           ├── CostCell (NEW - $0.00)
│           ├── DateCell
│           └── ActionsCell
│               ├── ViewButton → RightPanel
│               ├── ReprocessButton
│               └── DeleteButton
│
├── PaginationControls
│
└── DocumentDetailPanel (RightPanel)
    ├── DocumentHeader
    │   ├── Title
    │   ├── StatusBadge
    │   └── QuickActions
    ├── DocumentTabs
    │   ├── OverviewTab
    │   │   ├── KeyStats (grid)
    │   │   ├── ContentPreview
    │   │   └── MetadataSidebar
    │   ├── LineageTab (NEW - enhanced)
    │   │   ├── LineageTree (interactive)
    │   │   ├── ChunkExplorer (NEW)
    │   │   └── ExtractionDetails
    │   ├── EntitiesTab
    │   │   ├── EntityList
    │   │   └── EntityCard (expandable)
    │   ├── CostTab (NEW)
    │   │   ├── CostBreakdownChart
    │   │   ├── TokenUsageTable
    │   │   └── ModelInfo
    │   └── ErrorTab (conditional)
    │       ├── ErrorDetails
    │       ├── RetryButton
    │       └── SupportInfo
    └── DocumentFooter
        └── TimestampInfo
```

### 5.2 New Components Specification

| Component                 | Type          | Purpose                    | Status |
| ------------------------- | ------------- | -------------------------- | ------ |
| `RealTimeProgress`        | Container     | WebSocket progress display | NEW    |
| `IngestionStageIndicator` | Presentation  | Stage visualization        | NEW    |
| `CostBadge`               | Presentation  | $ cost display             | NEW    |
| `CostBreakdownChart`      | Visualization | Cost pie/bar chart         | NEW    |
| `ChunkExplorer`           | Interactive   | Browse chunks → entities   | NEW    |
| `EntityProvenance`        | Detail        | Entity source tracking     | NEW    |
| `LineageGraph`            | Visualization | Interactive lineage viz    | NEW    |
| `WebSocketStatus`         | Indicator     | Connection status          | NEW    |

---

## 6. State Management

### 6.1 State Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       STATE MANAGEMENT LAYERS                               │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│  LAYER 1: SERVER STATE (React Query / TanStack Query)                       │
│  ═══════════════════════════════════════════════════                        │
│  • Documents list, entities, relationships, graph data                      │
│  • Cached with configurable stale time                                      │
│  • Auto-refetch on window focus                                             │
│  • Query keys: ['documents', tenantId, workspaceId, page, filters]         │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  LAYER 2: GLOBAL APP STATE (Zustand)                                        │
│  ════════════════════════════════════                                       │
│  • Auth state (user, tokens)                                                │
│  • Tenant/Workspace selection                                               │
│  • UI preferences (theme, sidebar state)                                    │
│  • Active WebSocket connections                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  LAYER 3: REAL-TIME STATE (WebSocket + Local State)                         │
│  ═══════════════════════════════════════════════════                        │
│  • Active ingestion progress (per track_id)                                 │
│  • Live cost updates                                                        │
│  • Stage completion events                                                  │
│  • Connection status                                                        │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  LAYER 4: COMPONENT LOCAL STATE (useState/useReducer)                       │
│  ═════════════════════════════════════════════════════                      │
│  • Form inputs, expanded/collapsed states                                   │
│  • Selection states, modal visibility                                       │
│  • Pagination local state                                                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 6.2 New Zustand Stores

```typescript
// src/stores/use-ingestion-store.ts (NEW)
interface IngestionStore {
  // Active ingestion jobs by track_id
  activeJobs: Map<string, IngestionProgress>;

  // WebSocket connection state
  wsConnected: boolean;
  wsReconnecting: boolean;

  // Actions
  addJob: (trackId: string, initial: IngestionProgress) => void;
  updateProgress: (trackId: string, update: ProgressUpdate) => void;
  completeJob: (trackId: string, result: IngestionResult) => void;
  failJob: (trackId: string, error: IngestionError) => void;
  removeJob: (trackId: string) => void;
  setWsConnected: (connected: boolean) => void;
}

// src/stores/use-cost-store.ts (NEW)
interface CostStore {
  // Cost data by document/workspace
  documentCosts: Map<string, DocumentCost>;
  workspaceSummary: WorkspaceCostSummary | null;

  // Budget alerts
  budgetAlerts: BudgetAlert[];

  // Actions
  updateDocumentCost: (docId: string, cost: DocumentCost) => void;
  setWorkspaceSummary: (summary: WorkspaceCostSummary) => void;
  addBudgetAlert: (alert: BudgetAlert) => void;
  clearAlerts: () => void;
}
```

---

## 7. Technology Stack

### 7.1 Frontend Stack

| Category      | Technology       | Version | Purpose                       |
| ------------- | ---------------- | ------- | ----------------------------- |
| Framework     | Next.js          | 14.x    | App Router, Server Components |
| UI Library    | React            | 19.x    | Component model               |
| Styling       | Tailwind CSS     | 3.x     | Utility-first styling         |
| Components    | shadcn/ui        | latest  | UI component library          |
| State         | Zustand          | 4.x     | Global state                  |
| Data Fetching | TanStack Query   | 5.x     | Server state management       |
| WebSocket     | Native WebSocket | -       | Real-time updates             |
| Charts        | Recharts         | 2.x     | Cost visualization            |
| Icons         | Lucide React     | latest  | Icon library                  |
| Forms         | React Hook Form  | 7.x     | Form handling                 |
| Validation    | Zod              | 3.x     | Schema validation             |
| i18n          | i18next          | 23.x    | Internationalization          |

### 7.2 Build & Development

| Tool       | Purpose         |
| ---------- | --------------- |
| pnpm       | Package manager |
| TypeScript | Type safety     |
| ESLint     | Linting         |
| Playwright | E2E testing     |
| Vitest     | Unit testing    |

---

## Appendix A: File Structure

```
edgequake_webui/src/
├── app/
│   └── (dashboard)/
│       └── documents/
│           ├── page.tsx          # Documents list page
│           └── [id]/
│               └── page.tsx      # Document detail page (NEW)
├── components/
│   ├── documents/
│   │   ├── document-manager.tsx  # UPDATE
│   │   ├── batch-progress-card.tsx # UPDATE
│   │   ├── ingestion-progress.tsx  # NEW
│   │   ├── cost-badge.tsx          # NEW
│   │   └── chunk-explorer.tsx      # NEW
│   ├── document/
│   │   ├── lineage-tree.tsx      # UPDATE
│   │   ├── lineage-graph.tsx       # NEW
│   │   └── entity-provenance.tsx   # NEW
│   └── shared/
│       ├── websocket-status.tsx    # NEW
│       └── cost-breakdown-chart.tsx # NEW
├── hooks/
│   ├── use-websocket.ts            # NEW
│   ├── use-ingestion-progress.ts   # NEW
│   └── use-cost-tracking.ts        # NEW
├── lib/
│   └── api/
│       ├── edgequake.ts          # UPDATE - add lineage/cost APIs
│       └── websocket.ts            # NEW
├── stores/
│   ├── use-ingestion-store.ts      # NEW
│   └── use-cost-store.ts           # NEW
└── types/
    └── index.ts                  # UPDATE - add new types
```

---

_End of Document WEBUI-001_
