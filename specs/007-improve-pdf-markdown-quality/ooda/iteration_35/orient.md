# IT35 — Orient: Image Extraction Design

## Key Decision: Separate from Pipeline

Image extraction needs to happen at the **binary/CLI level**, not inside the document processing pipeline, because:

1. Images must be saved as **external files** (PNG in ./assets/)
2. The pipeline operates on structured `Document` blocks — images are binary blobs
3. Markdown references need to be **inserted after** the full markdown is rendered
4. The existing `--vision` flag handles LLM-based image description (separate concern)

## Architecture

```
PdfiumExtractor::extract_images_from_bytes()
    → Vec<ExtractedImageData> { page_num, index, bbox, image, width, height }

bin.rs::extract_and_save_images()
    → Saves PNG files to ./assets/ dir
    → Returns Vec<ImageRef> { page_num, index, filename }

bin.rs::insert_image_references()
    → Scans markdown for page markers (## Page N or <!-- Page N -->)
    → Inserts ![Figure](./assets/pageN_imgM.png) at page boundaries
```

## Cargo.toml Changes

- pdfium-render features: `["thread_safe", "image_024", "pdfium_latest"]`
- `image_024` ensures compatibility with our `image = "0.24"` crate
- `pdfium_latest` required for `crate::bindgen::*` imports
- `default-features = false` to avoid pulling `image_latest` (0.25)

## Page Marker Formats

MarkdownRenderer uses two possible formats depending on configuration:
- `## Page N` (default heading-based)  
- `<!-- Page N -->` (comment-based)

Both must be supported for image reference insertion.
