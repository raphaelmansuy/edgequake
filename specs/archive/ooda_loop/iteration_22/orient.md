# Iteration 22: Orient

## Gap Analysis

### Required by Mission Spec

- Queue position (#1, #2, #3...)
- Wait time per document (how long waiting)
- Estimated start time

### What Exists

- TaskResponse has `created_at` for wait time calculation
- Tasks API returns pending tasks
- QueueMetrics has `estimated_queue_time_seconds`

### What's Missing

- UI showing queue position
- Wait time calculation and display
- Better task list organization

## Design: Enhanced Task Queue Card

### New Structure

```
┌────────────────────────────────────────────────────────────────┐
│ TASK QUEUE                                         12 waiting  │
├────────────────────────────────────────────────────────────────┤
│ PENDING (12)                                                   │
│ ┌────┬────────────────────────┬──────────────┐                │
│ │ #  │ Task Type              │ Wait Time    │                │
│ ├────┼────────────────────────┼──────────────┤                │
│ │ 1  │ Document Ingestion     │ 2m 45s       │                │
│ │ 2  │ Document Ingestion     │ 2m 32s       │                │
│ │ 3  │ Document Ingestion     │ 2m 28s       │                │
│ └────┴────────────────────────┴──────────────┘                │
│                                                                │
│ PROCESSING (3)                                                 │
│ • Document Ingestion - 0:45 ago                               │
│ • Document Ingestion - 0:32 ago                               │
└────────────────────────────────────────────────────────────────┘
```

### Implementation

1. Filter tasks by status (pending vs processing)
2. Sort pending by created_at (oldest first = queue order)
3. Calculate wait time as `Date.now() - created_at`
4. Add queue position number

## Effort

- Small: ~40 lines of changes to TaskQueueCard
- No API changes needed
- Pure frontend enhancement
