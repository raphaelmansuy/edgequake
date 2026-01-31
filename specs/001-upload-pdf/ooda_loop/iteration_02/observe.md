# Iteration 02: Observe

## Mission Re-Read ✅

- [x] Re-read `/Users/raphaelmansuy/Github/03-working/edgequake/specs/001-upload-pdf.md`
- [x] Confirmed objectives:
  1. ProgressCallback trait with `on_page_start`, `on_page_complete`, `on_extraction_progress`
  2. `extract_to_markdown_with_progress()` method on PdfExtractor
  3. Page-by-page progress updates during PDF conversion phase
- [x] Current phase: Phase 1 - Architecture & Design (Iterations 1-10)
- [x] Previous iteration: Added progress tracking types (OODA-01)

## Code Analysis

### 1. PdfBackend Trait (`edgequake-pdf/src/backend/mod.rs`)

- **Lines**: 1-45
- **Purpose**: Abstract interface for PDF extraction backends
- **Current interface**:
  ```rust
  #[async_trait]
  pub trait PdfBackend: Send + Sync {
      async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document>;
      fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo>;
  }
  ```
- **Key insight**: No progress callback in trait signature

### 2. ExtractionEngine (`edgequake-pdf/src/backend/extraction_engine.rs`)

- **Lines**: 486-556
- **Purpose**: lopdf-based PDF extraction backend
- **Page iteration** (lines 516-554):
  - Uses parallel extraction for 2+ pages
  - Uses sequential extraction for single pages
  - Has `for (page_num, page_id) in pages.iter()` loop
- **Key insight**: Perfect injection point for progress callbacks

### 3. PdfExtractor (`edgequake-pdf/src/extractor.rs`)

- **Lines**: 1-605
- **Purpose**: High-level PDF to Markdown extraction
- **Current methods**:
  - `extract_to_markdown()` - Simple markdown output
  - `extract_document()` - Returns Document IR
  - `extract_full()` - Returns ExtractionResult with errors
- **Key insight**: None have progress callbacks

### 4. Vision Extractor (`edgequake-pdf/src/vision.rs`)

- **Purpose**: Vision-based extraction (renders pages as images)
- **Needs**: Page-level progress during image processing

## Architecture for Progress Callbacks

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         PROGRESS CALLBACK ARCHITECTURE                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  TaskProcessor                                                              │
│       │                                                                     │
│       │ creates callback                                                    │
│       ▼                                                                     │
│  ┌─────────────────┐                                                       │
│  │ProgressCallback │ ◄──────────────────────────────────────────────┐      │
│  │ trait object    │                                                 │      │
│  │ Arc<dyn...>     │                                                 │      │
│  └────────┬────────┘                                                 │      │
│           │                                                          │      │
│           │ passed to                                                │      │
│           ▼                                                          │      │
│  ┌─────────────────────────┐                                        │      │
│  │ PdfExtractor             │                                        │      │
│  │ extract_with_progress() │                                        │      │
│  └────────┬────────────────┘                                        │      │
│           │                                                          │      │
│           │ passed to                                                │      │
│           ▼                                                          │      │
│  ┌─────────────────────────┐                                        │      │
│  │ PdfBackend               │                                        │      │
│  │ extract_with_progress() │                                        │      │
│  └────────┬────────────────┘                                        │      │
│           │                                                          │      │
│           │ calls on each page                                       │      │
│           ▼                                                          │      │
│  ┌─────────────────────────┐     ┌─────────────────────────────────┐│      │
│  │ for page in pages:      │     │ callback.on_page_start(3, 10)   ││      │
│  │   callback.on_page_start│────►│ callback.on_page_complete(3, md)│├──────┘
│  │   extract_page()        │     │ callback.on_progress("pdf", 30%)││
│  │   callback.on_page_done │     └─────────────────────────────────┘│
│  └─────────────────────────┘                                        │
│                                                                      │
│  Callback implementation:                                            │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │ PdfProgressReporter {                                           ││
│  │   progress: Arc<Mutex<PdfUploadProgress>>,                      ││
│  │   websocket_tx: broadcast::Sender<ProgressEvent>,               ││
│  │ }                                                                ││
│  │                                                                  ││
│  │ impl ProgressCallback for PdfProgressReporter {                 ││
│  │   fn on_page_complete(&self, page, _md) {                       ││
│  │     let mut p = self.progress.lock().unwrap();                  ││
│  │     p.update_phase(PdfConversion, page, "Page {page}...");      ││
│  │     self.websocket_tx.send(progress_event);                     ││
│  │   }                                                              ││
│  │ }                                                                ││
│  └─────────────────────────────────────────────────────────────────┘│
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

## Where to Define ProgressCallback Trait?

**Option A: Define in `edgequake-pdf`**

- Pros: Close to where it's used, no cross-crate dependency
- Cons: Can't reference progress types from edgequake-tasks

**Option B: Define in `edgequake-tasks`**

- Pros: Can use PipelinePhase, PhaseProgress types
- Cons: Creates circular dependency (pdf → tasks, tasks → ?)

**Option C: Define in new `edgequake-progress` crate**

- Pros: Clean separation, reusable
- Cons: More crates to manage

**Decision: Option A** - Define in `edgequake-pdf` with simple types

- Keep callback trait generic (page numbers, percentages)
- Let the caller (TaskProcessor) translate to PhaseProgress

## Questions Answered

1. **Where to define trait?** → `edgequake-pdf/src/progress.rs` (new file)
2. **Async vs sync callbacks?** → Sync (blocking) - callbacks are fast operations
3. **Thread safety?** → `Send + Sync` bound on trait
4. **Channel vs closure?** → Trait object with Arc<dyn ProgressCallback>

## Questions for Next Iteration

1. How to handle parallel page extraction with progress callbacks?
2. Should we add progress to PdfBackend trait or just PdfExtractor?
3. How to estimate total items before parsing (for indeterminate progress)?
