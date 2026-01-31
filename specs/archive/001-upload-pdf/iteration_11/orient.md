# Iteration 11: Orient

## Gap Analysis

| Current State                                                    | Desired State                                   | Gap                                          | Priority |
| ---------------------------------------------------------------- | ----------------------------------------------- | -------------------------------------------- | -------- |
| VisionExtractor has no progress callbacks                        | VisionExtractor should emit page-level progress | Need `extract_from_pdf_with_progress` method | HIGH     |
| extract_from_images loops without callbacks                      | Each page should trigger on_page_start/complete | Add callback parameter to loop               | HIGH     |
| Vision mode falls back in processor.rs but doesn't use callbacks | Fallback should preserve progress tracking      | Update fallback to use same callback         | MEDIUM   |

## VisionExtractor Structure Analysis

```
VisionExtractor
    │
    ├── extract_from_pdf(pdf_bytes)
    │       │
    │       ├── PageRenderer.render_pages() → Vec<PageImage>
    │       │
    │       └── extract_from_images(images)
    │               │
    │               └── for image in images:
    │                       extract_page(image) → Page
    │
    └── extract_from_images(images)
            │
            └── for image in images:
                    extract_page(image)
```

## Callback Injection Points

1. **Before loop in `extract_from_images`**: `on_extraction_start(images.len())`
2. **Start of each iteration**: `on_page_start(page_num, total_pages)`
3. **After `extract_page` succeeds**: `on_page_complete(page_num, markdown_len)`
4. **If `extract_page` fails**: `on_page_error(page_num, error)`
5. **After loop completes**: `on_extraction_complete(total_pages, success_count)`

## Risk Assessment

- **Risk 1**: Breaking existing API without progress callback
  - Mitigation: Add new method `extract_from_pdf_with_progress` alongside existing
- **Risk 2**: Arc<dyn ProgressCallback> overhead in hot path
  - Mitigation: Minimal - one callback per page, not per token

## Design Decision

Follow same pattern as `PdfExtractor::extract_to_markdown_with_progress`:

```rust
pub async fn extract_from_pdf_with_progress<P>(
    &self,
    pdf_bytes: &[u8],
    progress: Arc<P>,
) -> Result<Document>
where
    P: ProgressCallback,
{
    // Render pages
    progress.on_extraction_start(images.len());

    // Loop with callbacks
    for (idx, image) in images.iter().enumerate() {
        progress.on_page_start(idx + 1, images.len());
        match self.extract_page(image).await {
            Ok(page) => {
                progress.on_page_complete(idx + 1, page.content.len());
                document.add_page(page);
            }
            Err(e) => {
                progress.on_page_error(idx + 1, &e.to_string());
                // Continue or fail based on config
            }
        }
    }

    progress.on_extraction_complete(images.len(), success_count);
    Ok(document)
}
```

## First Principles Analysis

- **Core problem**: Vision extraction is a black box with no visibility
- **Fundamental constraint**: Can't speed up LLM calls, but can show progress
- **Minimal solution**: Add callbacks at page boundaries (not token level)
- **Why this matters**: Users see "Processing with vision..." for minutes with no feedback

## Alternative Approaches

1. **Option A**: Add progress to existing method signature (BREAKING)
   - Pros: Cleaner API
   - Cons: Breaks all existing callers

2. **Option B**: Add new `_with_progress` method (ADDITIVE)
   - Pros: Backward compatible, follows PdfExtractor pattern
   - Cons: API duplication

**Decision**: Option B - Add `extract_from_pdf_with_progress` method.

## Files to Modify

1. `edgequake-pdf/src/vision.rs` - Add `extract_from_pdf_with_progress`, `extract_from_images_with_progress`
2. `edgequake-api/src/processor.rs` - Update vision path to use progress callback
