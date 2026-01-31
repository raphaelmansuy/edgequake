# Iteration 01: Observe

## Mission Re-Read ✅

- [x] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed objectives:
  1. PDF-to-Markdown conversion via edgequake-pdf BEFORE entity extraction
  2. Detailed 6-phase progress monitoring in web UI
  3. Real-time updates via WebSocket/polling
  4. Error visibility with actionable suggestions
  5. Historical tracking with filter/search
- [x] Current phase: Phase 1 - Architecture & Design (Iterations 1-10)

## Code Analysis

### 1. PDF Upload Handler (`edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs`)

- **Lines**: 1-844
- **Purpose**: HTTP endpoint for PDF upload (POST /api/v1/documents/pdf)
- **Current behavior**:
  1. Parse multipart form data (file, options, track_id)
  2. Validate PDF signature and size
  3. Calculate SHA-256 checksum for deduplication
  4. Store raw PDF in database
  5. Create background task (TaskType::PdfProcessing)
  6. Return response with task_id, pdf_id, estimated_time
- **Key functions**:
  - `upload_pdf_document()` - Main handler (lines 302-478)
  - `create_pdf_processing_task()` - Creates Task with PdfProcessingData (lines 722-777)
  - `estimate_processing_time()` - Simple heuristic (lines 793-803)
- **Dependencies**: edgequake_storage, edgequake_tasks

### 2. Task Processor (`edgequake/crates/edgequake-api/src/processor.rs`)

- **Lines**: 1-1850
- **Purpose**: Background task processing for document ingestion
- **PDF Processing Flow** (lines 1228-1429):
  1. Load PDF from storage by pdf_id
  2. Update status to "processing"
  3. Extract content via edgequake-pdf (vision or text mode)
  4. Store extracted markdown in pdf_documents table
  5. Create document via standard pipeline (text_insert flow)
  6. Link PDF to created document
  7. Update status to "completed"
- **Key insight**: ✅ PDF IS already converted to Markdown via `edgequake-pdf` before entity extraction
- **Missing**: No progress callbacks during PDF extraction phases

### 3. edgequake-pdf Extractor (`edgequake/crates/edgequake-pdf/src/extractor.rs`)

- **Lines**: 1-605
- **Purpose**: Convert PDF bytes to Markdown
- **Key types**:
  - `PdfExtractor` - Main extractor with LLM provider
  - `ExtractionResult` - Full result with page_errors for graceful degradation
  - `PageContent` - Per-page text/markdown
- **Methods**:
  - `extract_to_markdown()` - Simple markdown output
  - `extract_document()` - Returns structured Document IR
  - `extract_full()` - Returns ExtractionResult with page-level details
- **Missing**: ❌ No progress callback mechanism (ProgressCallback trait mentioned in mission spec doesn't exist)

### 4. Task Types (`edgequake/crates/edgequake-tasks/src/types.rs`)

- **Lines**: 1-1119
- **Purpose**: Task data models for background processing
- **Existing progress tracking**:
  - `TaskProgress` - current_step, total_steps, percent_complete
  - `ChunkProgress` - Detailed chunk-level progress with ETA, tokens, cost
- **Key insight**: ✅ ChunkProgress already exists with ETA calculation
- **Missing**: ❌ No PDF-specific phases (only generic chunk processing)

### 5. Worker Pool (`edgequake/crates/edgequake-tasks/src/worker.rs`)

- **Lines**: 1-392
- **Purpose**: Process tasks from queue with retry/backoff
- **Current behavior**: Simple process → success/fail loop
- **Missing**: ❌ No progress update mechanism during task execution

### 6. Frontend Progress Hook (`edgequake_webui/src/hooks/use-ingestion-progress.ts`)

- **Lines**: 1-178
- **Purpose**: Track ingestion progress via WebSocket/polling
- **Current behavior**:
  - Uses WebSocket for real-time updates
  - Falls back to polling when WebSocket unavailable
  - Integrates with useIngestionStore and useCostStore
- **Key insight**: ✅ WebSocket infrastructure already exists
- **Missing**: ❌ No PDF-specific phase display

### 7. Document Manager (`edgequake_webui/src/components/documents/document-manager.tsx`)

- **Lines**: 1-1515
- **Purpose**: Document upload and management UI
- **Current behavior**: Generic progress display, batch tracking
- **Missing**: ❌ No 6-phase pipeline visualization

## Current PDF Upload Flow (ASCII Diagram)

```
User                 Frontend              Backend (pdf_upload.rs)      TaskProcessor           edgequake-pdf
 |                      |                      |                           |                        |
 |-- Upload PDF ------->|                      |                           |                        |
 |                      |-- POST /api/v1/documents/pdf                     |                        |
 |                      |                      |                           |                        |
 |                      |                      |-- validate_pdf()          |                        |
 |                      |                      |-- calculate_checksum()    |                        |
 |                      |                      |-- store_pdf_in_db()       |                        |
 |                      |                      |-- create_task(PdfProcessing)                       |
 |                      |<-- 200 {task_id, pdf_id, status: "processing"}   |                        |
 |                      |                      |                           |                        |
 |<-- Show "Uploading" -|                      |                           |                        |
 |                      |                      |                           |                        |
 |                      |                      |       Worker picks up task|                        |
 |                      |                      |                           |-- load_pdf()           |
 |                      |                      |                           |-- update_status(processing)
 |                      |                      |                           |                        |
 |                      |                      |                           |-- extract_to_markdown()-->|
 |                      |                      |                           |                        |-- parse_pdf()
 |                      |                      |                           |                        |-- apply_processors()
 |                      |                      |                           |                        |-- render_markdown()
 |                      |                      |                           |<-- markdown ------------|
 |                      |                      |                           |                        |
 |                      |                      |                           |-- process_text_insert()
 |                      |                      |                           |   (chunking, embedding, extraction, graph)
 |                      |                      |                           |                        |
 |                      |                      |                           |-- update_status(completed)
 |                      |                      |                           |                        |
 |                      |-- Poll /tracks/{id} -------------------------------->|                   |
 |                      |<-- {status: "indexed"} ------------------------------|                   |
 |<-- Show "Complete!" -|                      |                           |                        |
```

## Key Findings

1. **✅ PDF-to-Markdown conversion DOES use edgequake-pdf** - See processor.rs lines 1284-1359
2. **✅ ChunkProgress exists** with ETA, tokens, cost - See types.rs lines 159-216
3. **✅ WebSocket infrastructure exists** - See use-ingestion-progress.ts
4. **❌ NO progress callbacks in PDF extraction** - extractor.rs has no ProgressCallback trait
5. **❌ NO 6-phase tracking** - Only generic "processing" status
6. **❌ NO real-time progress during extraction** - Task just runs to completion
7. **❌ NO page-by-page progress** for PDF extraction phase

## Questions to Answer Next Iteration

1. Where should ProgressCallback trait be defined? (edgequake-pdf or edgequake-tasks?)
2. How to propagate progress from deep in PDF extraction back to TaskProcessor?
3. Should we use channels, Arc<Mutex<Progress>>, or callback closures?
4. How to estimate total time when page count might be unknown initially?
5. What WebSocket message format should we use for phase updates?
