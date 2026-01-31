# Iteration 03: Observe

## Mission Re-Read ✅

- [x] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed objectives: Page-by-page progress during PDF-to-Markdown conversion
- [x] Current phase: Phase 1 - Architecture & Design (Iterations 1-10)
- [x] Previous iterations:
  - OODA-01: Added PipelinePhase, PhaseProgress, PdfUploadProgress types
  - OODA-02: Added ProgressCallback trait with NoopProgress, LoggingProgress, CountingProgress

## Code Analysis

### 1. PdfExtractor Methods to Add (`extractor.rs`)

Current methods:

- `extract_to_markdown(&self, pdf_bytes: &[u8]) -> Result<String>` - No progress
- `extract_document(&self, pdf_bytes: &[u8]) -> Result<Document>` - No progress
- `extract_full(&self, pdf_bytes: &[u8]) -> Result<ExtractionResult>` - No progress

Need to add:

- `extract_to_markdown_with_progress(&self, pdf_bytes: &[u8], callback: Arc<dyn ProgressCallback>) -> Result<String>`

### 2. PdfBackend Trait (`backend/mod.rs`)

Current:

```rust
#[async_trait]
pub trait PdfBackend: Send + Sync {
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document>;
    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo>;
}
```

Options for adding progress:

1. **Add optional callback to extract()** - Breaking change
2. **Add new method with default impl** - Non-breaking ✅

Proposed:

```rust
async fn extract_with_progress(
    &self,
    pdf_bytes: &[u8],
    callback: Arc<dyn ProgressCallback>,
) -> Result<Document> {
    // Default: ignore callback, call existing extract()
    self.extract(pdf_bytes).await
}
```

### 3. ExtractionEngine Page Loop (`backend/extraction_engine.rs`)

Lines 516-554: This is where pages are iterated:

```rust
// Sequential extraction for small documents
for (page_num, page_id) in pages.iter().take(pages_to_process) {
    debug!("Processing page {}", page_num);
    match self.extract_page(&lopdf_doc, *page_id, *page_num as usize) {
        Ok(page) => { document.add_page(page); }
        Err(e) => { warn!("Failed to extract page {}: {}", page_num, e); }
    }
}
```

This is the perfect injection point for progress callbacks!

### 4. Parallel Extraction Challenge

Lines 521-532: Parallel mode uses rayon for multi-page extraction:

```rust
let mut results = self.extract_pages_parallel(pdf_bytes, page_infos);
```

**Challenge**: Calling callbacks from multiple threads needs care.
**Solution**: ProgressCallback is already `Send + Sync`, so safe to call from rayon threads.

## Architecture Decision

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    INTEGRATION ARCHITECTURE                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PdfExtractor                                                               │
│       │                                                                     │
│       │ extract_to_markdown_with_progress(bytes, callback)                 │
│       │                                                                     │
│       ├─────────────────────────────────────────────────────────────────┐  │
│       │                                                                 │  │
│       │  1. Get page count from backend.get_info()                     │  │
│       │  2. callback.on_extraction_start(page_count)                   │  │
│       │  3. Call backend.extract_with_progress(bytes, callback)        │  │
│       │     └─► ExtractionEngine loops pages, calls:                   │  │
│       │         ├─ callback.on_page_start(page, total)                 │  │
│       │         ├─ extract_page(...)                                   │  │
│       │         └─ callback.on_page_complete(page, len)                │  │
│       │  4. Apply processors (no callbacks yet)                        │  │
│       │  5. Render to Markdown                                         │  │
│       │  6. callback.on_extraction_complete(total, success)            │  │
│       │                                                                 │  │
│       └─────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Implementation Plan

1. Add `extract_with_progress()` to `PdfBackend` trait with default impl
2. Override in `ExtractionEngine` to call callbacks during page loop
3. Add `extract_to_markdown_with_progress()` to `PdfExtractor`
4. Add integration test with `CountingProgress`

## Questions for This Iteration

1. Should parallel mode also call callbacks? → Yes, but may be out of order
2. Should we report progress during processor chain? → Later iteration
3. Where to calculate markdown length for `on_page_complete`? → After render
