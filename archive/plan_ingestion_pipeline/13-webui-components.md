# WebUI Specification: Component Specifications

> Document ID: WEBUI-004
> Version: 1.0
> Created: 2024-12-28
> Status: SPECIFICATION

---

## Table of Contents

1. [Component Overview](#1-component-overview)
2. [New Components](#2-new-components)
3. [Updated Components](#3-updated-components)
4. [Shared Components](#4-shared-components)
5. [Component Props & Interfaces](#5-component-props--interfaces)
6. [Accessibility Requirements](#6-accessibility-requirements)

---

## 1. Component Overview

### 1.1 Component Inventory

| Component                | Category  | Status | Priority |
| ------------------------ | --------- | ------ | -------- |
| `IngestionProgressPanel` | Progress  | NEW    | P0       |
| `StageIndicator`         | Progress  | NEW    | P0       |
| `CostBadge`              | Cost      | NEW    | P0       |
| `CostBreakdownChart`     | Cost      | NEW    | P1       |
| `ChunkExplorer`          | Lineage   | NEW    | P0       |
| `LineageGraph`           | Lineage   | NEW    | P1       |
| `EntityProvenance`       | Lineage   | NEW    | P1       |
| `WebSocketStatus`        | Shared    | NEW    | P0       |
| `DocumentManager`        | Documents | UPDATE | P0       |
| `DocumentDetailPanel`    | Documents | UPDATE | P0       |
| `LineageTree`            | Document  | UPDATE | P0       |
| `BatchProgressCard`      | Documents | UPDATE | P0       |

### 1.2 Component Hierarchy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         COMPONENT HIERARCHY                                 │
└─────────────────────────────────────────────────────────────────────────────┘

src/components/
├── documents/
│   ├── document-manager.tsx         # UPDATE: Add cost column, progress
│   ├── batch-progress-card.tsx      # UPDATE: WebSocket integration
│   ├── ingestion-progress-panel.tsx # NEW: Real-time progress
│   ├── upload-progress-list.tsx     # UPDATE: Enhanced states
│   ├── cost-badge.tsx               # NEW: Inline cost display
│   └── document-row.tsx             # NEW: Extracted row component
│
├── document/
│   ├── lineage-tree.tsx             # UPDATE: Interactive, expandable
│   ├── chunk-explorer.tsx           # NEW: Browse chunks
│   ├── chunk-detail-modal.tsx       # NEW: Full chunk view
│   ├── entity-provenance.tsx        # NEW: Entity sources
│   └── extraction-details.tsx       # UPDATE: More metadata
│
├── progress/
│   ├── stage-indicator.tsx          # NEW: Pipeline stages
│   ├── progress-timeline.tsx        # NEW: Horizontal timeline
│   ├── live-message.tsx             # NEW: Streaming messages
│   └── eta-display.tsx              # NEW: Time estimates
│
├── cost/
│   ├── cost-breakdown-chart.tsx     # NEW: Pie/bar chart
│   ├── cost-summary-card.tsx        # NEW: Overview card
│   ├── budget-indicator.tsx         # NEW: Budget status
│   └── token-usage-table.tsx        # NEW: Token details
│
├── lineage/
│   ├── lineage-graph.tsx            # NEW: Interactive visualization
│   ├── lineage-node.tsx             # NEW: Graph node
│   └── lineage-edge.tsx             # NEW: Graph edge
│
└── shared/
    ├── websocket-status.tsx         # NEW: Connection indicator
    ├── animated-progress.tsx        # NEW: Smooth progress bar
    └── api-error-display.tsx        # NEW: Error component
```

---

## 2. New Components

### 2.1 IngestionProgressPanel

Real-time ingestion progress display with WebSocket support.

```tsx
// src/components/documents/ingestion-progress-panel.tsx

interface IngestionProgressPanelProps {
  trackId: string;
  documentName: string;
  onComplete?: () => void;
  onCancel?: () => void;
  className?: string;
}

/**
 * Displays real-time ingestion progress for a document.
 *
 * Features:
 * - Stage-by-stage progress visualization
 * - Live cost tracking
 * - ETA estimation
 * - Cancel/pause controls
 * - WebSocket-powered updates with polling fallback
 */
export function IngestionProgressPanel({
  trackId,
  documentName,
  onComplete,
  onCancel,
  className,
}: IngestionProgressPanelProps) {
  // Implementation uses useIngestionProgress hook
}
```

**Visual Structure:**

```
┌────────────────────────────────────────────────────────────────────────────┐
│ 📊 Ingesting: {documentName}                          [⏸ Pause] [✗ Cancel]│
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│ Overall: ████████████████████░░░░░░░░░░░░░░░░░░ 65%    ETA: ~45s          │
│                                                                            │
│ ┌────────────────────────────────────────────────────────────────────────┐│
│ │ <StageIndicator stages={stages} currentStage="extracting" />           ││
│ └────────────────────────────────────────────────────────────────────────┘│
│                                                                            │
│ ┌────────────────────────────────────────────────────────────────────────┐│
│ │ <LiveMessage message="Extracting entities from chunk 5/10..." />       ││
│ └────────────────────────────────────────────────────────────────────────┘│
│                                                                            │
│ ┌────────────────────────────────────────────────────────────────────────┐│
│ │ 💰 <CostBadge cost={0.0033} estimated={0.0045} showBreakdown />        ││
│ └────────────────────────────────────────────────────────────────────────┘│
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 StageIndicator

Pipeline stage visualization with progress.

```tsx
// src/components/progress/stage-indicator.tsx

interface Stage {
  id: string;
  label: string;
  status: "pending" | "running" | "completed" | "failed";
  progress?: number; // 0-100 for running stage
  duration?: number; // ms
  message?: string;
}

interface StageIndicatorProps {
  stages: Stage[];
  currentStage: string;
  variant?: "horizontal" | "vertical";
  showDetails?: boolean;
  className?: string;
}

/**
 * Displays pipeline stages with status indicators.
 *
 * Variants:
 * - horizontal: Timeline view for desktop
 * - vertical: Stacked view for mobile/sidebar
 */
export function StageIndicator({
  stages,
  currentStage,
  variant = "horizontal",
  showDetails = true,
  className,
}: StageIndicatorProps) {
  // Implementation
}
```

**Horizontal Variant:**

```
   ┌─────┐     ┌─────┐     ┌─────┐     ┌─────┐     ┌─────┐
   │ ✓ 1 │────▶│ ✓ 2 │────▶│ ◐ 3 │────▶│ ○ 4 │────▶│ ○ 5 │
   └─────┘     └─────┘     └─────┘     └─────┘     └─────┘
     Pre-       Chunk      Extract      Merge       Index
     2.1s       1.3s       45%          —           —
```

**Vertical Variant:**

```
┌─ ✓ Preprocessing ────────────────────── 2.1s ─┐
│    Validated, normalized                      │
└───────────────────────────────────────────────┘
        │
┌─ ✓ Chunking ─────────────────────────── 1.3s ─┐
│    10 chunks created                          │
└───────────────────────────────────────────────┘
        │
┌─ ◐ Extracting ────────────────────────── 45% ─┐
│    Chunk 5/10 | 18 entities found             │
└───────────────────────────────────────────────┘
        │
┌─ ○ Merging ──────────────────────── pending ──┐
│    Waiting...                                 │
└───────────────────────────────────────────────┘
```

### 2.3 CostBadge

Inline cost display with optional breakdown tooltip.

```tsx
// src/components/documents/cost-badge.tsx

interface CostBadgeProps {
  cost: number; // USD
  estimated?: number; // Estimated final cost
  showBreakdown?: boolean;
  breakdown?: CostBreakdown;
  size?: "sm" | "md" | "lg";
  className?: string;
}

/**
 * Displays cost in USD with optional breakdown tooltip.
 */
export function CostBadge({
  cost,
  estimated,
  showBreakdown = false,
  breakdown,
  size = "md",
  className,
}: CostBadgeProps) {
  // Format: $0.0045 or $0.00 (0) / $0.01 (est)
}
```

**Visual States:**

```
Size sm:  💰 $0.004          (inline badge)
Size md:  💰 $0.0045         (standard)
Size lg:  💰 $0.0045 / $0.01 (with estimate)

With breakdown hover:
┌─────────────────────────┐
│ Cost Breakdown          │
│ ────────────────────── │
│ Extraction:  $0.0040   │
│ Gleaning:    $0.0004   │
│ Embedding:   $0.0001   │
│ ────────────────────── │
│ Total:       $0.0045   │
└─────────────────────────┘
```

### 2.4 ChunkExplorer

Browse document chunks with entity highlighting.

```tsx
// src/components/document/chunk-explorer.tsx

interface ChunkExplorerProps {
  documentId: string;
  chunks: ChunkLineage[];
  onChunkSelect: (chunkId: string) => void;
  selectedChunkId?: string;
  highlightEntities?: boolean;
  className?: string;
}

/**
 * Displays a list of chunks with extraction summaries.
 * Clicking a chunk opens the ChunkDetailModal.
 */
export function ChunkExplorer({
  documentId,
  chunks,
  onChunkSelect,
  selectedChunkId,
  highlightEntities = true,
  className,
}: ChunkExplorerProps) {
  // Implementation
}
```

**Visual Structure:**

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Chunks (10)                                              [🔍 Search]       │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│ ┌──────────────────────────────────────────────────────────────────────┐  │
│ │ 📦 Chunk 1                                    Lines 1-50 | 1,200 tok │  │
│ │    "Dr. Sarah Chen, the lead researcher at Quantum Labs..."         │  │
│ │    👤 3 entities | 🔗 2 relationships | ⚡ 2.3s                      │  │
│ └──────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
│ ┌──────────────────────────────────────────────────────────────────────┐  │
│ │ 📦 Chunk 2 (selected)                        Lines 45-95 | 1,180 tok │  │
│ │    "The collaboration with MIT and Stanford..."                      │  │
│ │    🏢 2 entities | 🔗 1 relationships | ⚡ 1.8s                      │  │
│ └──────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
│ ... (8 more chunks)                                                        │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### 2.5 CostBreakdownChart

Visual cost breakdown with pie or bar chart.

```tsx
// src/components/cost/cost-breakdown-chart.tsx

interface CostBreakdownChartProps {
  breakdown: CostBreakdown;
  type?: "pie" | "bar";
  showLegend?: boolean;
  showValues?: boolean;
  height?: number;
  className?: string;
}

/**
 * Visualizes cost breakdown by operation type.
 * Uses Recharts for rendering.
 */
export function CostBreakdownChart({
  breakdown,
  type = "pie",
  showLegend = true,
  showValues = true,
  height = 200,
  className,
}: CostBreakdownChartProps) {
  // Recharts implementation
}
```

**Pie Chart:**

```
        ┌────────────────────┐
        │    Extraction      │
        │       81%          │
        │   ╱─────────╲      │
        │  ╱    $12.50 ╲     │
        │ │             │    │
        │  ╲ Gleaning  ╱     │
        │   ╲   12%   ╱      │
        │    ╲───────╱       │
        └────────────────────┘

        Legend:
        ■ Extraction  $12.50 (81%)
        ■ Gleaning    $1.80  (12%)
        ■ Summarize   $0.90  (6%)
        ■ Embedding   $0.25  (1%)
```

### 2.6 WebSocketStatus

Connection status indicator.

```tsx
// src/components/shared/websocket-status.tsx

interface WebSocketStatusProps {
  showLabel?: boolean;
  size?: "sm" | "md";
  className?: string;
}

/**
 * Shows WebSocket connection status.
 * Green dot = connected, yellow = reconnecting, red = disconnected
 */
export function WebSocketStatus({
  showLabel = true,
  size = "sm",
  className,
}: WebSocketStatusProps) {
  const { connected, reconnecting } = useWebSocket();

  // Visual: ● Connected / ◐ Reconnecting / ○ Disconnected
}
```

---

## 3. Updated Components

### 3.1 DocumentManager Updates

```tsx
// src/components/documents/document-manager.tsx

// ADD: Cost column to table
const columns = [
  // ... existing columns ...
  {
    id: "cost",
    header: "Cost",
    cell: ({ row }) => <CostBadge cost={row.original.cost ?? 0} size="sm" />,
    sortable: true,
  },
];

// ADD: WebSocket connection for real-time updates
useEffect(() => {
  if (activeTrackId) {
    // Subscribe to WebSocket for this track
  }
}, [activeTrackId]);

// ADD: Processing status with progress bar in status cell
const StatusCell = ({ document }: { document: Document }) => {
  if (document.status === "processing" && document.track_id) {
    return <InlineProgress trackId={document.track_id} />;
  }
  return <StatusBadge status={document.status} />;
};
```

### 3.2 LineageTree Updates

```tsx
// src/components/document/lineage-tree.tsx

interface LineageTreeProps {
  lineage: DocumentLineage;
  expanded?: boolean;
  onChunkClick?: (chunkId: string) => void;
  onEntityClick?: (entityId: string) => void;
}

// ADD: Interactive nodes that expand to show details
// ADD: Click handlers to drill into chunks/entities
// ADD: Extraction metadata display (model, time, tokens)
// ADD: Cache hit indicators
```

### 3.3 BatchProgressCard Updates

```tsx
// src/components/documents/batch-progress-card.tsx

// UPDATE: Use WebSocket instead of polling when available
const { progress, isLive } = useIngestionProgress(trackId);

// ADD: Live indicator when using WebSocket
{
  isLive && <WebSocketStatus showLabel={false} />;
}

// ADD: Per-document progress within batch
// ADD: Cost accumulation display
```

---

## 4. Shared Components

### 4.1 AnimatedProgress

Smooth animated progress bar.

```tsx
// src/components/shared/animated-progress.tsx

interface AnimatedProgressProps {
  value: number; // 0-100
  max?: number;
  showValue?: boolean;
  variant?: "default" | "success" | "warning" | "error";
  size?: "sm" | "md" | "lg";
  animated?: boolean;
  className?: string;
}

/**
 * Smooth animated progress bar using CSS transitions.
 * Updates animate over 300ms for smooth visual feedback.
 */
export function AnimatedProgress({
  value,
  max = 100,
  showValue = false,
  variant = "default",
  size = "md",
  animated = true,
  className,
}: AnimatedProgressProps) {
  // Use spring animation for smooth transitions
}
```

### 4.2 LiveMessage

Streaming message display.

```tsx
// src/components/progress/live-message.tsx

interface LiveMessageProps {
  message: string;
  timestamp?: string;
  level?: "info" | "success" | "warning" | "error";
  showIcon?: boolean;
  className?: string;
}

/**
 * Displays live status messages with fade-in animation.
 */
export function LiveMessage({
  message,
  timestamp,
  level = "info",
  showIcon = true,
  className,
}: LiveMessageProps) {
  // Fade-in animation on message change
}
```

### 4.3 EtaDisplay

Estimated time remaining display.

```tsx
// src/components/progress/eta-display.tsx

interface EtaDisplayProps {
  etaSeconds?: number;
  startedAt?: string;
  showElapsed?: boolean;
  className?: string;
}

/**
 * Shows estimated time remaining with live countdown.
 */
export function EtaDisplay({
  etaSeconds,
  startedAt,
  showElapsed = false,
  className,
}: EtaDisplayProps) {
  // Format: "~45s remaining" or "2m 30s elapsed"
}
```

---

## 5. Component Props & Interfaces

### 5.1 Common Props Pattern

```typescript
// Base props for all components
interface BaseComponentProps {
  className?: string;
  "data-testid"?: string;
}

// Loading state pattern
interface LoadableProps {
  isLoading?: boolean;
  error?: Error | null;
}

// Interactive element pattern
interface InteractiveProps {
  disabled?: boolean;
  onAction?: () => void;
}
```

### 5.2 Component Prop Summary

| Component              | Key Props                     | Events               |
| ---------------------- | ----------------------------- | -------------------- |
| IngestionProgressPanel | trackId, documentName         | onComplete, onCancel |
| StageIndicator         | stages, currentStage, variant | —                    |
| CostBadge              | cost, estimated, breakdown    | —                    |
| ChunkExplorer          | documentId, chunks            | onChunkSelect        |
| CostBreakdownChart     | breakdown, type               | —                    |
| WebSocketStatus        | showLabel, size               | —                    |
| AnimatedProgress       | value, variant, animated      | —                    |
| LiveMessage            | message, level                | —                    |
| EtaDisplay             | etaSeconds, startedAt         | —                    |

---

## 6. Accessibility Requirements

### 6.1 WCAG 2.1 AA Compliance

| Requirement             | Implementation                                        |
| ----------------------- | ----------------------------------------------------- |
| **Keyboard Navigation** | All interactive elements focusable, logical tab order |
| **Screen Reader**       | ARIA labels on all icons, progress announcements      |
| **Color Contrast**      | 4.5:1 minimum for text, 3:1 for UI elements           |
| **Focus Indicators**    | Visible focus rings on all interactive elements       |
| **Motion**              | Respect `prefers-reduced-motion` preference           |

### 6.2 ARIA Patterns

```tsx
// Progress bar
<div
  role="progressbar"
  aria-valuenow={progress}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-label={`${stage} progress`}
>
  {/* Visual progress */}
</div>

// Status badge
<Badge aria-label={`Status: ${status}`}>
  <span aria-hidden="true">●</span>
  {label}
</Badge>

// Stage indicator
<ol role="list" aria-label="Pipeline stages">
  {stages.map((stage, index) => (
    <li
      key={stage.id}
      aria-current={stage.id === currentStage ? 'step' : undefined}
    >
      {stage.label}
    </li>
  ))}
</ol>
```

### 6.3 Motion Preferences

```tsx
// Respect reduced motion
const prefersReducedMotion = useMediaQuery('(prefers-reduced-motion: reduce)');

<div
  className={cn(
    'transition-all',
    prefersReducedMotion ? 'duration-0' : 'duration-300'
  )}
>
```

---

## Appendix: Component File Structure

```
src/components/
├── documents/
│   ├── document-manager.tsx         # ~1000 lines
│   ├── batch-progress-card.tsx      # ~200 lines
│   ├── ingestion-progress-panel.tsx # ~300 lines (NEW)
│   ├── cost-badge.tsx               # ~80 lines (NEW)
│   ├── document-row.tsx             # ~150 lines (NEW)
│   └── index.ts                     # Exports
│
├── document/
│   ├── lineage-tree.tsx             # ~150 lines
│   ├── chunk-explorer.tsx           # ~250 lines (NEW)
│   ├── chunk-detail-modal.tsx       # ~300 lines (NEW)
│   ├── entity-provenance.tsx        # ~200 lines (NEW)
│   └── index.ts
│
├── progress/
│   ├── stage-indicator.tsx          # ~200 lines (NEW)
│   ├── progress-timeline.tsx        # ~150 lines (NEW)
│   ├── live-message.tsx             # ~60 lines (NEW)
│   ├── eta-display.tsx              # ~80 lines (NEW)
│   └── index.ts
│
├── cost/
│   ├── cost-breakdown-chart.tsx     # ~150 lines (NEW)
│   ├── cost-summary-card.tsx        # ~120 lines (NEW)
│   ├── budget-indicator.tsx         # ~80 lines (NEW)
│   ├── token-usage-table.tsx        # ~100 lines (NEW)
│   └── index.ts
│
├── lineage/
│   ├── lineage-graph.tsx            # ~400 lines (NEW)
│   ├── lineage-node.tsx             # ~100 lines (NEW)
│   ├── lineage-edge.tsx             # ~60 lines (NEW)
│   └── index.ts
│
└── shared/
    ├── websocket-status.tsx         # ~50 lines (NEW)
    ├── animated-progress.tsx        # ~80 lines (NEW)
    ├── api-error-display.tsx        # ~100 lines (NEW)
    └── index.ts
```

---

_End of Document WEBUI-004_
