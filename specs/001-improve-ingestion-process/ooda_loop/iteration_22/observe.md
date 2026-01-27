# Iteration 22: Observe

## Mission Reference

Re-read: `/specs/001-improve-ingestion-process.md`

**Objective B: Workspace-Level Task Queue Visibility**

Required UI from spec:

```
│ TASK QUEUE (12 waiting)                                        │
│ ┌────┬────────────────────────┬──────────┬─────────────┐       │
│ │ #  │ Document               │ Wait Time│ Est. Start  │       │
│ ├────┼────────────────────────┼──────────┼─────────────┤       │
│ │ 1  │ report-2024-q4.pdf     │ 0:45     │ ~2 min      │       │
│ │ 2  │ analysis-v2.md         │ 0:32     │ ~4 min      │       │
│ │ 3  │ meeting-notes.txt      │ 0:28     │ ~5 min      │       │
```

## Current State

### Backend Task Type

`edgequake-tasks/src/types.rs:63`

```rust
pub struct Task {
    pub track_id: String,
    pub created_at: DateTime<Utc>,      // ✅ Available
    pub started_at: Option<DateTime<Utc>>,  // ✅ Available
    pub metadata: Option<serde_json::Value>, // Contains doc name
    pub status: TaskStatus,
    // ...
}
```

### Frontend TaskResponse Type

`edgequake_webui/src/types/index.ts:596`

```typescript
export interface TaskResponse {
  track_id: string;
  created_at: string; // ✅ Already available
  started_at?: string; // ✅ Already available
  status: "pending" | "processing" | "indexed" | "failed" | "cancelled";
  metadata?: Record<string, unknown>; // Contains document info
}
```

### Current TaskQueueCard

`edgequake_webui/src/components/pipeline/pipeline-monitor.tsx:686`

Shows:

- Task type
- Status badge

Missing:

- Queue position (#)
- Wait time (calculated from created_at)
- Est. Start time

## Analysis

Wait time can be calculated as:

```typescript
const waitTime = Date.now() - new Date(task.created_at).getTime();
```

Est. Start can be estimated from:

1. Queue position (index in pending tasks)
2. Average processing time per document
3. `estimated_queue_time_seconds` from QueueMetrics

## Design

Enhanced TaskQueueCard to show:

1. Queue position for pending tasks
2. Wait time per task
3. Separate sections for pending vs processing
