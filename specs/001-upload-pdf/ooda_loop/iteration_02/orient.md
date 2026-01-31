# Iteration 02: Orient

## Mission Re-Read ✅

- [x] Re-read mission file at `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed: ProgressCallback trait is core requirement for Phase 2 progress

## Gap Analysis

| Current State                            | Desired State                           | Gap                 | Priority |
| ---------------------------------------- | --------------------------------------- | ------------------- | -------- |
| No progress callback in PdfExtractor     | `extract_with_progress()` with callback | Need trait + method | HIGH     |
| PdfBackend.extract() has no progress     | Need callback injection point           | Add to backend      | HIGH     |
| ExtractionEngine iterates pages silently | Call progress on each page              | Add callback calls  | HIGH     |
| VisionExtractor has no progress          | Page-level vision progress              | Separate iteration  | MEDIUM   |

## Risk Assessment

- **Risk 1: Breaking existing PdfBackend implementations**
  - Mitigation: Add `extract_with_progress()` as new method with default impl
  - Mitigation: Keep `extract()` unchanged

- **Risk 2: Progress overhead slowing extraction**
  - Mitigation: Callbacks are ~1µs (just increment counter + send event)
  - Mitigation: Batch updates if needed

- **Risk 3: Callback lifetime issues in async context**
  - Mitigation: Use `Arc<dyn ProgressCallback>` not references
  - Mitigation: `Send + Sync` bounds on trait

## First Principles Analysis

### Core Problem

PDF extraction happens page-by-page but gives no visibility to the caller.

### Fundamental Truths

1. PDF page count is known early (from PDF catalog)
2. Each page takes variable time (0.1s to 5s)
3. Progress callback must be fast (< 1ms)
4. Trait objects work well for callbacks in Rust

### Minimal Solution

1. Define `ProgressCallback` trait in `edgequake-pdf`
2. Add `NoopProgress` default implementation
3. Add `extract_with_progress()` to `PdfExtractor`
4. Call callbacks in `ExtractionEngine::extract()`

## ProgressCallback Trait Design

```rust
/// Callback trait for PDF extraction progress.
///
/// WHY trait object instead of closure?
/// - More flexible: can hold state (counters, channels)
/// - More testable: can mock easily
/// - More ergonomic: named methods vs. closure types
pub trait ProgressCallback: Send + Sync {
    /// Called when extraction starts.
    fn on_extraction_start(&self, total_pages: usize) {
        // Default: no-op
        let _ = total_pages;
    }

    /// Called before processing a page.
    fn on_page_start(&self, page_num: usize, total_pages: usize) {
        // Default: no-op
        let _ = (page_num, total_pages);
    }

    /// Called after a page is successfully extracted.
    fn on_page_complete(&self, page_num: usize, markdown_len: usize) {
        // Default: no-op
        let _ = (page_num, markdown_len);
    }

    /// Called when a page fails to extract.
    fn on_page_error(&self, page_num: usize, error: &str) {
        // Default: no-op
        let _ = (page_num, error);
    }

    /// Called when extraction is complete.
    fn on_extraction_complete(&self, total_pages: usize, success_count: usize) {
        // Default: no-op
        let _ = (total_pages, success_count);
    }

    /// Called for general progress updates (0.0 - 100.0).
    fn on_progress(&self, phase: &str, percent: f32) {
        // Default: no-op
        let _ = (phase, percent);
    }
}
```

## Alternative Designs Considered

### Option A: Closure-based callbacks

```rust
pub async fn extract_with_progress<F>(
    &self,
    pdf_bytes: &[u8],
    on_progress: F,
) -> Result<String>
where
    F: Fn(usize, usize) + Send + Sync,
```

- **Pros**: Simple for one-off use
- **Cons**: Only one callback type, can't pass state easily
- **Verdict**: Not flexible enough

### Option B: Channel-based

```rust
pub async fn extract_with_progress(
    &self,
    pdf_bytes: &[u8],
    tx: mpsc::Sender<ProgressEvent>,
) -> Result<String>
```

- **Pros**: Decoupled, natural for async
- **Cons**: Requires receiver setup, more boilerplate
- **Verdict**: Good but more complex

### Option C: Trait object (SELECTED)

```rust
pub async fn extract_with_progress(
    &self,
    pdf_bytes: &[u8],
    callback: Arc<dyn ProgressCallback>,
) -> Result<String>
```

- **Pros**: Flexible, testable, can hold state
- **Cons**: Slightly more boilerplate to implement
- **Verdict**: ✅ Best balance of flexibility and simplicity

## Implementation Order

1. Create `edgequake-pdf/src/progress.rs` with trait
2. Add `NoopProgress` default implementation
3. Add `extract_with_progress()` to PdfExtractor
4. Modify ExtractionEngine to accept optional callback
5. Add tests
