# Iteration 21: Act

## Changes Made

### 1. Added QueueMetrics Type

**File**: `edgequake_webui/src/types/index.ts`
**Lines**: 234-265 (added ~30 lines)

```typescript
export interface QueueMetrics {
  pending_count: number;
  processing_count: number;
  active_workers: number;
  max_workers: number;
  worker_utilization: number;
  avg_wait_time_seconds: number;
  max_wait_time_seconds: number;
  throughput_per_minute: number;
  estimated_queue_time_seconds: number;
  rate_limited: boolean;
  timestamp: string;
}
```

### 2. Added getQueueMetrics API Function

**File**: `edgequake_webui/src/lib/api/edgequake.ts`
**Lines**: 1204-1236 (added ~35 lines)

```typescript
export async function getQueueMetrics(): Promise<QueueMetrics> {
  try {
    return await api.get<QueueMetrics>("/pipeline/queue-metrics");
  } catch {
    // Return default metrics if endpoint not available
    return { ... };
  }
}
```

### 3. Created QueueMetricsCard Component

**File**: `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx`
**Lines**: 493-619 (added ~125 lines)

Features:

- Worker utilization gauge with color coding (green/yellow/red)
- Three metric tiles: Throughput, Avg Wait, Queue ETA
- Queue status footer with pending count
- Rate limited warning badge
- Live indicator badge when active

### 4. Integrated into PipelineMonitor Layout

**File**: `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx`
**Line**: 794

Added `<QueueMetricsCard />` to left column between PipelineProgressCard and ProcessingDocumentsCard.

### 5. Updated Refresh Button

**File**: `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx`
**Line**: 773

Added `queryClient.invalidateQueries({ queryKey: ['queue-metrics'] })` to refresh handler.

## Validation

```
npx tsc --noEmit → ✅ No errors (TypeScript compiles)
```

## UI Preview

```
┌────────────────────────────────────────────────────────────────┐
│ QUEUE METRICS                                        ● Live    │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Workers    [████████░░░░] 3/4 (75%)                          │
│                                                                │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐               │
│  │ Throughput │  │ Avg Wait   │  │ Queue ETA  │               │
│  │   2.3/min  │  │   45s      │  │   5m 12s   │               │
│  └────────────┘  └────────────┘  └────────────┘               │
│                                                                │
│  Queue: 12 pending                                             │
└────────────────────────────────────────────────────────────────┘
```

## Objective B Progress

| Requirement                 | Status          |
| --------------------------- | --------------- |
| Backend QueueMetrics struct | ✅ Iteration 19 |
| API endpoint                | ✅ Iteration 20 |
| Frontend QueueMetricsCard   | ✅ Iteration 21 |
| Wait time per document      | ⏳ Iteration 22 |
| Queue order display         | ⏳ Iteration 23 |
