# Iteration 12: Decide

## Decision

Add in-memory progress storage to `PipelineState` with methods to create, update, and query `PdfUploadProgress` by track_id.

## Rationale

1. **Minimal change**: Extend existing PipelineState, no new components
2. **Fast**: In-memory access for real-time updates
3. **Queryable**: GET endpoint can call `get_pdf_progress(track_id)`
4. **Thread-safe**: Uses existing RwLock pattern

## Action Items

1. [x] Add `pdf_progress: HashMap<String, PdfUploadProgress>` to `PipelineStateInner`
2. [x] Add `start_pdf_progress()` method to `PipelineState`
3. [x] Add `get_pdf_progress()` method to `PipelineState`
4. [x] Add `update_pdf_phase()` method to `PipelineState`
5. [x] Add `complete_pdf_phase()` and `fail_pdf_phase()` methods
6. [x] Add `remove_pdf_progress()` for cleanup
7. [x] Add unit tests for new methods
8. [ ] Update `PipelineProgressCallback` to update progress (next iteration)

## Success Metrics

- [ ] PipelineState can store PdfUploadProgress by track_id
- [ ] get_pdf_progress returns correct data after updates
- [ ] All existing tests pass
- [ ] New tests pass

## Testing Strategy

- Unit tests: start → update → get → complete → remove flow
- Manual: N/A (internal API)

## Implementation

```rust
// In PipelineStateInner
struct PipelineStateInner {
    // ... existing fields ...

    /// OODA-12: Active PDF upload progress, keyed by track_id.
    /// Enables queryable progress for GET /api/v1/documents/pdf/:id/progress
    pdf_progress: HashMap<String, PdfUploadProgress>,
}

// In PipelineState impl
impl PipelineState {
    /// OODA-12: Start tracking a new PDF upload.
    pub async fn start_pdf_progress(&self, track_id: &str, pdf_id: &str, filename: &str) {
        let progress = PdfUploadProgress::new(
            track_id.to_string(),
            pdf_id.to_string(),
            filename.to_string(),
        );
        let mut inner = self.inner.write().await;
        inner.pdf_progress.insert(track_id.to_string(), progress);
    }

    /// OODA-12: Get current progress for a PDF upload.
    pub async fn get_pdf_progress(&self, track_id: &str) -> Option<PdfUploadProgress> {
        let inner = self.inner.read().await;
        inner.pdf_progress.get(track_id).cloned()
    }

    // ... more methods ...
}
```
