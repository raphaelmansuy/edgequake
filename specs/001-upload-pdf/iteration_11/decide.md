# Iteration 11: Decide

## Decision

We will add progress callback support to VisionExtractor following the same pattern as PdfExtractor.

## Rationale

1. **Consistency**: Same pattern as `extract_to_markdown_with_progress` (OODA-04)
2. **Backward compatible**: New method doesn't break existing callers
3. **Same trait**: Uses existing `ProgressCallback` trait (OODA-02)
4. **Full coverage**: Completes extraction layer instrumentation

## Action Items

1. [x] Add `extract_from_pdf_with_progress` method to VisionExtractor
2. [x] Add `extract_from_images_with_progress` method to VisionExtractor
3. [x] Update processor.rs vision path to use progress callback
4. [x] Add unit tests for VisionExtractor progress callbacks
5. [x] Verify existing vision tests still pass

## Success Metrics

- [ ] VisionExtractor has `extract_from_pdf_with_progress` method
- [ ] Vision path in processor.rs uses progress callback
- [ ] All existing vision tests pass
- [ ] New progress callback test passes

## Testing Strategy

- Unit tests: VisionExtractor with mock callback
- Integration: Not needed (vision requires LLM)
- Manual verification: Build check only

## Implementation Plan

```rust
// In vision.rs

#[cfg(feature = "vision")]
pub async fn extract_from_pdf_with_progress<P>(
    &self,
    pdf_bytes: &[u8],
    progress: Arc<P>,
) -> Result<Document>
where
    P: ProgressCallback,
{
    // 1. Render pages (could be slow - emit progress)
    let renderer = PageRenderer::new()?
        .with_dpi(self.config.dpi)
        .with_format(ImageFormat::Png);
    let images = renderer.render_pages(pdf_bytes)?;

    // 2. Emit extraction start
    progress.on_extraction_start(images.len());

    // 3. Extract with progress
    self.extract_from_images_with_progress(&images, progress).await
}

pub async fn extract_from_images_with_progress<P>(
    &self,
    images: &[PageImage],
    progress: Arc<P>,
) -> Result<Document>
where
    P: ProgressCallback,
{
    let mut document = Document::new();
    document.method = ExtractionMethod::Vision;
    let mut success_count = 0;
    let total = images.len();

    for (idx, image) in images.iter().enumerate() {
        let page_num = idx + 1;
        progress.on_page_start(page_num, total);

        match self.extract_page(image).await {
            Ok(page) => {
                progress.on_page_complete(page_num, page.content.len());
                document.add_page(page);
                success_count += 1;
            }
            Err(e) => {
                progress.on_page_error(page_num, &e.to_string());
                // Continue - vision extraction continues on error
            }
        }
    }

    progress.on_extraction_complete(total, success_count);

    document.update_stats();
    document.generate_toc();

    Ok(document)
}
```
