# Iteration 12: Observe

## Mission Re-Read ✅

- [x] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed objectives: 6-phase pipeline, edgequake-pdf first, real-time UI
- [x] Current phase: Phase 2 - Backend Implementation (Iterations 11-25)

## Current Focus: Progress Persistence

The mission specifies:

- `GET /api/v1/documents/pdf/:id/progress` - Get current progress
- Historical tracking: Maintain upload history with success/failure rates

Current state:

- ProgressEvent/PipelineEvent are emitted but ephemeral (in-memory broadcast)
- WebSocket clients receive live events but lose history on reconnect
- No way to query "what's the current progress of PDF xyz?"

## Code Analysis: Current Progress Flow

```
PDF Extraction
     │
     v
ProgressCallback (on_page_start/complete/error)
     │
     v
PipelineProgressCallback
     │
     ├── PipelineState.emit_pdf_page_progress() → broadcast::Sender
     │                                              │
     │                                              └── Ephemeral! Lost when receiver drops
     │
     └── ProgressBroadcaster.broadcast() → broadcast::Sender
                                              │
                                              └── Ephemeral! Lost when receiver drops
```

## What We Need

```
PDF Extraction
     │
     v
ProgressCallback
     │
     v
PipelineProgressCallback
     │
     ├── Broadcast (for WebSocket)
     │
     └── Persist (for GET endpoint) ← NEW
             │
             ├── Option A: Write to TaskStorage (edgequake-tasks)
             ├── Option B: Write to PdfDocumentStorage (edgequake-storage)
             └── Option C: In-memory map with TTL
```

## Task Storage Analysis

The task system already has:

- `TaskStorage` trait with `update_task_result()`
- Tasks have `metadata` field (HashMap<String, Value>)
- PDF upload creates a task with `track_id`

Possible approach: Store progress in task metadata.

## PdfDocumentStorage Analysis

The PDF storage has:

- `update_pdf_status()` method
- `PdfProcessingStatus` enum (Pending, Processing, Completed, Failed)
- Could add `PdfProgressState` table or column

## Questions to Answer

1. Where should progress be persisted?
2. How to link progress to pdf_id vs task_id vs track_id?
3. What's the granularity? (every page, every 10%, final only)
4. How to handle concurrent updates?
5. What's the retention policy? (delete after 24h, keep forever?)

## Data Gathered

1. Task metadata can store arbitrary JSON - good for progress
2. PdfDocument table has processing_status but no progress field
3. Events flow through broadcast channels (ephemeral)
4. No current persistence layer for granular progress

## Design Options

### Option A: Task Metadata

- Store progress in task.metadata["progress"] as JSON
- Pros: No schema changes, uses existing task system
- Cons: Task may complete before query, need to keep task around

### Option B: Dedicated Progress Table

- New table: `pdf_progress(pdf_id, page_num, total_pages, phase, updated_at)`
- Pros: Clean separation, can query by pdf_id
- Cons: Schema migration needed

### Option C: In-Memory Cache

- Store progress in a DashMap<String, PdfUploadProgress>
- Pros: Fast, no DB overhead
- Cons: Lost on restart, doesn't scale across instances

## Recommendation

**Option A (Task Metadata)** for OODA-12:

- Quick to implement
- No schema changes
- Task persists for job lifetime
- Can be queried via existing task endpoints

Later: Can migrate to Option B for production if needed.
