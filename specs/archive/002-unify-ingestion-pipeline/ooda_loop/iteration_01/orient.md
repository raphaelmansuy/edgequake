# Iteration 01 - Orient

**Mission File**: `./specs/002-unify-ingestion-pipeline.md`

**Date**: 2026-02-01

---

## Gap Analysis

### GAP-01: Inconsistent Pipeline Stages

**Current State**:

- Backend: 9 stages (Preprocessing, Chunking, Extracting, Gleaning, Merging, Summarizing, Embedding, Storing, Finalizing)
- Frontend: 6 states (pending, processing, chunking, extracting, embedding, indexing)
- PDF: 3 phases (PdfConversion, EntityExtraction, VectorIndexing)

**Impact**: Users see different progress for PDF vs Markdown. Confusion.

**First Principles Solution**:
Define a **single unified stage enum** used by all paths:

```
┌────────────────────────────────────────────────────────────────────────┐
│                    UNIFIED INGESTION STAGES                            │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  Stage           │ PDF Flow           │ Markdown Flow                  │
│  ────────────────┼────────────────────┼────────────────────────────────│
│  uploading       │ ✓ File upload      │ ✓ Content upload               │
│  converting      │ ✓ PDF → Markdown   │ - (skip)                       │
│  preprocessing   │ ✓ Validation       │ ✓ Validation                   │
│  chunking        │ ✓ Split chunks     │ ✓ Split chunks                 │
│  extracting      │ ✓ LLM extraction   │ ✓ LLM extraction               │
│  gleaning        │ ✓ Re-extraction    │ ✓ Re-extraction                │
│  merging         │ ✓ Graph merge      │ ✓ Graph merge                  │
│  summarizing     │ ✓ Descriptions     │ ✓ Descriptions                 │
│  embedding       │ ✓ Vectors          │ ✓ Vectors                      │
│  storing         │ ✓ Persist          │ ✓ Persist                      │
│  completed       │ ✓ Done             │ ✓ Done                         │
│  failed          │ ✓ Error            │ ✓ Error                        │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

### GAP-02: Separate Upload Handlers

**Current State**:

- `pdf_upload.rs` (1200 lines) handles PDF
- `documents.rs` (4036 lines) handles text/markdown

**Impact**: Code duplication, inconsistent behavior.

**First Principles Solution**:
Keep separate endpoints but **share a unified ingestion task type**:

```
POST /api/v1/documents (multipart)
  → Detect file type
  → Store source file
  → Create unified IngestionTask
  → Return unified response with track_id
```

OR keep endpoints but create **shared pipeline orchestrator**:

```rust
// New: unified_pipeline.rs
pub async fn start_unified_ingestion(
    source: IngestionSource,  // PDF or Markdown
    workspace_id: Uuid,
    options: IngestionOptions,
) -> Result<IngestionTask, Error>
```

### GAP-03: Progress Event Duplication

**Current State**:

- `PipelineProgressCallback` for PDF (pipeline_progress_callback.rs)
- Generic progress for text (embedded in pipeline)
- Two broadcast paths (PipelineState + ProgressBroadcaster)

**Impact**: Inconsistent progress UI, double event emission.

**First Principles Solution**:
Create **single IngestionProgress type** used everywhere:

```rust
#[derive(Clone, Serialize)]
pub struct IngestionProgress {
    pub track_id: String,
    pub document_id: Option<String>,
    pub source_type: SourceType,  // Pdf | Markdown | Text
    pub current_stage: IngestionStage,
    pub stages: Vec<StageProgress>,
    pub overall_progress: f32,
    pub message: String,
    pub error: Option<IngestionError>,
}
```

### GAP-04: Error Structure Inconsistency

**Current State**:

- PDF: `errors: Option<serde_json::Value>` (PdfStatusResponse)
- Document: `error_message: Option<String>` (document metadata)

**First Principles Solution**:
Unified error structure:

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct IngestionError {
    pub code: String,           // e.g., "PDF_CONVERSION_FAILED"
    pub message: String,        // Human-readable
    pub stage: IngestionStage,  // Where it failed
    pub details: Option<Value>, // Extra context (page number, etc.)
    pub recoverable: bool,      // Can user retry?
}
```

---

## Proposed Architecture

### Unified Ingestion Flow

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                         UNIFIED INGESTION ARCHITECTURE                        │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────────────┐   │
│  │ PDF Upload      │    │ Markdown Upload │    │ Text Upload             │   │
│  │ /documents/pdf  │    │ /documents      │    │ /documents              │   │
│  └────────┬────────┘    └────────┬────────┘    └────────┬────────────────┘   │
│           │                      │                      │                    │
│           ▼                      ▼                      ▼                    │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │                    UNIFIED INGESTION COORDINATOR                       │  │
│  │                                                                        │  │
│  │  • Detect source type (PDF, Markdown, Text)                            │  │
│  │  • Store source file/content                                           │  │
│  │  • Create IngestionTask with unified stages                            │  │
│  │  • Emit unified IngestionProgress events                               │  │
│  │                                                                        │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                    │                                         │
│                                    ▼                                         │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │                         INGESTION PIPELINE                             │  │
│  │                                                                        │  │
│  │   [uploading] → [converting?] → [preprocessing] → [chunking]           │  │
│  │        ↓                             ↓               ↓                 │  │
│  │   [extracting] → [gleaning] → [merging] → [summarizing]                │  │
│  │        ↓              ↓           ↓            ↓                       │  │
│  │   [embedding] → [storing] → [completed/failed]                         │  │
│  │                                                                        │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                    │                                         │
│                                    ▼                                         │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │                      UNIFIED PROGRESS EVENTS                           │  │
│  │                                                                        │  │
│  │  • IngestionProgress → PipelineState (REST polling)                    │  │
│  │  • IngestionProgress → ProgressBroadcaster (WebSocket)                 │  │
│  │  • IngestionProgress → Document status update (DB)                     │  │
│  │                                                                        │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Risk Assessment

### Approach A: Create Unified Coordinator Module

**Benefits**:

- Clean separation of concerns
- Minimal changes to existing handlers
- Easy to test in isolation

**Risks**:

- Another abstraction layer
- Existing handlers still need modification
- Migration complexity

**Effort**: Medium (2-3 days)

### Approach B: Refactor Existing Handlers

**Benefits**:

- Direct fix at source
- No new modules
- Familiar codebase

**Risks**:

- Large files become larger
- Harder to maintain
- Higher regression risk

**Effort**: Medium-High (3-4 days)

### Approach C: Hybrid - Shared Types + Minimal Coordinator

**Benefits**:

- Shared types reduce duplication
- Small coordinator for orchestration
- Existing handlers remain stable

**Risks**:

- Two places to update
- Need careful interface design

**Effort**: Low-Medium (1-2 days)

---

## Recommendation

**Choose Approach C: Hybrid**

1. Create shared types in `edgequake-pipeline`:
   - `IngestionStage` (unified enum)
   - `IngestionProgress` (shared struct)
   - `IngestionError` (unified error)

2. Create minimal coordinator in `edgequake-api`:
   - `unified_ingestion.rs` - orchestrates progress emission
   - Reuses existing handlers' logic

3. Update frontend:
   - Single `statusConfig` aligned with `IngestionStage`
   - Unified progress component

---

## Questions Resolved

1. **Should PDF and Markdown share a single upload endpoint?**
   → No, keep separate endpoints but unify internal processing.

2. **How to unify stage naming?**
   → Create `IngestionStage` enum in `edgequake-pipeline`.

3. **Should error structures be unified?**
   → Yes, create `IngestionError` struct.

4. **What should unified progress events look like?**
   → `IngestionProgress` with source_type, stages, and unified error.
