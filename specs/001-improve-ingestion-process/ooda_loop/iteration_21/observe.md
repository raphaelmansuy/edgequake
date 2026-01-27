# Iteration 21: Observe

## Mission Reference

Re-read: `/specs/001-improve-ingestion-process.md`

**Objective B: Workspace-Level Task Queue Visibility**
UI requirements from spec:

- Queue depth (how many waiting)
- Throughput (docs/min)
- Worker utilization (% capacity)
- Wait time estimates

## Current State

### Backend API (from Iteration 20)

Endpoint: `GET /api/v1/pipeline/queue-metrics`

```json
{
  "pending_count": 12,
  "processing_count": 3,
  "active_workers": 3,
  "max_workers": 4,
  "worker_utilization": 75,
  "avg_wait_time_seconds": 45.2,
  "max_wait_time_seconds": 120.5,
  "throughput_per_minute": 2.3,
  "estimated_queue_time_seconds": 312.0,
  "rate_limited": false,
  "timestamp": "2025-01-28T10:30:00Z"
}
```

### Frontend Structure

- Pipeline Monitor: `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx`
- API client: `edgequake_webui/src/lib/api/edgequake.ts`
- Types: `edgequake_webui/src/types/index.ts`

### Existing Components Used

1. `PipelineProgressCard` - Shows task counts (pending, processing, completed, failed)
2. `TaskQueueCard` - Shows recent tasks list
3. `ChunkProgressCard` - Real-time chunk-level progress

### Gap

Missing `QueueMetricsCard` that displays:

- Worker utilization gauge
- Throughput rate
- Average wait time
- Queue ETA

## Files to Modify

1. **types/index.ts** - Add QueueMetrics interface
2. **lib/api/edgequake.ts** - Add getQueueMetrics function
3. **components/pipeline/pipeline-monitor.tsx** - Add QueueMetricsCard component

## Design

```
┌────────────────────────────────────────────────────────────────┐
│ QUEUE METRICS                                        ● Live    │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Workers    [████████░░░░] 3/4 (75%)                          │
│                                                                │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐               │
│  │ Throughput │  │ Avg Wait   │  │ Queue ETA  │               │
│  │   2.3/min  │  │   45.2s    │  │   ~5 min   │               │
│  └────────────┘  └────────────┘  └────────────┘               │
│                                                                │
│  Queue: 12 pending | Rate Limited: No                         │
└────────────────────────────────────────────────────────────────┘
```
