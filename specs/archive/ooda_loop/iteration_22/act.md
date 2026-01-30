# Iteration 22: Act

## Changes Made

### Enhanced TaskQueueCard Component

**File**: `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx`
**Lines**: 684-816 (replaced ~50 lines with ~130 lines)

**Features Added:**

1. **Queue Position Display** - Shows `#1`, `#2`, `#3` for pending tasks
2. **Wait Time Calculation** - Calculates time since `created_at`
3. **Separate Sections** - Split into PENDING and PROCESSING sections
4. **Visual Indicators**
   - Yellow background for pending tasks
   - Blue background for processing tasks
   - Timer icon with wait time
   - Loader animation for processing tasks
5. **Queue Count Badge** - Shows "12 waiting" in header
6. **Overflow Handling** - "+N more in queue" for large queues

### Code Highlights

```tsx
// Wait time calculation
const formatWaitTime = (createdAt: string): string => {
  const waitMs = Date.now() - new Date(createdAt).getTime();
  const seconds = Math.floor(waitMs / 1000);
  if (seconds < 60) return `${seconds}s`;
  // ... format as Xm Ys or Xh Ym
};

// Split tasks by status, sort by oldest first (queue order)
const { pendingTasks, processingTasks } = useMemo(() => {
  const pending = tasks.tasks
    .filter((t) => t.status === "pending")
    .sort(
      (a, b) =>
        new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
    );
  // ...
}, [tasks]);
```

## Validation

```
npx tsc --noEmit → ✅ No errors
```

## UI Preview

```
┌────────────────────────────────────────────────────────────────┐
│ ⏱ TASK QUEUE                                     12 waiting   │
├────────────────────────────────────────────────────────────────┤
│ ⏱ PENDING (12)                                                 │
│ #1  Document Ingestion            ⏱ 2m 45s                    │
│ #2  Document Ingestion            ⏱ 2m 32s                    │
│ #3  Document Ingestion            ⏱ 2m 28s                    │
│                                                                │
│ ◔ PROCESSING (3)                                               │
│ ◔  Document Ingestion          Started 45s ago                │
│ ◔  Document Ingestion          Started 32s ago                │
└────────────────────────────────────────────────────────────────┘
```

## Objective B Progress

| Requirement                 | Status                     |
| --------------------------- | -------------------------- |
| Backend QueueMetrics struct | ✅ Iteration 19            |
| API endpoint                | ✅ Iteration 20            |
| Frontend QueueMetricsCard   | ✅ Iteration 21            |
| Wait time per document      | ✅ Iteration 22            |
| Queue order display         | ✅ Iteration 22 (combined) |

**Objective B Complete!** ✓
