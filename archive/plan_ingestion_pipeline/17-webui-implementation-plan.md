# WebUI Specification: Implementation Plan

> Document ID: WEBUI-008
> Version: 1.0
> Created: 2024-12-28
> Status: SPECIFICATION

---

## Table of Contents

1. [Overview](#1-overview)
2. [Implementation Phases](#2-implementation-phases)
3. [File Changes Summary](#3-file-changes-summary)
4. [Detailed Task Breakdown](#4-detailed-task-breakdown)
5. [Dependencies](#5-dependencies)
6. [Testing Strategy](#6-testing-strategy)
7. [Rollout Plan](#7-rollout-plan)
8. [Risk Assessment](#8-risk-assessment)

---

## 1. Overview

### 1.1 Purpose

This document provides a detailed implementation roadmap for the WebUI specification. It covers all file changes, task breakdowns, dependencies, and rollout strategy.

### 1.2 Timeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     WEBUI IMPLEMENTATION TIMELINE                           │
└─────────────────────────────────────────────────────────────────────────────┘

Week 6    Week 7    Week 8    Week 9    Week 10
  │         │         │         │         │
  ├─────────┼─────────┼─────────┼─────────┤
  │  Phase  │  Phase  │  Phase  │  Phase  │
  │   W1    │   W2    │   W3    │   W4    │
  │         │         │         │         │
  │ Found-  │ Progress│ Lineage │  Cost   │
  │ ation   │ Comps   │  Viz    │ Monitor │
  │         │         │         │         │
  └─────────┴─────────┴─────────┴─────────┘

Milestones:
├── M1: WebSocket client operational (End Week 6)
├── M2: Real-time progress working (End Week 7)
├── M3: Lineage explorer complete (End Week 8)
├── M4: Cost dashboard live (End Week 9)
└── M5: E2E tests passing (End Week 10)
```

### 1.3 Effort Summary

| Phase     | Focus                 | Effort      | New Files | Modified Files |
| --------- | --------------------- | ----------- | --------- | -------------- |
| W1        | Foundation            | 3 days      | 8         | 4              |
| W2        | Progress Components   | 4 days      | 6         | 3              |
| W3        | Lineage Visualization | 5 days      | 9         | 2              |
| W4        | Cost Monitoring       | 4 days      | 8         | 2              |
| **Total** |                       | **16 days** | **31**    | **11**         |

---

## 2. Implementation Phases

### Phase W1: Foundation (Week 6-7)

**Objective:** Establish WebSocket infrastructure and state management.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ PHASE W1: FOUNDATION                                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Priority 0 Tasks:                                                          │
│  ═════════════════                                                          │
│                                                                             │
│  W1-001: Create WebSocket Client                         [4h]              │
│    └── src/lib/websocket/progress-websocket.ts                             │
│    └── src/lib/websocket/websocket-manager.ts                              │
│                                                                             │
│  W1-002: Create Ingestion Store                          [3h]              │
│    └── src/lib/stores/use-ingestion-store.ts                               │
│                                                                             │
│  W1-003: Create Cost Store                               [2h]              │
│    └── src/lib/stores/use-cost-store.ts                                    │
│                                                                             │
│  W1-004: Add TypeScript Types                            [3h]              │
│    └── src/types/ingestion.ts (NEW)                                        │
│    └── src/types/cost.ts (NEW)                                             │
│    └── src/types/lineage.ts (NEW)                                          │
│    └── src/types/index.ts (UPDATE)                                         │
│                                                                             │
│  W1-005: Create WebSocket Provider                       [2h]              │
│    └── src/providers/websocket-provider.tsx                                │
│    └── src/app/layout.tsx (UPDATE)                                         │
│                                                                             │
│  W1-006: Create WebSocket Hooks                          [3h]              │
│    └── src/lib/hooks/use-websocket.ts                                      │
│    └── src/lib/hooks/use-ingestion-progress.ts                             │
│                                                                             │
│  W1-007: Create WebSocketStatus Component                [1h]              │
│    └── src/components/shared/websocket-status.tsx                          │
│                                                                             │
│  W1-008: Update API Client                               [2h]              │
│    └── src/lib/api/edgequake.ts (UPDATE)                                   │
│    └── Add lineage, cost, progress endpoints                               │
│                                                                             │
│  Estimated Total: 20h (2.5 days)                                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Phase W2: Progress Components (Week 7-8)

**Objective:** Build real-time ingestion progress UI.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ PHASE W2: PROGRESS COMPONENTS                                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  W2-001: Create StageIndicator                           [4h]              │
│    └── src/components/progress/stage-indicator.tsx                         │
│    └── Horizontal and vertical variants                                    │
│    └── Animated progress for running stage                                 │
│                                                                             │
│  W2-002: Create IngestionProgressPanel                   [6h]              │
│    └── src/components/documents/ingestion-progress-panel.tsx               │
│    └── WebSocket integration                                               │
│    └── Cancel/pause controls                                               │
│    └── ETA display                                                         │
│                                                                             │
│  W2-003: Create Progress Utilities                       [3h]              │
│    └── src/components/progress/live-message.tsx                            │
│    └── src/components/progress/eta-display.tsx                             │
│    └── src/components/shared/animated-progress.tsx                         │
│                                                                             │
│  W2-004: Update BatchProgressCard                        [3h]              │
│    └── src/components/documents/batch-progress-card.tsx (UPDATE)           │
│    └── WebSocket integration                                               │
│    └── Per-document progress                                               │
│                                                                             │
│  W2-005: Update DocumentManager                          [4h]              │
│    └── src/components/documents/document-manager.tsx (UPDATE)              │
│    └── Add cost column                                                     │
│    └── Add inline progress for processing docs                             │
│    └── WebSocket subscription                                              │
│                                                                             │
│  W2-006: Create CostBadge                                [2h]              │
│    └── src/components/documents/cost-badge.tsx                             │
│    └── Tooltip with breakdown                                              │
│                                                                             │
│  Estimated Total: 22h (2.75 days)                                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Phase W3: Lineage Visualization (Week 8-9)

**Objective:** Build lineage exploration and visualization.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ PHASE W3: LINEAGE VISUALIZATION                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  W3-001: Create Lineage API Hooks                        [3h]              │
│    └── src/lib/hooks/use-lineage-queries.ts                                │
│    └── useDocumentLineage, useEntityProvenance                             │
│                                                                             │
│  W3-002: Create ChunkExplorer                            [5h]              │
│    └── src/components/document/chunk-explorer.tsx                          │
│    └── Chunk list with entity counts                                       │
│    └── Search/filter                                                       │
│                                                                             │
│  W3-003: Create ChunkDetailModal                         [4h]              │
│    └── src/components/document/chunk-detail-modal.tsx                      │
│    └── Full content view                                                   │
│    └── Extraction metadata                                                 │
│    └── Entity list                                                         │
│                                                                             │
│  W3-004: Update LineageTree                              [4h]              │
│    └── src/components/document/lineage-tree.tsx (UPDATE)                   │
│    └── Interactive expansion                                               │
│    └── Click handlers                                                      │
│    └── Metadata display                                                    │
│                                                                             │
│  W3-005: Create LineageExplorer Container                [3h]              │
│    └── src/components/lineage/lineage-explorer.tsx                         │
│    └── Tab switching (tree/graph/table)                                    │
│    └── Filter/search                                                       │
│                                                                             │
│  W3-006: Create LineageGraphView                         [8h]              │
│    └── src/components/lineage/lineage-graph-view.tsx                       │
│    └── Install: reactflow                                                  │
│    └── Custom node components                                              │
│    └── Layout algorithm                                                    │
│                                                                             │
│  W3-007: Create Graph Nodes                              [3h]              │
│    └── src/components/lineage/graph-nodes/document-node.tsx                │
│    └── src/components/lineage/graph-nodes/chunk-node.tsx                   │
│    └── src/components/lineage/graph-nodes/entity-node.tsx                  │
│                                                                             │
│  W3-008: Create LineageTableView                         [2h]              │
│    └── src/components/lineage/lineage-table-view.tsx                       │
│                                                                             │
│  W3-009: Create EntityProvenance Panel                   [4h]              │
│    └── src/components/lineage/entity-provenance-panel.tsx                  │
│    └── Source chunks display                                               │
│    └── Merge history                                                       │
│    └── Relationships                                                       │
│                                                                             │
│  Estimated Total: 36h (4.5 days)                                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Phase W4: Cost Monitoring (Week 9-10)

**Objective:** Build cost dashboard and monitoring UI.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ PHASE W4: COST MONITORING                                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  W4-001: Create Cost API Hooks                           [2h]              │
│    └── src/lib/hooks/use-cost-queries.ts                                   │
│    └── useCostSummary, useDocumentCost, useBudgetStatus                    │
│                                                                             │
│  W4-002: Create CostSummaryCard                          [2h]              │
│    └── src/components/cost/cost-summary-card.tsx                           │
│                                                                             │
│  W4-003: Create CostBreakdownChart                       [4h]              │
│    └── src/components/cost/cost-breakdown-chart.tsx                        │
│    └── Install: recharts                                                   │
│    └── Pie chart for operation breakdown                                   │
│                                                                             │
│  W4-004: Create CostTrendChart                           [3h]              │
│    └── src/components/cost/cost-trend-chart.tsx                            │
│    └── Line chart for daily trends                                         │
│                                                                             │
│  W4-005: Create TokenUsageTable                          [2h]              │
│    └── src/components/cost/token-usage-table.tsx                           │
│                                                                             │
│  W4-006: Create BudgetIndicator                          [2h]              │
│    └── src/components/cost/budget-indicator.tsx                            │
│    └── Compact and full variants                                           │
│                                                                             │
│  W4-007: Create BudgetSettings                           [3h]              │
│    └── src/components/cost/budget-settings.tsx                             │
│    └── Daily/monthly limits                                                │
│    └── Alert thresholds                                                    │
│                                                                             │
│  W4-008: Create Cost Dashboard Page                      [5h]              │
│    └── src/app/cost/page.tsx                                               │
│    └── Layout with all cost components                                     │
│    └── Period selection                                                    │
│    └── Export functionality                                                │
│                                                                             │
│  W4-009: Create Export Components                        [2h]              │
│    └── src/components/cost/export-cost-report-button.tsx                   │
│    └── CSV/JSON export                                                     │
│                                                                             │
│  W4-010: Update Navigation                               [1h]              │
│    └── Add cost dashboard link to sidebar                                  │
│                                                                             │
│  Estimated Total: 26h (3.25 days)                                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. File Changes Summary

### 3.1 New Files

```
src/
├── lib/
│   ├── websocket/
│   │   ├── progress-websocket.ts         # WebSocket client class
│   │   ├── websocket-manager.ts          # Singleton manager
│   │   └── index.ts                      # Exports
│   │
│   ├── stores/
│   │   ├── use-ingestion-store.ts        # Ingestion state
│   │   └── use-cost-store.ts             # Cost state
│   │
│   └── hooks/
│       ├── use-websocket.ts              # WebSocket connection hook
│       ├── use-ingestion-progress.ts     # Progress tracking hook
│       ├── use-lineage-queries.ts        # Lineage API hooks
│       └── use-cost-queries.ts           # Cost API hooks
│
├── types/
│   ├── ingestion.ts                      # Ingestion types
│   ├── cost.ts                           # Cost types
│   └── lineage.ts                        # Lineage types
│
├── providers/
│   └── websocket-provider.tsx            # WebSocket context provider
│
├── components/
│   ├── shared/
│   │   ├── websocket-status.tsx          # Connection indicator
│   │   ├── animated-progress.tsx         # Smooth progress bar
│   │   └── api-error-display.tsx         # Error component
│   │
│   ├── progress/
│   │   ├── stage-indicator.tsx           # Pipeline stages
│   │   ├── live-message.tsx              # Streaming messages
│   │   ├── eta-display.tsx               # Time estimates
│   │   └── index.ts
│   │
│   ├── documents/
│   │   ├── ingestion-progress-panel.tsx  # Main progress panel
│   │   └── cost-badge.tsx                # Inline cost display
│   │
│   ├── document/
│   │   ├── chunk-explorer.tsx            # Chunk list
│   │   ├── chunk-detail-modal.tsx        # Chunk details
│   │   └── entity-provenance.tsx         # Entity sources
│   │
│   ├── lineage/
│   │   ├── lineage-explorer.tsx          # Container with tabs
│   │   ├── lineage-graph-view.tsx        # React Flow graph
│   │   ├── lineage-table-view.tsx        # Table view
│   │   ├── graph-nodes/
│   │   │   ├── document-node.tsx
│   │   │   ├── chunk-node.tsx
│   │   │   └── entity-node.tsx
│   │   └── index.ts
│   │
│   └── cost/
│       ├── cost-summary-card.tsx
│       ├── cost-breakdown-chart.tsx
│       ├── cost-trend-chart.tsx
│       ├── token-usage-table.tsx
│       ├── budget-indicator.tsx
│       ├── budget-settings.tsx
│       ├── export-cost-report-button.tsx
│       └── index.ts
│
└── app/
    └── cost/
        └── page.tsx                      # Cost dashboard page
```

### 3.2 Modified Files

| File                                               | Changes                                   |
| -------------------------------------------------- | ----------------------------------------- |
| `src/types/index.ts`                               | Add exports for new types                 |
| `src/lib/api/edgequake.ts`                         | Add lineage, cost, progress API functions |
| `src/app/layout.tsx`                               | Wrap with WebSocketProvider               |
| `src/components/documents/document-manager.tsx`    | Add cost column, WebSocket                |
| `src/components/documents/batch-progress-card.tsx` | WebSocket integration                     |
| `src/components/document/lineage-tree.tsx`         | Interactive, expandable                   |
| `src/app/(dashboard)/layout.tsx`                   | Add navigation link to cost dashboard     |

---

## 4. Detailed Task Breakdown

### 4.1 Task Priority Matrix

| Priority | Description                              | Tasks                          |
| -------- | ---------------------------------------- | ------------------------------ |
| P0       | Critical path, blocks other work         | W1-001, W1-002, W1-004, W2-002 |
| P1       | Important, needed for full functionality | W1-005, W2-001, W3-006, W4-008 |
| P2       | Nice to have, can defer                  | W3-008, W4-007, W4-009         |

### 4.2 Dependency Graph

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     TASK DEPENDENCY GRAPH                                   │
└─────────────────────────────────────────────────────────────────────────────┘

W1-004 (Types)
    │
    ├──▶ W1-001 (WebSocket Client)
    │        │
    │        ├──▶ W1-002 (Ingestion Store)
    │        │        │
    │        │        └──▶ W1-006 (Hooks) ──▶ W2-002 (ProgressPanel)
    │        │
    │        └──▶ W1-005 (Provider) ──▶ W2-004 (BatchProgress)
    │
    └──▶ W1-008 (API Client)
             │
             ├──▶ W3-001 (Lineage Hooks) ──▶ W3-002, W3-006
             │
             └──▶ W4-001 (Cost Hooks) ──▶ W4-002, W4-008

W2-001 (StageIndicator) ──▶ W2-002 (ProgressPanel)

W2-006 (CostBadge) ──▶ W2-005 (DocumentManager)

W3-002 (ChunkExplorer) ──▶ W3-003 (ChunkDetailModal)

W3-005 (LineageExplorer) ◀── W3-004, W3-006, W3-008
```

---

## 5. Dependencies

### 5.1 New NPM Packages

```json
{
  "dependencies": {
    "reactflow": "^11.10.0",
    "recharts": "^2.10.0",
    "zustand": "^4.4.0",
    "immer": "^10.0.0"
  },
  "devDependencies": {
    "@types/recharts": "^1.8.0"
  }
}
```

### 5.2 Backend Dependencies

| API Endpoint                           | Required For          | Backend Ticket |
| -------------------------------------- | --------------------- | -------------- |
| `GET /api/v1/documents/{id}/lineage`   | Lineage visualization | Phase 4        |
| `GET /api/v1/entities/{id}/provenance` | Entity provenance     | Phase 4        |
| `GET /api/v1/costs/summary`            | Cost dashboard        | Phase 3        |
| `GET /api/v1/progress/{track_id}`      | Progress polling      | Phase 3        |
| `WS /api/v1/ws/progress`               | Real-time updates     | Phase 5        |

---

## 6. Testing Strategy

### 6.1 Unit Tests

```
__tests__/
├── lib/
│   ├── websocket/
│   │   └── progress-websocket.test.ts    # WebSocket client tests
│   └── stores/
│       ├── use-ingestion-store.test.ts   # Store tests
│       └── use-cost-store.test.ts
│
└── components/
    ├── progress/
    │   └── stage-indicator.test.tsx      # Component tests
    ├── documents/
    │   ├── ingestion-progress-panel.test.tsx
    │   └── cost-badge.test.tsx
    ├── lineage/
    │   ├── lineage-graph-view.test.tsx
    │   └── chunk-explorer.test.tsx
    └── cost/
        ├── cost-summary-card.test.tsx
        └── cost-breakdown-chart.test.tsx
```

### 6.2 E2E Tests (Playwright)

```typescript
// e2e/ingestion-progress.spec.ts
test("displays real-time progress during upload", async ({ page }) => {
  await page.goto("/documents");

  // Upload a document
  await page.setInputFiles('input[type="file"]', "test-document.txt");

  // Verify progress panel appears
  await expect(
    page.locator('[data-testid="ingestion-progress-panel"]')
  ).toBeVisible();

  // Verify stages are shown
  await expect(page.locator('[data-testid="stage-indicator"]')).toBeVisible();

  // Wait for completion
  await expect(page.locator("text=completed")).toBeVisible({ timeout: 60000 });
});

// e2e/lineage-visualization.spec.ts
test("displays document lineage graph", async ({ page }) => {
  await page.goto("/documents/doc-1");

  // Navigate to lineage tab
  await page.click('[data-testid="lineage-tab"]');

  // Verify graph is rendered
  await expect(page.locator(".react-flow")).toBeVisible();

  // Click on a chunk node
  await page.click('[data-testid="chunk-node-1"]');

  // Verify chunk detail modal
  await expect(
    page.locator('[data-testid="chunk-detail-modal"]')
  ).toBeVisible();
});

// e2e/cost-dashboard.spec.ts
test("displays cost dashboard with breakdown", async ({ page }) => {
  await page.goto("/cost");

  // Verify summary card
  await expect(page.locator('[data-testid="cost-summary-card"]')).toBeVisible();

  // Verify breakdown chart
  await expect(
    page.locator('[data-testid="cost-breakdown-chart"]')
  ).toBeVisible();

  // Export CSV
  await page.click('[data-testid="export-button"]');
  await page.click("text=Export as CSV");

  // Verify download started
  const download = await page.waitForEvent("download");
  expect(download.suggestedFilename()).toMatch(/cost-report.*\.csv/);
});
```

### 6.3 Test Coverage Targets

| Category          | Target         |
| ----------------- | -------------- |
| Unit tests        | 80%            |
| Component tests   | 70%            |
| E2E tests         | Critical paths |
| Visual regression | Key screens    |

---

## 7. Rollout Plan

### 7.1 Feature Flags

```typescript
// src/lib/feature-flags.ts

export const featureFlags = {
  // Phase W1
  "webui-websocket-progress": true,

  // Phase W2
  "webui-progress-panel": true,

  // Phase W3
  "webui-lineage-graph": false, // Enable after Phase W3

  // Phase W4
  "webui-cost-dashboard": false, // Enable after Phase W4
};
```

### 7.2 Rollout Stages

```
Stage 1: Internal Testing (Week 10)
├── Deploy to staging environment
├── Internal team testing
├── Bug fixes and adjustments
└── Performance validation

Stage 2: Beta Users (Week 11)
├── Enable for 10% of users
├── Collect feedback
├── Monitor error rates
└── Fix critical issues

Stage 3: General Availability (Week 12)
├── Enable for all users
├── Documentation update
├── Announcement
└── Support preparation
```

---

## 8. Risk Assessment

### 8.1 Risk Matrix

| Risk                             | Probability | Impact | Mitigation                           |
| -------------------------------- | ----------- | ------ | ------------------------------------ |
| WebSocket connection instability | Medium      | High   | Polling fallback, reconnection logic |
| Large lineage graphs slow        | Medium      | Medium | Virtualization, pagination           |
| Backend API delays               | High        | High   | Mock data for parallel development   |
| React Flow bundle size           | Low         | Low    | Dynamic import, code splitting       |
| Cost calculation discrepancies   | Medium      | Medium | Reconciliation with backend          |

### 8.2 Mitigation Strategies

**WebSocket Instability:**

- Implement exponential backoff reconnection
- Provide visual indicator of connection state
- Fall back to polling when WebSocket unavailable

**Large Lineage Graphs:**

- Use viewport culling in React Flow
- Paginate API responses
- Lazy load entity details

**Backend API Delays:**

- Create comprehensive mock data
- Use MSW for API mocking in development
- Develop features in parallel with backend

---

## Appendix: Code Templates

### A.1 Component Template

```tsx
// src/components/{category}/{component-name}.tsx

import { cn } from '@/lib/utils';

interface {ComponentName}Props {
  // Props definition
  className?: string;
}

export function {ComponentName}({
  // Destructure props
  className,
}: {ComponentName}Props) {
  // Hooks

  // Derived state

  // Handlers

  return (
    <div className={cn('', className)}>
      {/* Component content */}
    </div>
  );
}
```

### A.2 Hook Template

```typescript
// src/lib/hooks/use-{hook-name}.ts

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';

export function use{HookName}(/* params */) {
  const queryClient = useQueryClient();

  // Query
  const query = useQuery({
    queryKey: ['key'],
    queryFn: async () => {
      // Fetch logic
    },
  });

  // Mutation
  const mutation = useMutation({
    mutationFn: async (/* data */) => {
      // Mutation logic
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['key'] });
    },
  });

  return {
    ...query,
    mutate: mutation.mutate,
  };
}
```

### A.3 Store Template

```typescript
// src/lib/stores/use-{store-name}.ts

import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';

interface {StoreName}State {
  // State properties

  // Actions
}

export const use{StoreName} = create<{StoreName}State>()(
  immer((set, get) => ({
    // Initial state

    // Actions
    action: () => {
      set((state) => {
        // Mutate with immer
      });
    },
  }))
);
```

---

_End of Document WEBUI-008_
