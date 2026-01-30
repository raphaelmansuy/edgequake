# OODA Iteration 18: OBSERVE

**Date**: 2025-01-28
**Mission Re-Read**: ✅ YES - `/specs/001-improve-ingestion-process.md` lines 1-421

---

## Observation Focus: Chunk-Level Progress Architecture

### Current State Analysis

#### 1. Pipeline Progress Tracking (progress.rs)

**Location**: `edgequake/crates/edgequake-pipeline/src/progress.rs`

**Current Model (INCORRECT for real progress visibility)**:

```
┌──────────────────────────────────────────────────────────────┐
│  IngestionProgress                                           │
├──────────────────────────────────────────────────────────────┤
│  - job_id: String                                            │
│  - document_id: String                                       │
│  - status: IngestionStatus (Pending/Running/Completed/Failed)│
│  - current_stage: PipelineStage                              │
│  - stages: Vec<StageProgress>   ← STAGE-BASED (wrong)        │
│  - completion_percentage: f32   ← Based on stages completed  │
│  - eta_seconds: Option<u64>     ← Never populated            │
└──────────────────────────────────────────────────────────────┘
```

**Key Issue**: `StageProgress` tracks progress PER STAGE, not per chunk:

```rust
pub struct StageProgress {
    pub stage: PipelineStage,
    pub status: StageStatus,
    pub total_items: usize,      // ← Items in this stage (NOT chunks)
    pub completed_items: usize,  // ← Completed in this stage
    pub completion_percentage: f32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}
```

**9 Stages Defined** (too granular, misleading):

1. Preprocessing
2. Chunking
3. Extracting
4. Gleaning
5. Merging
6. Summarizing
7. Embedding
8. Storing
9. Finalizing

#### 2. Pipeline Processing (pipeline.rs)

**Location**: `edgequake/crates/edgequake-pipeline/src/pipeline.rs`

**Actual Processing Flow**:

```
Step 1: Chunk document → chunks: Vec<TextChunk>
Step 2: Extract entities (parallel) → extractions: Vec<ExtractionResult>
Step 3: Generate embeddings (batch) → chunks get embeddings
Step 4: Build lineage (optional)
```

**Critical Finding**: Extraction is done in parallel via `extract_parallel()`:

```rust
async fn extract_parallel(
    &self,
    chunks: &[TextChunk],
    extractor: &Arc<dyn EntityExtractor>,
) -> Result<Vec<ExtractionResult>>
```

Each chunk → one LLM call for extraction. This is where chunk-level progress should be tracked.

**No progress callback**: The `Pipeline.process()` method does NOT take a progress callback - it runs synchronously from the caller's perspective (internally async but no progress reporting).

#### 3. Task System (edgequake-tasks)

**Location**: `edgequake/crates/edgequake-tasks/src/`

**Current Task Progress**:

```rust
pub struct TaskProgress {
    pub current_step: String,    // ← Text description
    pub total_steps: u32,        // ← Generic "steps"
    pub percent_complete: u8,    // ← 0-100%
}
```

**Missing for Chunk-Level Visibility**:

- `total_chunks: u32`
- `processed_chunks: u32`
- `current_chunk_index: u32`
- `current_chunk_preview: String`
- `avg_chunk_time_ms: f64`
- `eta_seconds: u64`
- `tokens_consumed: TokenUsage`

#### 4. Pipeline State (pipeline_state.rs)

**Location**: `edgequake/crates/edgequake-tasks/src/pipeline_state.rs`

**Current Tracking**:

```rust
struct PipelineStateInner {
    is_busy: bool,
    job_name: Option<String>,
    total_documents: u32,      // ← Document-level
    processed_documents: u32,  // ← Document-level
    current_batch: u32,        // ← Batch-level
    total_batches: u32,
    messages: Vec<PipelineMessage>,
    ...
}
```

**Gap**: No chunk-level fields. State tracks documents and batches, NOT chunks within a document.

**Events Emitted**:

```rust
pub enum PipelineEvent {
    Log(PipelineMessage),
    Progress { processed: u32, total: u32, batch: u32, total_batches: u32 },
    StateChange { is_busy: bool, job_name: Option<String> },
}
```

**Missing Event Types**:

- `ChunkProgress { chunk_index, total_chunks, chunk_preview, avg_time_ms, eta_seconds }`
- `DocumentChunkingComplete { total_chunks }`
- `ExtractionProgress { chunks_extracted, total_chunks }`

---

## Findings Summary

### GAPS IDENTIFIED

| Gap ID | Description                                   | Severity |
| ------ | --------------------------------------------- | -------- |
| GAP-A1 | `IngestionProgress` tracks stages, not chunks | HIGH     |
| GAP-A2 | `Pipeline.process()` has no progress callback | HIGH     |
| GAP-A3 | `TaskProgress` lacks chunk-specific fields    | HIGH     |
| GAP-A4 | `PipelineState` tracks documents, not chunks  | HIGH     |
| GAP-A5 | `PipelineEvent` has no chunk-level events     | HIGH     |
| GAP-B1 | No queue visibility in task storage           | MEDIUM   |
| GAP-B2 | No wait time tracking per task                | MEDIUM   |
| GAP-B3 | No throughput calculation                     | MEDIUM   |

### REQUIRED CHANGES

**Backend (Priority Order)**:

1. **Add ChunkProgress struct** to `edgequake-tasks/src/types.rs`:

   ```rust
   pub struct ChunkProgress {
       pub total_chunks: u32,
       pub processed_chunks: u32,
       pub current_chunk_index: u32,
       pub current_chunk_preview: String, // First 50 chars
       pub avg_chunk_time_ms: f64,
       pub eta_seconds: u64,
       pub tokens_in: u64,
       pub tokens_out: u64,
       pub cost_usd: f64,
   }
   ```

2. **Add progress callback to Pipeline.process()**:

   ```rust
   pub async fn process_with_progress<F>(
       &self,
       document_id: &str,
       content: &str,
       progress_callback: F,
   ) -> Result<ProcessingResult>
   where
       F: Fn(ChunkProgressEvent) + Send + Sync
   ```

3. **Add ChunkProgressEvent to PipelineEvent**:

   ```rust
   ChunkProgress {
       document_id: String,
       chunk_index: u32,
       total_chunks: u32,
       chunk_preview: String,
       time_ms: u64,
       eta_seconds: u64,
   }
   ```

4. **Update TaskProgress** to include `chunk_progress: Option<ChunkProgress>`

5. **Add queue metrics** to `TaskStorage` trait:

   ```rust
   async fn get_queue_metrics(&self) -> TaskResult<QueueMetrics>;

   pub struct QueueMetrics {
       pub pending_count: u32,
       pub processing_count: u32,
       pub completed_count: u32,
       pub failed_count: u32,
       pub avg_wait_time_seconds: f64,
       pub avg_processing_time_seconds: f64,
       pub throughput_per_minute: f64,
   }
   ```

---

## Code Locations Requiring Modification

| File                                    | Change Required          | Lines Affected |
| --------------------------------------- | ------------------------ | -------------- |
| `edgequake-tasks/src/types.rs`          | Add ChunkProgress struct | +40 lines      |
| `edgequake-tasks/src/pipeline_state.rs` | Add chunk-level state    | ~50 lines      |
| `edgequake-pipeline/src/pipeline.rs`    | Add progress callback    | ~100 lines     |
| `edgequake-pipeline/src/progress.rs`    | Add chunk tracking       | ~80 lines      |
| `edgequake-api/src/handlers/*.rs`       | Expose new metrics       | ~60 lines      |

---

## Evidence Collected

1. **progress.rs:284-312**: `IngestionProgress` struct definition
2. **progress.rs:358-466**: `ProgressTracker` implementation (stage-based)
3. **pipeline.rs:230-280**: `Pipeline::extract_parallel()` - chunk processing
4. **pipeline_state.rs:68-90**: `PipelineStateInner` - document-level only
5. **types.rs:109-113**: `TaskProgress` - lacks chunk fields

---

## Next Step

ORIENT → Analyze gaps and prioritize which changes will provide maximum visibility improvement with minimum code changes.
