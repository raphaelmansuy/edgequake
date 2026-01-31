# Iteration 01 - Observe

## Re-read Mission

**Mission file**: `/Users/raphaelmansuy/Github/03-working/edgequake/specs/002-fix-pdf-processing.md`

**Goal**: Test end-to-end PDF upload → Markdown → KG extraction → Processing completion

## Current State

### Test Scenario

- **PDF File**: `zz-explore/AgenticPlatformReference Architecture.pdf` (480KB)
- **Frontend**: http://localhost:3000/documents
- **Backend**: http://localhost:8080 (healthy, PostgreSQL mode, Ollama provider)

### Network Request Analysis

Using Playwright MCP, I captured the following network traffic during PDF upload:

```
[POST] /api/v1/documents/pdf => [200] OK          ✅ Upload succeeded
[GET]  /api/v1/documents/pdf/progress/{track_id} => [404] Not Found  ❌ PROBLEM
[GET]  /api/v1/documents/track/{track_id} => [200] OK  ✅ Track works
```

### Root Cause Analysis

```ascii
UPLOAD FLOW (Current - BROKEN)
==============================

Frontend                         Backend                          PipelineState
    |                               |                                   |
    |--- POST /documents/pdf ------>|                                   |
    |                               |--- Store PDF ------------------>  |
    |                               |--- Create Task -------------->    |
    |<---- 200 OK (track_id) -------|                                   |
    |                               |                                   |
    |--- GET /pdf/progress/{id} --->|--- get_pdf_progress() --------->  |
    |                               |<-------- None (not initialized) --|
    |<---- 404 Not Found -----------|                                   |
    |                               |                                   |
    |                               |    [LATER: Task starts]           |
    |                               |    on_extraction_start() -------->|
    |                               |    start_pdf_progress() --------->|
    |                               |                                   |
    |--- GET /pdf/progress/{id} --->|--- get_pdf_progress() --------->  |
    |                               |<-------- Some(progress) ---------|
    |<---- 200 OK (progress) -------|                                   |
```

### Code Location

1. **Upload handler**: `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs:309-476`
   - Returns `track_id` in response (line 468)
   - Does NOT initialize progress tracking

2. **Progress getter**: `edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs:732-752`
   - Calls `state.pipeline_state.get_pdf_progress(&track_id)`
   - Returns 404 if `None`

3. **Progress initialization**: `edgequake/crates/edgequake-api/src/pipeline_progress_callback.rs:161-172`
   - Called from `on_extraction_start()` callback
   - Too late! Frontend is already polling

4. **PipelineState storage**: `edgequake/crates/edgequake-tasks/src/pipeline_state.rs:565-582`
   - `start_pdf_progress()` initializes in-memory HashMap
   - `get_pdf_progress()` returns from HashMap

### Evidence: Console Errors

```
[ERROR] Failed to load resource: the server respon...ocuments/pdf/progress/upload_1769869170018_fn1nmdwt:0
```

Frontend uses `upload_` prefixed track_id but progress is never initialized for it.

## Data Gathered

| Component         | File                          | Line    | Status                                         |
| ----------------- | ----------------------------- | ------- | ---------------------------------------------- |
| Upload handler    | pdf_upload.rs                 | 309-476 | Returns track_id without initializing progress |
| Progress endpoint | pdf_upload.rs                 | 732-752 | Returns 404 when progress not in HashMap       |
| Progress init     | pipeline_progress_callback.rs | 161-172 | Called too late (on extraction start)          |
| PipelineState     | pipeline_state.rs             | 565-582 | In-memory HashMap, not pre-populated           |

## Dependencies

```
upload_pdf_document()
  └─> returns track_id (from options.track_id or generated)
  └─> does NOT call start_pdf_progress()

on_extraction_start() [LATER, when task runs]
  └─> tokio::spawn(start_pdf_progress())  [race condition]
```

## Immediate Gap

**Gap**: No progress initialization on upload → 404 on immediate progress poll

**Impact**: Frontend shows error, user experience degraded
