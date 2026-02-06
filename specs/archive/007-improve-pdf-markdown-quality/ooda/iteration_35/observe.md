# IT35 — Observe: Image Extraction Infrastructure

## Current State

- `image_ocr.rs` has LLM-based OCR module but is NOT connected to actual PDF image extraction
- `pdfium_backend.rs` has `has_images: false, image_count: 0` hardcoded in `get_info()`
- `BlockType::Figure` and `BlockType::Picture` exist in the schema but are unused for embedded images
- `bin.rs` has a `--vision` flag for LLM-based image description but no raw image extraction
- pdfium-render 0.8 provides `PdfPageImageObject::get_raw_image()` returning `image::DynamicImage`

## Spec Requirement

> "If image is discovered in the PDF they should be extracted in ./assets/ subfolder and linked as image in the transformed markdown as a Markdown image"

## pdfium-render Image API

- `page.objects().iter()` iterates all page objects (text, image, path, etc.)
- `object.as_image_object()` returns `Some(PdfPageImageObject)` for image objects
- `image_obj.get_raw_image()` returns `Result<DynamicImage>` — the raw pixel data
- `object.bounds()` returns `PdfQuadPoints`, `.to_rect()` gives `PdfRect` with `.left()/.bottom()/.right()/.top()`
- Minimum viable image filter: skip images < 10×10 pixels (decorative/spacer elements)

## Test Documents

- LightRAG PDF (`lighrag_2410.05779v3.pdf`): 16 pages, 5 embedded figures on pages 12-14
- Elitizon PDF (`AI_Services__Elitizon.pdf`): Business document, no embedded images
