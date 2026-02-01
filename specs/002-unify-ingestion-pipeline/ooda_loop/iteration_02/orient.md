# Iteration 02 - Orient

**Mission File**: `./specs/002-unify-ingestion-pipeline.md`

**Date**: 2026-02-01

---

## First Principles Analysis

### Core Problem

The current system has **two separate progress tracking mechanisms**:
1. PDF: Uses `PipelineProgressCallback` with `PipelinePhase`
2. Text: No intermediate progress, only status string in metadata

### First Principle: Single Source of Truth

Both PDF and Markdown should:
1. Store progress in the **same format**
2. Use the **same stage enum**
3. Be queryable via the **same API endpoint**

### First Principle: Separation of Concerns

- **Storage**: Where progress is persisted
- **Emission**: How progress updates are broadcast
- **Display**: How frontend shows progress

---

## Gap Analysis

### GAP-02: Document Metadata Lacks Structured Status

**Current**: `status: "processing"` (arbitrary string)

**Required**:
```json
{
  "id": "doc-123",
  "source_type": "pdf",
  "current_stage": "extracting",
  "stage_progress": 0.45,
  "stage_message": "Extracting entities from chunk 5/12"
}
```

### GAP-03: Pipeline Has No Stage Callback

Looking at `run_ingestion_pipeline()`:

```rust
// edgequake-api/src/processor.rs (conceptual)
async fn run_ingestion_pipeline(content: &str, ...) -> Result<...> {
    // No callback mechanism for stage updates
    let chunks = chunker.chunk(content);
    let extractions = extractor.extract(chunks);
    let merged = merger.merge(extractions);
    // ...
}
```

**Required**: Add callback parameter for stage updates.

### GAP-04: TrackStatusResponse Needs UnifiedStage

Current `TrackStatusResponse` returns:
- `documents: Vec<DocumentSummary>` with string status
- `status_counts: StatusCounts` (pending, processing, completed, failed)

**Required**: Add per-document stage info:
```rust
pub struct DocumentSummary {
    // Existing fields...
    pub source_type: Option<SourceType>,
    pub current_stage: Option<String>,  // UnifiedStage as string
    pub stage_progress: Option<f32>,
    pub stage_message: Option<String>,
}
```

---

## Solution Design

### Approach: Unified Progress in Metadata

Store progress as part of document metadata in KV storage:

```json
{
  "id": "doc-123",
  "title": "Research Paper",
  "status": "processing",
  
  "source_type": "pdf",
  "current_stage": "extracting",
  "stage_progress": 0.45,
  "stage_message": "Extracting entities (chunk 5/12)",
  "stages_completed": ["uploading", "converting", "preprocessing", "chunking"],
  "error": null
}
```

### Benefits

1. **No new storage** - Uses existing KV storage
2. **Backward compatible** - `status` field still works
3. **Query-able** - Existing `get_track_status` works
4. **Simple** - No new abstractions

### Progress Update Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     UNIFIED PROGRESS UPDATE FLOW                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  1. Upload Handler                                                      │
│     ├─ Set source_type, current_stage = "uploading"                     │
│     └─ For PDF: current_stage = "converting" after store                │
│                                                                         │
│  2. Task Runner (background)                                            │
│     ├─ Before each stage: update_document_stage(doc_id, stage)          │
│     │                                                                   │
│     │  update_document_stage:                                           │
│     │    1. Read metadata from KV                                       │
│     │    2. Set current_stage, stage_message                            │
│     │    3. Write metadata to KV                                        │
│     │    4. Broadcast WebSocket event                                   │
│     │                                                                   │
│     └─ On complete/error: update_document_stage(doc_id, completed/failed)
│                                                                         │
│  3. Frontend polls GET /documents/track/{id}                            │
│     └─ Gets current_stage for each document                             │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Implementation Plan

#### Step 1: Add `source_type` to Document Metadata

When creating document:
```rust
let doc_metadata = serde_json::json!({
    "id": document_id,
    "source_type": if is_pdf { "pdf" } else { "markdown" },
    "current_stage": "uploading",
    // ... existing fields
});
```

#### Step 2: Create `update_document_stage` Function

```rust
async fn update_document_stage(
    kv_storage: &Arc<dyn KVStorage>,
    document_id: &str,
    stage: UnifiedStage,
    message: Option<&str>,
) -> Result<(), ApiError> {
    let key = format!("{}-metadata", document_id);
    let mut metadata = kv_storage.get_by_ids(&[key.clone()]).await?;
    
    // Update stage fields
    if let Some(obj) = metadata.get_mut(0).and_then(|v| v.as_object_mut()) {
        obj.insert("current_stage".into(), stage.to_string().into());
        obj.insert("updated_at".into(), Utc::now().to_rfc3339().into());
        if let Some(msg) = message {
            obj.insert("stage_message".into(), msg.into());
        }
    }
    
    kv_storage.upsert(&[(key, metadata[0].clone())]).await?;
    Ok(())
}
```

#### Step 3: Update Pipeline Callbacks

In `run_ingestion_pipeline`, add stage callbacks:
```rust
// Before chunking
callback.on_stage(UnifiedStage::Chunking, "Splitting into chunks");

// Before extraction
callback.on_stage(UnifiedStage::Extracting, format!("Processing chunk 1/{}", total));
```

#### Step 4: Update Response Types

Add fields to `DocumentSummary`:
```rust
pub struct DocumentSummary {
    // Existing fields...
    
    /// Document source type (pdf, markdown, text)
    #[schema(example = "pdf")]
    pub source_type: Option<String>,
    
    /// Current ingestion stage
    #[schema(example = "extracting")]
    pub current_stage: Option<String>,
    
    /// Progress within current stage (0.0-1.0)
    #[schema(example = 0.45)]
    pub stage_progress: Option<f32>,
    
    /// Human-readable stage message
    #[schema(example = "Extracting entities from chunk 5/12")]
    pub stage_message: Option<String>,
}
```

---

## Risk Assessment

### Risk: Performance of Frequent Metadata Updates

**Concern**: Updating metadata for every chunk could be slow.

**Mitigation**: 
- Batch updates (every N chunks)
- Use stage-level updates (not chunk-level)
- Rate-limit WebSocket broadcasts

### Risk: Breaking Existing Status Logic

**Concern**: Code that checks `status == "completed"` might break.

**Mitigation**:
- Keep `status` field for backward compatibility
- `status` = "completed" when `current_stage` = "completed"
- Add migration logic for existing documents

---

## Decision Points

1. **Frequency of stage updates**: Per-stage or per-chunk?
   → Per-stage (10 updates per document max)

2. **WebSocket event format**: Reuse existing or new type?
   → Add new `ProgressEvent::StageUpdate` variant

3. **Backward compatibility**: Keep `status` field?
   → Yes, derive from `current_stage`
