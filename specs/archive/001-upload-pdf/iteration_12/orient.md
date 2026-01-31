# Iteration 12: Orient

## Gap Analysis

| Current State                         | Desired State                       | Gap                                  | Priority |
| ------------------------------------- | ----------------------------------- | ------------------------------------ | -------- |
| PdfUploadProgress exists but unused   | Store and update per-PDF progress   | Add storage map to PipelineState     | HIGH     |
| Events broadcast but not persisted    | Progress queryable via GET endpoint | Store in HashMap by track_id         | HIGH     |
| 6 callback methods fire independently | Map callbacks to phase updates      | Bridge callbacks → PdfUploadProgress | MEDIUM   |

## Architecture Discovery

The edgequake-tasks crate already has:

1. `PdfUploadProgress` struct with 6 phases
2. `PhaseProgress` with update/complete/fail methods
3. `PipelineState` with broadcast channel
4. `PipelineEvent::PdfPageProgress` variant

What's missing:

1. Storage for `PdfUploadProgress` instances
2. Methods to create/update/query progress by track_id
3. Bridge from `PipelineProgressCallback` to `PdfUploadProgress`

## Design Decision

Add to `PipelineStateInner`:

```rust
/// Active PDF upload progress, keyed by track_id.
pdf_progress: HashMap<String, PdfUploadProgress>,
```

Add methods to `PipelineState`:

```rust
/// Start tracking a PDF upload.
pub async fn start_pdf_progress(&self, track_id: &str, pdf_id: &str, filename: &str);

/// Get current progress for a PDF upload.
pub async fn get_pdf_progress(&self, track_id: &str) -> Option<PdfUploadProgress>;

/// Update phase progress for a PDF upload.
pub async fn update_pdf_phase(&self, track_id: &str, phase: PipelinePhase, current: usize, message: &str);

/// Complete a phase for a PDF upload.
pub async fn complete_pdf_phase(&self, track_id: &str, phase: PipelinePhase);

/// Fail a phase for a PDF upload.
pub async fn fail_pdf_phase(&self, track_id: &str, phase: PipelinePhase, error: PhaseError);

/// Remove completed progress (garbage collection).
pub async fn remove_pdf_progress(&self, track_id: &str);
```

## Callback Bridge

Update `PipelineProgressCallback` to also update `PipelineState.pdf_progress`:

```
on_extraction_start(total_pages)
    → state.start_pdf_progress(track_id, pdf_id, filename)
    → state.start_phase(PipelinePhase::PdfConversion, total_pages)

on_page_complete(page_num, markdown_len)
    → state.update_pdf_phase(track_id, PdfConversion, page_num, "Extracting page N...")

on_extraction_complete(total, success)
    → state.complete_pdf_phase(track_id, PdfConversion)
```

## Risk Assessment

- **Risk**: Memory growth from accumulated progress
  - Mitigation: Add `remove_pdf_progress` after pipeline completes
  - Mitigation: Add TTL-based cleanup (24 hours)

- **Risk**: Concurrent access to progress HashMap
  - Mitigation: Using `RwLock` with fine-grained writes

## First Principles

**Core problem**: Progress events are ephemeral, need persistence for GET endpoint.

**Minimal solution**: In-memory HashMap in PipelineState (same process).

**Why not DB**:

- Real-time updates would require many DB writes
- Progress is transient (only needed during processing)
- DB persistence can be added later for history feature
