# Iteration 03: Decide

## Decision

We will implement `extract_with_progress()` method on both `PdfBackend` trait and `ExtractionEngine`, then add `extract_to_markdown_with_progress()` to `PdfExtractor`.

## Rationale

Using first principles:

1. **Non-breaking**: Default implementation means existing code works unchanged
2. **Testable**: Can verify callbacks are called with CountingProgress
3. **Thread-safe**: ProgressCallback already requires Send + Sync
4. **Minimal change**: Only ~150 lines across 3 files

## Action Items

1. [x] Add `extract_with_progress()` to `PdfBackend` trait with default impl
   - File: `edgequake/crates/edgequake-pdf/src/backend/mod.rs`
   - Est: 5 min

2. [x] Override `extract_with_progress()` in `ExtractionEngine`
   - File: `edgequake/crates/edgequake-pdf/src/backend/extraction_engine.rs`
   - Est: 15 min

3. [ ] Add `extract_to_markdown_with_progress()` to `PdfExtractor`
   - File: `edgequake/crates/edgequake-pdf/src/extractor.rs`
   - Est: 10 min (deferred to iteration 04)

4. [x] Add integration test with CountingProgress
   - File: `edgequake/crates/edgequake-pdf/src/backend/extraction_engine.rs` (inline tests)
   - Est: 10 min

## Success Metrics

- [x] `PdfBackend` trait compiles with new method
- [x] `ExtractionEngine::extract_with_progress()` calls all 4 callback types
- [x] Test verifies: start(1), page_start(N), page_complete(N), complete(1)
- [x] Existing tests still pass

## Testing Strategy

- Unit test: `test_extract_with_progress_calls_callbacks`
  - Uses CountingProgress to verify exact callback counts
  - Asserts: `starts == 1`, `page_starts == page_count`, `page_completes + page_errors == page_count`, `completes == 1`

- Manual verification:
  ```bash
  cargo test --package edgequake-pdf -- --nocapture
  ```

## Callback Flow (Sequential Mode)

```text
extract_with_progress(bytes, callback)
├── callback.on_extraction_start(total_pages)
│
├── for page in 1..=total_pages:
│   ├── callback.on_page_start(page, total_pages)
│   ├── extract_page(...)
│   └── callback.on_page_complete(page, 0) OR on_page_error(page, err)
│
└── callback.on_extraction_complete(total_pages, success_count)
```

## Callback Flow (Parallel Mode)

```text
extract_with_progress(bytes, callback)
├── callback.on_extraction_start(total_pages)
│
├── pages.par_iter().map(|page| {
│   │   callback.on_page_start(page, total_pages)  // may be out of order!
│   │   extract_page(...)
│   └── callback.on_page_complete/error(page, ...)
│   })
│
└── callback.on_extraction_complete(total_pages, success_count)
```

Note: In parallel mode, `on_page_start` and `on_page_complete` may arrive out of order.
The UI should handle this by tracking state per page_num, not assuming sequential order.
