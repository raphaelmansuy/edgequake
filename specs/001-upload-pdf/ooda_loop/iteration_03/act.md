# Iteration 03: Act

## Changes Made

### File 1: `edgequake/crates/edgequake-pdf/src/backend/mod.rs`

- Lines: 1-64
- Change: Added `extract_with_progress()` method to `PdfBackend` trait
- Why: Enable progress callbacks during PDF extraction without breaking existing API
- New method has default implementation that falls back to `extract()`

### File 2: `edgequake/crates/edgequake-pdf/src/backend/extraction_engine.rs`

- Lines: 1-30 (imports)
- Change: Added `use std::sync::Arc;` and `use crate::progress::ProgressCallback;`
- Why: Required for callback parameter type

- Lines: 498-547 (new helper method)
- Change: Added `extract_pages_parallel_with_progress()` method
- Why: Parallel mode needs separate helper to call callbacks from rayon threads

- Lines: 582-700 (trait impl)
- Change: Override `extract_with_progress()` with callback calls in page loops
- Why: Sequential and parallel paths both need to call:
  - `on_extraction_start(total_pages)`
  - `on_page_start(page_num, total)` before each page
  - `on_page_complete(page_num, 0)` or `on_page_error(page_num, err)` after each page
  - `on_extraction_complete(total_pages, success_count)` at end

- Lines: 780-870 (tests)
- Change: Added 2 integration tests
- Why: Verify callbacks are invoked correctly for both 1-page (sequential) and 5-page (parallel) PDFs

### File 3: `edgequake/crates/edgequake-pdf/src/progress.rs`

- Lines: 228-237
- Change: Fixed doctest example for CountingProgress
- Why: Example was asserting non-zero values without calling callbacks

## Tests Added/Modified

### Test 1: `test_extract_with_progress_calls_callbacks`

- File: `extraction_engine.rs`
- Purpose: Verify sequential path callbacks (1-page PDF)
- Assertions:
  - `extraction_started == 1`
  - `pages_started >= 1`
  - `pages_completed + pages_failed == pages_started`
  - `extraction_completed == 1`
- Result: ✅ PASS

### Test 2: `test_extract_with_progress_parallel_mode`

- File: `extraction_engine.rs`
- Purpose: Verify parallel path callbacks (5-page PDF)
- Assertions:
  - `extraction_started == 1`
  - `pages_started >= 2`
  - `extraction_completed == 1`
  - `doc.pages.len() >= 2`
- Result: ✅ PASS

## Verification

```bash
# Build
cargo build --package edgequake-pdf
# Result: Finished `dev` profile in 7.48s

# Unit tests
cargo test --package edgequake-pdf --lib
# Result: ok. 407 passed; 0 failed

# Specific tests
cargo test --package edgequake-pdf extract_with_progress -- --nocapture
# Result: ok. 2 passed; 0 failed

# Doctests
cargo test --package edgequake-pdf --doc
# Result: ok. 4 passed; 0 failed; 8 ignored
```

## Commit

- SHA: `ca45afe3`
- Message: `OODA-03: Integrate ProgressCallback into ExtractionEngine`

## Architecture Diagram

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                    CALLBACK FLOW (OODA-03)                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Caller                                                                     │
│    │                                                                        │
│    │ extract_with_progress(pdf_bytes, Arc<dyn ProgressCallback>)           │
│    │                                                                        │
│    ▼                                                                        │
│  ExtractionEngine                                                           │
│    │                                                                        │
│    ├──► callback.on_extraction_start(total_pages)                          │
│    │                                                                        │
│    ├──► [SEQUENTIAL PATH - 1 page]                                         │
│    │    ├──► callback.on_page_start(1, total)                              │
│    │    ├──► extract_page(...)                                             │
│    │    └──► callback.on_page_complete(1, 0)                               │
│    │                                                                        │
│    ├──► [PARALLEL PATH - 2+ pages]                                         │
│    │    └──► extract_pages_parallel_with_progress()                        │
│    │         └──► rayon::par_iter                                          │
│    │              ├──► callback.on_page_start(n, total)  // out of order!  │
│    │              ├──► load doc per thread                                 │
│    │              ├──► extract_page(...)                                   │
│    │              └──► callback.on_page_complete(n, 0)                     │
│    │                                                                        │
│    └──► callback.on_extraction_complete(total_pages, success_count)        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Documentation Updated

- [x] Inline WHY comments in extraction_engine.rs
- [x] Module docstring updated with "Progress callbacks" bullet
- [x] Trait documentation with example usage
- [x] OODA iteration files (observe.md, orient.md, decide.md, act.md)

## Next Iteration Focus

**OODA-04: Add `extract_to_markdown_with_progress()` to PdfExtractor**

The `PdfExtractor` is the high-level API that most callers use. Currently it has:

- `extract_to_markdown(pdf_bytes)` - returns String

Need to add:

- `extract_to_markdown_with_progress(pdf_bytes, callback)` - calls backend's new method

This will complete the progress callback chain from extractor → backend → pages.
