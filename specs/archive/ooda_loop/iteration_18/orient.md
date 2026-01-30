# OODA Iteration 18: ORIENT

**Date**: 2025-01-28
**Mission Re-Read**: ✅ YES - `/specs/001-improve-ingestion-process.md`

---

## Gap Analysis

### Priority Ranking (Signal Value × Effort)

| Priority | Gap                             | User Value | Effort | Signal/Effort |
| -------- | ------------------------------- | ---------- | ------ | ------------- |
| **P0**   | TaskProgress lacks chunk fields | CRITICAL   | LOW    | **10**        |
| **P1**   | Pipeline.process() no callback  | CRITICAL   | MEDIUM | **8**         |
| **P2**   | PipelineEvent no chunk events   | HIGH       | LOW    | **8**         |
| **P3**   | PipelineState document-only     | HIGH       | LOW    | **7**         |
| **P4**   | Queue metrics not exposed       | MEDIUM     | MEDIUM | **5**         |

### Analysis: Why Chunk-Level Progress is Missing

**Root Cause**: The system was designed with a BATCH mindset, not STREAMING mindset.

```
BATCH MODEL (Current):
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Document 1  │───▶│   Process    │───▶│   Complete   │
└──────────────┘    └──────────────┘    └──────────────┘
                         │
                    [No visibility]

STREAMING MODEL (Required):
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Document 1  │───▶│  Chunk 1/10  │───▶│  Chunk 2/10  │───▶ ...
└──────────────┘    └──────────────┘    └──────────────┘
                         │                    │
                    [Progress 10%]       [Progress 20%]
```

### Architecture Decision: Where to Track Chunk Progress

**Option A: In `IngestionProgress` (progress.rs)**

- ❌ This is per-job, not integrated with task system
- ❌ Duplicates data with TaskProgress

**Option B: In `TaskProgress` (types.rs)** ✅ CHOSEN

- ✅ Already part of Task struct
- ✅ Persisted in task storage
- ✅ Accessible via API
- ✅ Single source of truth

**Option C: In `PipelineState` (pipeline_state.rs)**

- ❌ Transient, not persisted
- ✅ Good for real-time events
- → Use for broadcasting events, not as source of truth

### Data Flow Design

```
┌─────────────────────────────────────────────────────────────────────┐
│                     CHUNK PROGRESS DATA FLOW                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Pipeline.process_with_progress()                                  │
│       │                                                            │
│       ▼                                                            │
│  [For each chunk extracted]                                        │
│       │                                                            │
│       ├──▶ Update Task.progress.chunk_progress (storage)          │
│       │                                                            │
│       └──▶ Emit PipelineEvent::ChunkProgress (broadcast)          │
│                  │                                                 │
│                  ▼                                                 │
│            WebSocket → Frontend                                    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Technical Design

### 1. ChunkProgress Struct

```rust
/// Chunk-level progress tracking for a document being processed.
///
/// @implements SPEC-001/Objective-A: Chunk-Level Progress Visibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkProgress {
    /// Total number of chunks in the document.
    pub total_chunks: u32,

    /// Number of chunks fully processed (extracted + embedded).
    pub processed_chunks: u32,

    /// Current chunk index being processed (0-based).
    pub current_chunk_index: u32,

    /// Preview of current chunk content (first 50 chars).
    pub current_chunk_preview: String,

    /// Average time to process a single chunk (milliseconds).
    pub avg_chunk_time_ms: f64,

    /// Estimated time remaining (seconds).
    pub eta_seconds: u64,

    /// Total input tokens consumed so far.
    pub tokens_in: u64,

    /// Total output tokens consumed so far.
    pub tokens_out: u64,

    /// Running cost estimate (USD).
    pub cost_usd: f64,
}
```

### 2. Enhanced TaskProgress

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub current_step: String,
    pub total_steps: u32,
    pub percent_complete: u8,

    /// NEW: Chunk-level progress for document processing.
    /// @implements SPEC-001/Objective-A
    pub chunk_progress: Option<ChunkProgress>,
}
```

### 3. PipelineEvent Enhancement

```rust
pub enum PipelineEvent {
    Log(PipelineMessage),
    Progress { ... },
    StateChange { ... },

    /// NEW: Chunk-level progress update for a document.
    /// @implements SPEC-001/Objective-A
    ChunkProgress {
        document_id: String,
        task_id: String,
        chunk_index: u32,
        total_chunks: u32,
        chunk_preview: String,
        time_ms: u64,
        eta_seconds: u64,
        tokens_in: u64,
        tokens_out: u64,
        cost_usd: f64,
    },
}
```

### 4. Progress Callback in Pipeline

```rust
/// Progress callback type for chunk-level updates.
pub type ChunkProgressCallback = Box<dyn Fn(ChunkProgressUpdate) + Send + Sync>;

/// Update sent via progress callback.
pub struct ChunkProgressUpdate {
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub chunk_preview: String,
    pub elapsed_ms: u64,
    pub input_tokens: usize,
    pub output_tokens: usize,
}

impl Pipeline {
    /// Process document with chunk-level progress callback.
    pub async fn process_with_progress(
        &self,
        document_id: &str,
        content: &str,
        on_chunk_progress: Option<ChunkProgressCallback>,
    ) -> Result<ProcessingResult>
}
```

---

## Integration Points

### Frontend Changes Required

**File**: `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx`

**Current (WRONG)**:

```tsx
<div>Chunking → Extracting → Embedding → Indexing</div>
```

**Required (CORRECT)**:

```tsx
<div>
  <ProgressBar
    value={chunk_progress.processed_chunks / chunk_progress.total_chunks}
  />
  <span>
    {chunk_progress.processed_chunks}/{chunk_progress.total_chunks} chunks
  </span>
  <span>ETA: {formatTime(chunk_progress.eta_seconds)}</span>
</div>
```

---

## Risk Assessment

| Risk                  | Mitigation                                           |
| --------------------- | ---------------------------------------------------- |
| Breaking existing API | ChunkProgress is OPTIONAL field, backward compatible |
| Performance overhead  | Progress callback is lightweight, <1ms per chunk     |
| WebSocket flooding    | Throttle events to max 2/second per document         |

---

## Decision Point

**Proceed with P0**: Add `ChunkProgress` to `TaskProgress` struct, then:

1. Update task storage to persist chunk progress
2. Update API to expose chunk progress
3. Update frontend to consume chunk progress

This provides MAXIMUM user value with MINIMUM code changes.
