# OODA Iteration 01 - OBSERVE

## Mission

Integrate `process_with_resilience` into API handler for production use, add metrics/telemetry for tracking chunk failure rates, implement retry queue for failed chunks, and improve UX/UI for real-time feedback.

## Current State Analysis

### Backend Processing Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         DOCUMENT PROCESSING FLOW                             │
└─────────────────────────────────────────────────────────────────────────────┘

                      ┌─────────────────────────────┐
                      │    POST /api/v1/documents    │
                      └──────────────┬──────────────┘
                                     │
                    ┌────────────────┴────────────────┐
                    ▼                                 ▼
         ┌────────────────────┐           ┌────────────────────┐
         │  async_processing  │           │  async_processing  │
         │      = false       │           │      = true        │
         │   (SYNCHRONOUS)    │           │   (ASYNCHRONOUS)   │
         └─────────┬──────────┘           └─────────┬──────────┘
                   │                                 │
                   ▼                                 ▼
    ┌──────────────────────────────┐   ┌──────────────────────────────┐
    │ documents.rs:upload_document │   │  processor.rs:process_text   │
    │ workspace_pipeline.process() │   │  _insert() via TaskProcessor │
    │     ↓                        │   │       ↓                      │
    │ USES: pipeline.process()     │   │ USES: pipeline.process_with  │
    │      (NO RESILIENCE)         │   │       _progress()            │
    └──────────────────────────────┘   └──────────────────────────────┘
```

### Key Files & Current Implementation

#### 1. processor.rs (Async Path)

- **Location**: `edgequake/crates/edgequake-api/src/processor.rs`
- **Handler**: `DocumentTaskProcessor::process_text_insert()`
- **Current Call**: `pipeline.process_with_progress()` (line ~631)
- **Issue**: Uses progress callback but NOT resilient extraction

```rust
// CURRENT CODE (processor.rs:628-632)
let result = match pipeline
    .process_with_progress(&document_id, &data.text, Some(chunk_progress_callback))
    .await
```

#### 2. documents.rs (Sync Path)

- **Location**: `edgequake/crates/edgequake-api/src/handlers/documents.rs`
- **Handler**: `upload_document()` (line ~675)
- **Current Call**: `workspace_pipeline.process()` wrapped in timeout
- **Issue**: No resilience, no chunk-level progress

```rust
// CURRENT CODE (documents.rs:675-677)
let result = tokio::time::timeout(
    std::time::Duration::from_secs(SYNC_PROCESSING_TIMEOUT_SECS),
    workspace_pipeline.process(&document_id, &request.content),
)
```

#### 3. pipeline.rs (Available Methods)

- **Location**: `edgequake/crates/edgequake-pipeline/src/pipeline.rs`
- **Methods Available**:
  1. `process()` - Basic extraction, fails on any chunk error
  2. `process_with_progress()` - Progress callback, fails on any chunk error
  3. `process_with_resilience()` - **NEW** - MAP-REDUCE pattern with partial results

```rust
// process_with_resilience signature (pipeline.rs:1593-1597)
pub async fn process_with_resilience(
    &self,
    document_id: &str,
    content: &str,
    progress_callback: Option<ChunkProgressCallback>,
) -> Result<ProcessingResult>
```

### Resilience Types Available (error.rs)

```rust
/// Outcome of extracting a single chunk
pub enum ChunkExtractionOutcome {
    Success { chunk_index: usize, result: ExtractionResult },
    Failed(ChunkFailure),
}

/// Details of a chunk extraction failure
pub struct ChunkFailure {
    pub chunk_index: usize,
    pub chunk_id: String,
    pub error: String,
    pub retry_attempts: u32,
    pub was_timeout: bool,
    pub processing_time_ms: u64,
}

/// Stats now include failure tracking
pub struct ProcessingStats {
    pub successful_chunks: usize,
    pub failed_chunks: usize,
    pub chunk_errors: Option<Vec<ChunkErrorInfo>>,
    // ... other fields
}
```

### Frontend WebSocket Events

The frontend already supports chunk-level progress via `ChunkProgress` event:

```typescript
// types/ingestion.ts
interface ChunkProgressEvent {
  type: "ChunkProgress";
  data: {
    document_id: string;
    task_id: string;
    chunk_index: number;
    total_chunks: number;
    chunk_preview: string;
    time_ms: number;
    eta_seconds: number;
    tokens_in: number;
    tokens_out: number;
    cost_usd: number;
  };
}
```

**Missing**: No event for chunk failures / partial success notification.

### WebSocket Handler (websocket.rs)

Currently emits via `ProgressBroadcaster`:
- `Connected` / `StatusSnapshot`
- `DocumentProgress` / `DocumentCompleted` / `DocumentFailed`
- `ChunkProgress` (via emit_chunk_progress)

**Missing**: `ChunkFailed` event for individual chunk failures.

### Database Schema

No table currently exists for storing failed chunks for retry.

## Integration Points Identified

| Component | File | Current Method | Target Method | Change Type |
|-----------|------|----------------|---------------|-------------|
| Async processor | processor.rs | process_with_progress | process_with_resilience | Simple swap |
| Sync handler | documents.rs | process | process_with_resilience | Simple swap |
| WebSocket events | websocket.rs | ChunkProgress only | + ChunkFailed | New event |
| PipelineState | pipeline_state.rs | emit_chunk_progress | + emit_chunk_failure | New method |
| Frontend types | ingestion.ts | ChunkProgressEvent | + ChunkFailureEvent | New type |
| Frontend hook | use-chunk-progress.ts | ChunkProgressState | + failedChunks | New field |
| Database | N/A | N/A | failed_chunks table | New migration |
| API | documents.rs | N/A | POST /retry-chunks | New endpoint |
| Metrics | N/A | N/A | chunk_success/failure | New counters |

## Observations Summary

1. **process_with_resilience** is fully implemented in pipeline.rs but NOT CALLED anywhere
2. **Async path** (processor.rs) is the easier integration - already uses progress callback
3. **Sync path** (documents.rs) also needs migration
4. **Frontend** already handles chunk progress, needs failure events
5. **No retry queue** exists - need to design schema
6. **No metrics** for chunk success/failure rates

## Next Steps (Orient Phase)

1. Prioritize: Async path (processor.rs) first as it handles most documents
2. Design ChunkFailed event structure
3. Design failed_chunks table schema
4. Plan frontend UI for showing failed chunks
