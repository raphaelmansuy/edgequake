# Iteration 12: Act

## Changes Made

### File 1: `edgequake-tasks/src/pipeline_state.rs`
- Lines: 10-14
- Change: Added imports for HashMap, PdfUploadProgress, PhaseError, PipelinePhase
- Why: Required for progress storage

### File 2: `edgequake-tasks/src/pipeline_state.rs`
- Lines: 168-172
- Change: Added `pdf_progress: HashMap<String, PdfUploadProgress>` to PipelineStateInner
- Why: Storage for queryable progress by track_id

### File 3: `edgequake-tasks/src/pipeline_state.rs`
- Lines: 548-652
- Change: Added 8 new methods to PipelineState:
  - `start_pdf_progress()` - Create new progress entry
  - `get_pdf_progress()` - Query by track_id
  - `start_pdf_phase()` - Begin a phase
  - `update_pdf_phase()` - Update current/total
  - `complete_pdf_phase()` - Mark phase complete
  - `fail_pdf_phase()` - Mark phase failed
  - `remove_pdf_progress()` - Cleanup
  - `list_pdf_progress()` - Admin monitoring
- Why: Enables GET /api/v1/documents/pdf/:id/progress endpoint

### File 4: `edgequake-tasks/src/pipeline_state.rs`
- Lines: 810-932
- Change: Added 7 new tests
- Why: Verify progress storage CRUD operations

## Tests Run

```
running 19 tests
test pipeline_state::tests::test_start_pdf_progress ... ok
test pipeline_state::tests::test_get_pdf_progress_not_found ... ok
test pipeline_state::tests::test_update_pdf_phase ... ok
test pipeline_state::tests::test_complete_pdf_phase ... ok
test pipeline_state::tests::test_fail_pdf_phase ... ok
test pipeline_state::tests::test_remove_pdf_progress ... ok
test pipeline_state::tests::test_list_pdf_progress ... ok
... (12 more existing tests)

test result: ok. 19 passed
```

## Commit
`fd80f76a` - OODA-12: Add PDF progress persistence to PipelineState

## Architecture Update

```
                          ┌───────────────────────────────┐
                          │       PipelineState           │
                          ├───────────────────────────────┤
                          │ PipelineStateInner:           │
                          │   - is_busy                   │
                          │   - messages                  │
                          │   - pdf_progress ◄── NEW      │
                          │       HashMap<track_id,       │
                          │         PdfUploadProgress>    │
                          └───────────────────────────────┘
                                      │
                    ┌─────────────────┼─────────────────┐
                    │                 │                 │
                    ▼                 ▼                 ▼
              broadcast()      get_pdf_progress()  update_pdf_phase()
              (ephemeral)       (queryable)         (persistent)
```

## Phase 2 Progress

- [x] Progress persistence to task storage (OODA-12) ✅
- [ ] Implement GET /api/v1/documents/pdf/:id/progress endpoint
- [ ] Add WebSocket /ws/progress/:track_id endpoint
- [ ] Add error recovery endpoints (retry, cancel)

## Next: OODA-13

Connect `PipelineProgressCallback` to update `PipelineState.pdf_progress`.
Currently callbacks emit events but don't update the persistent storage.
