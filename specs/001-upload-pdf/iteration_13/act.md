# Iteration 13: Act

## Changes Made

### File 1: `edgequake/crates/edgequake-api/src/pipeline_progress_callback.rs`
- Lines: 34-45 (imports), 63-78 (struct), 82-103 (impl)
- Change: Added `PipelinePhase` import, `filename` field, `with_filename()` builder
- Why: Persistent progress needs filename for display in GET response

### File 2: `edgequake/crates/edgequake-api/src/pipeline_progress_callback.rs`
- Lines: 123-135 (on_extraction_start)
- Change: Added `tokio::spawn()` call to `start_pdf_progress()` and `start_pdf_phase()`
- Why: Bridges sync callback to async PipelineState methods

### File 3: `edgequake/crates/edgequake-api/src/pipeline_progress_callback.rs`
- Lines: 177-192 (on_page_complete)
- Change: Added `tokio::spawn()` call to `update_pdf_phase()`
- Why: Updates progress as each page completes

### File 4: `edgequake/crates/edgequake-api/src/pipeline_progress_callback.rs`
- Lines: 244-250 (on_extraction_complete)
- Change: Added `tokio::spawn()` call to `complete_pdf_phase()`
- Why: Marks PdfConversion phase as complete

### File 5: `edgequake/crates/edgequake-api/src/pipeline_progress_callback.rs`
- Lines: 218-230 (on_page_error)
- Change: Added `tokio::spawn()` call to `update_pdf_phase()` with error message
- Why: Records page-level errors in persistent storage

### File 6: `edgequake/crates/edgequake-api/src/state.rs`
- Lines: 519-520, 613-614
- Change: Added missing `#[cfg(feature = "postgres")]` on `pdf_storage: None`
- Why: Field is conditionally compiled, initializations must match

## Tests Added/Modified

- Test file: `pipeline_progress_callback.rs`
- Test 1: `test_pipeline_progress_callback_persists_progress`
  - Verifies `get_pdf_progress()` returns data after callbacks fire
  - Checks track_id, pdf_id, filename, phase status, current, total
- Test 2: `test_pipeline_progress_callback_completes_phase`
  - Verifies full extraction flow marks phase as `Complete`
  - Tests 5 pages through start→complete cycle
- Result: 7/7 PASS

## Verification

```bash
# Build test
cargo test --package edgequake-api --lib pipeline_progress_callback
# Result: 7 passed

# Full test to ensure no regressions
cargo test --package edgequake-tasks --lib pipeline_state
# Result: 19 passed (from OODA-12)
```

## Evidence

```
running 7 tests
test pipeline_progress_callback::tests::test_pipeline_progress_callback_with_broadcaster ... ok
test pipeline_progress_callback::tests::test_pipeline_progress_callback_complete ... ok
test pipeline_progress_callback::tests::test_pipeline_progress_callback_page_complete ... ok
test pipeline_progress_callback::tests::test_pipeline_progress_callback_page_error ... ok
test pipeline_progress_callback::tests::test_pipeline_progress_callback_partial_complete ... ok
test pipeline_progress_callback::tests::test_pipeline_progress_callback_persists_progress ... ok
test pipeline_progress_callback::tests::test_pipeline_progress_callback_completes_phase ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 428 filtered out
```

## Commit

- SHA: `53910821`
- Message: "OODA-13: Connect callbacks to persistent progress storage"

## Data Flow After OODA-13

```text
┌──────────────────────┐
│     PdfExtractor     │
│ extract_with_progress│
└──────────┬───────────┘
           │
           ▼
┌──────────────────────────────────────────────────────────────────┐
│                  PipelineProgressCallback                         │
│                                                                   │
│  on_page_complete(page, len)                                      │
│    ├─► emit_pdf_page_progress() ─► PipelineEvent (broadcast)      │
│    ├─► broadcast_event() ────────► ProgressEvent (WebSocket)      │
│    └─► tokio::spawn(update_pdf_phase()) ─► HashMap (persistent)   │
└──────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
                       ┌───────────────────────────────────┐
                       │ PipelineState.pdf_progress        │
                       │ HashMap<String, PdfUploadProgress>│
                       │                                   │
                       │ Queryable by:                     │
                       │ - get_pdf_progress(track_id)      │
                       │ - list_pdf_progress()             │
                       └───────────────────────────────────┘
```

## Next Iteration Focus

OODA-14: Implement GET /api/v1/documents/pdf/:id/progress endpoint
- Create handler function that calls `state.pipeline_state.get_pdf_progress()`
- Add route to router
- Return JSON `PdfUploadProgress` response
- Add OpenAPI documentation
