# Iteration 21: Orient

## Gap Analysis

### Required by Mission Spec (Objective B)

- ✅ Queue depth (pending_count)
- ✅ Processing rate (throughput_per_minute)
- ✅ Worker utilization (active_workers/max_workers)
- ✅ Average wait time (avg_wait_time_seconds)
- ⏳ UI visualization needed

### What Exists

- Backend: QueueMetrics struct ✅
- API: /pipeline/queue-metrics endpoint ✅

### What's Missing

- Frontend TypeScript interface for QueueMetrics
- API function to call queue-metrics endpoint
- QueueMetricsCard React component

## Implementation Pattern

Following existing pattern from `getEnhancedPipelineStatus`:

```typescript
// types/index.ts
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

// lib/api/edgequake.ts
export async function getQueueMetrics(): Promise<QueueMetrics> {
  return api.get<QueueMetrics>("/pipeline/queue-metrics");
}
```

## Component Design

### QueueMetricsCard Features

1. Worker utilization gauge/bar
2. Three metric tiles: Throughput, Avg Wait, Queue ETA
3. Queue status footer (pending count, rate limit status)
4. Live indicator badge

### Placement in PipelineMonitor

Add as sibling to PipelineProgressCard in left column

## Risk Assessment

- LOW: Simple component following existing patterns
- No breaking changes
- Adds value without modifying existing functionality
