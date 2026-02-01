# Iteration 02 - Observe

**Mission File**: `./specs/002-unify-ingestion-pipeline.md`

**Date**: 2026-02-01

---

## Re-Read Mission Objectives

1. **Unified Ingestion Flow**: PDF and Markdown share common KG pipeline
2. **Unified Status Tracking**: Same stages displayed for both types
3. **Unified Pipeline Display**: Business-informative, clear errors
4. **Code Quality**: SRP, DRY, KISS

---

## Progress from Iteration 01

✅ Created `UnifiedStage` enum in backend (`ingestion_types.rs`)
✅ Created `SourceType` enum (Pdf, Markdown, Text)
✅ Created `IngestionProgress` and `IngestionError` structs
✅ Updated frontend `IngestionStage` type to include all stages
✅ Updated `StatusBadge` with icons/colors for all stages
✅ Updated `StageIndicator` labels

---

## Current State Analysis

### Gap: Backend Still Uses Old Progress System

Looking at the code, I found:

1. **PDF Progress Callback** ([pipeline_progress_callback.rs#L200-350](edgequake/crates/edgequake-api/src/pipeline_progress_callback.rs#L200-L350)):
   - Uses ad-hoc phase strings: `"extraction"`, `"extracted"`, `"extraction_error"`, `"complete"`
   - Emits `PdfPageProgress` events (custom structure)
   - Does NOT use `UnifiedStage`

2. **Track Status Response** ([documents.rs#L2791-2900](edgequake/crates/edgequake-api/src/handlers/documents.rs#L2791-L2900)):
   - Returns `DocumentSummary` with `status: Option<String>`
   - Status is arbitrary string from metadata
   - No structured stage information

3. **Document Upload** ([documents.rs#L430-600](edgequake/crates/edgequake-api/src/handlers/documents.rs#L430-L600)):
   - Sets `status: "pending"` or `"processing"` initially
   - Updates status to `"completed"` or `"failed"` at end
   - No intermediate stage updates

### Gap: PipelineState Uses Different Phase Type

**PipelinePhase** from `edgequake-tasks/src/progress.rs`:

```rust
pub enum PipelinePhase {
    PdfConversion,
    EntityExtraction,
    VectorIndexing,
}
```

This is used by the PDF progress system but:
- Only 3 phases (not enough granularity)
- Doesn't align with `UnifiedStage` (12 stages)
- Text uploads don't use it at all

### Gap: Document Metadata Status Field

Documents stored in KV storage have status as arbitrary string:

```json
{
  "id": "doc-123",
  "status": "processing",  // arbitrary string
  "track_id": "upload_xxx"
}
```

**Need**: Store `source_type` and `current_stage` in metadata.

---

## Data Flow Analysis

### Current PDF Upload Flow

```
┌─────────────┐     ┌────────────────┐     ┌─────────────────┐
│ PDF Upload  │────►│ Store PDF      │────►│ Create Task     │
│ POST /pdf   │     │ (PostgreSQL)   │     │ (TaskType::Pdf) │
└─────────────┘     └────────────────┘     └────────┬────────┘
                                                    │
                                           ┌────────▼────────┐
                                           │ Task Runner     │
                                           │ (background)    │
                                           └────────┬────────┘
                                                    │
                    ┌───────────────────────────────┼───────────────────┐
                    │                               │                   │
           ┌────────▼────────┐           ┌─────────▼─────────┐         │
           │ PDF Extraction  │           │ PipelineProgress   │         │
           │ (edgequake-pdf) │──────────►│ Callback           │         │
           └────────┬────────┘           │ (emits progress)   │         │
                    │                    └─────────┬──────────┘         │
                    │                              │                    │
           ┌────────▼────────┐           ┌─────────▼──────────┐        │
           │ KG Pipeline     │           │ ProgressBroadcaster │        │
           │ (run_ingestion) │           │ (WebSocket)         │        │
           └────────┬────────┘           └────────────────────┘        │
                    │                                                   │
           ┌────────▼────────┐                                          │
           │ Update metadata │◄─────────────────────────────────────────┘
           │ status="indexed"│
           └─────────────────┘
```

### Current Text Upload Flow

```
┌─────────────┐     ┌────────────────┐     ┌─────────────────┐
│ Text Upload │────►│ Store Metadata │────►│ Create Task     │
│ POST /docs  │     │ (KV Storage)   │     │ (TaskType::Text)│
└─────────────┘     └────────────────┘     └────────┬────────┘
                                                    │
                                           ┌────────▼────────┐
                                           │ Task Runner     │
                                           │ (background)    │
                                           └────────┬────────┘
                                                    │
                                           ┌────────▼────────┐
                                           │ KG Pipeline     │
                                           │ (run_ingestion) │
                                           └────────┬────────┘
                                                    │
                                           ┌────────▼────────┐
                                           │ Update metadata │
                                           │ status="indexed"│
                                           └─────────────────┘

                    ⚠️ NO INTERMEDIATE PROGRESS UPDATES!
```

### Required Unified Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         UNIFIED PROGRESS FLOW                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  PDF/Markdown Upload                                                    │
│         │                                                               │
│         ▼                                                               │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                   UNIFIED PROGRESS TRACKER                       │   │
│  │                                                                  │   │
│  │  track_id: "upload_xxx"                                          │   │
│  │  source_type: Pdf | Markdown                                     │   │
│  │  current_stage: UnifiedStage                                     │   │
│  │  stages: [StageProgress; N]                                      │   │
│  │                                                                  │   │
│  │  Stage transitions:                                              │   │
│  │    uploading → converting? → preprocessing → chunking →         │   │
│  │    extracting → gleaning → merging → summarizing →              │   │
│  │    embedding → storing → completed                              │   │
│  │                                                                  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│         │                                                               │
│         ├──────────────► REST API (polling)                            │
│         │                GET /documents/track/{id}                     │
│         │                                                               │
│         └──────────────► WebSocket (push)                              │
│                          ProgressEvent::StageUpdate                    │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Files to Modify

### Backend

| File | Change |
|------|--------|
| `documents_types.rs` | Add `source_type` and `current_stage` to `TrackStatusResponse` |
| `documents.rs` | Store `source_type` in metadata, update stage during pipeline |
| `pdf_upload.rs` | Store `source_type: "pdf"` in metadata |
| `pipeline_progress_callback.rs` | Use `UnifiedStage` instead of string phases |

### Frontend

| File | Change |
|------|--------|
| `types/index.ts` | Update `Document` type with `source_type` and `current_stage` |
| `ingestion-progress-panel.tsx` | Display unified stages for both types |

---

## Key Questions

1. **Where should unified progress be stored?**
   - Option A: KV Storage (current approach)
   - Option B: Dedicated progress table
   - Option C: In-memory with persistence

2. **How to trigger stage updates from pipeline?**
   - Need callback mechanism from `run_ingestion_pipeline()`
   - Currently no callback for intermediate stages

---

## Next Steps

1. Orient: Design unified progress storage and callback mechanism
2. Decide: Choose implementation approach
3. Act: Implement changes
