# Iteration 04: Observe

## Mission Re-Read ✅

- [x] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed objectives: Page-by-page progress during PDF-to-Markdown conversion
- [x] Current phase: Phase 1 - Architecture & Design (Iterations 1-10)
- [x] Previous iterations:
  - OODA-01: Added PipelinePhase, PhaseProgress, PdfUploadProgress types
  - OODA-02: Added ProgressCallback trait with NoopProgress, LoggingProgress, CountingProgress
  - OODA-03: Integrated ProgressCallback into ExtractionEngine (backend level)

## Code Analysis

### `PdfExtractor` (extractor.rs)

The main high-level API that callers use:

```rust
pub struct PdfExtractor {
    backend: Box<dyn PdfBackend>,
    llm_provider: Arc<dyn LLMProvider>,
    config: PdfConfig,
}
```

Current methods:

- `extract_to_markdown(&self, pdf_bytes) -> Result<String>` - Main entry point
- `extract_document(&self, pdf_bytes) -> Result<Document>` - Returns Document IR
- `extract_full(&self, pdf_bytes) -> Result<ExtractionResult>` - Detailed results
- `extract_text(&self, pdf_bytes) -> Result<String>` - Raw text only
- `get_info(&self, pdf_bytes) -> Result<PdfInfo>` - Metadata only

Need to add:

- `extract_to_markdown_with_progress(&self, pdf_bytes, callback) -> Result<String>`

### Method Flow

```text
extract_to_markdown(pdf_bytes)
├── extract_document(pdf_bytes)
│   ├── backend.extract(pdf_bytes)  ◄── OODA-03 added progress here
│   ├── apply_processors(doc)       ◄── No progress yet (future)
│   └── Optional AI enhancement     ◄── No progress yet (future)
└── MarkdownRenderer.render(doc)
```

For OODA-04, we only need to wire through the callback to the backend level.
Later iterations can add progress for processors and AI enhancement.

### Dependencies

- `PdfBackend::extract_with_progress()` - Added in OODA-03 ✅
- `Arc<dyn ProgressCallback>` - From progress.rs ✅

## Implementation Strategy

1. Add `extract_document_with_progress()` internal method
2. Add `extract_to_markdown_with_progress()` public method
3. Wire callback through to `backend.extract_with_progress()`
4. Test with CountingProgress

## Questions for Next Iteration

1. Should processors also report progress? → Yes, but in later iteration
2. Should AI enhancement report progress? → Yes, but in later iteration
3. How to handle phase transitions? → Callback already has `on_progress(phase, percent)`
