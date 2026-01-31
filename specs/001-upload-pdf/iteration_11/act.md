# Iteration 11: Act

## Changes Made

### File 1: `edgequake-pdf/src/vision.rs`

- Lines: 235-301
- Change: Added `extract_from_pdf_with_progress()` method
- Why: Users need page-level progress visibility during vision extraction

### File 2: `edgequake-pdf/src/vision.rs`

- Lines: 320-372
- Change: Added `extract_from_images_with_progress()` method
- Why: Core extraction loop with progress callbacks

### File 3: `edgequake-pdf/src/vision.rs`

- Lines: 20
- Change: Added `use crate::progress::ProgressCallback;`
- Why: Import trait for progress callbacks

### File 4: `edgequake-api/src/processor.rs`

- Lines: 1339-1342
- Change: Updated vision path to use `extract_from_pdf_with_progress`
- Why: Connect vision extraction to progress event system

## Tests Run

```
running 408 tests
...
test result: ok. 408 passed; 0 failed
```

## Commit

`34f7f7ab` - OODA-11: Add progress callbacks to VisionExtractor

## Complete Extraction Coverage

Both extraction paths now emit progress:

```
PDF Upload
    │
    ├── Vision Mode (enable_vision=true)
    │       │
    │       └── VisionExtractor.extract_from_pdf_with_progress()
    │               │
    │               └── on_page_start/complete/error
    │
    └── Text Mode (enable_vision=false)
            │
            └── PdfExtractor.extract_to_markdown_with_progress()
                    │
                    └── on_page_start/complete/error
```

## Phase 2 Progress

From mission file Phase 2 tasks:

- [x] observe.md: PDF worker task handler code review
- [x] orient.md: Progress callback injection points
- [x] decide.md: Progress update event schema
- [x] act.md: Instrument PDF extractor with progress callbacks (OODA-04)
- [x] act.md: Instrument vision processor with page-level progress (OODA-11) ✅
- [ ] act.md: Add progress persistence to task storage
- [ ] act.md: Implement GET /api/v1/documents/pdf/:id/progress endpoint
- [ ] act.md: Add WebSocket /ws/progress/:track_id endpoint
- [ ] act.md: Add error recovery endpoints (retry, cancel)

## Next: OODA-12

Progress persistence to task storage - currently events are ephemeral.
