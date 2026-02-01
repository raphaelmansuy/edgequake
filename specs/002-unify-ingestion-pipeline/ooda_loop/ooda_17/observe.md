# OODA-17: Observe - PDF Progress Phase Tracking

## Date: 2026-02-01

## Mission Reference
- Spec: `./specs/002-unify-ingestion-pipeline.md`
- Objective: Unified Status Tracking for PDF and Markdown documents

## Problem Statement

User reported that PDF uploads don't show progress status updates during pipeline processing while Markdown uploads do. The issue manifested as:

> "I have downloaded → but I don't see progress if handling in documents → I want to see the status moving with the phase of the pipeline as with the markdown upload and ingestion"

## Observations

### 1. Current Backend Architecture

```
PDF Upload Flow:
┌─────────────┐      ┌─────────────────────────┐      ┌─────────────────────┐
│ PDF Handler │ ──► │ PipelineProgressCallback │ ──► │ process_text_insert │
└─────────────┘      └─────────────────────────┘      └─────────────────────┘
       │                        │                              │
       ▼                        ▼                              ▼
   Upload Phase           PdfConversion              Chunking/Embedding/
   (tracked ✓)            Phase (tracked ✓)         Extraction/GraphStorage
                                                    (NOT tracked ✗)
```

### 2. Root Cause Analysis

**File: `edgequake-api/src/pipeline_progress_callback.rs`**

The `PipelineProgressCallback` only tracks the `PdfConversion` phase:

```rust
impl PipelineProgressCallback {
    // Only updates PdfConversion phase
    self.pipeline_state
        .start_pdf_phase(&track_id, PipelinePhase::PdfConversion, ...)
}
```

**File: `edgequake-api/src/processor.rs`**

The `process_text_insert()` function handles post-PDF-conversion processing but did NOT call PDF phase tracking methods for:
- Chunking phase
- Extraction phase
- Embedding phase
- GraphStorage phase

### 3. Pipeline Phase Model

From `edgequake-tasks/src/progress.rs`:

```rust
pub enum PipelinePhase {
    Upload,        // Phase 1: File upload to server
    PdfConversion, // Phase 2: PDF → Markdown conversion
    Chunking,      // Phase 3: Document chunking
    Extraction,    // Phase 4: Entity/relationship extraction
    Embedding,     // Phase 5: Vector embedding generation
    GraphStorage,  // Phase 6: Graph/vector storage
}
```

### 4. Existing Progress Tracking Methods

From `edgequake-tasks/src/pipeline_state.rs`:

- `start_pdf_phase(&track_id, phase, total)` - Begin a phase
- `update_pdf_phase(&track_id, phase, completed)` - Update progress
- `complete_pdf_phase(&track_id, phase)` - Complete a phase
- `get_pdf_progress(&track_id)` - Get current progress

These methods exist but were not being called for phases after PdfConversion.

## Test Files Analyzed

| File | Purpose | Status |
|------|---------|--------|
| `processor.rs` | Task processor for document pipeline | Missing PDF phase calls |
| `pipeline_progress_callback.rs` | PDF extraction progress adapter | Only tracks PdfConversion |
| `pipeline_state.rs` | Progress state management | Methods exist, not used |
| `progress.rs` | Progress type definitions | Complete, 6 phases defined |

## Evidence

Backend logs showed document processing but no phase updates after PdfConversion:
- `status: "chunking"` - Updated document status
- `status: "extracting"` - Updated document status  
- `status: "embedding"` - Updated document status
- `status: "completed"` - Final status

But `PdfUploadProgress.phases` was not being updated for these phases.
