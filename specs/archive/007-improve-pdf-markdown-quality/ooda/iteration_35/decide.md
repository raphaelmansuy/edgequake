# IT35 — Decide: Implementation Plan

## Changes Required

1. **`Cargo.toml`**: Update pdfium-render features to `["thread_safe", "image_024", "pdfium_latest"]` with `default-features = false`

2. **`src/backend/pdfium.rs`**: Add `ExtractedImageData` struct and `extract_images_from_bytes()` method
   - Iterate page objects, find image objects
   - Extract raw image data via `get_raw_image()`
   - Capture bounding box via `bounds().to_rect()`
   - Filter out tiny images (< 10×10 pixels)

3. **`src/backend/mod.rs`**: Export `ExtractedImageData` under `#[cfg(feature = "pdfium")]`

4. **`src/bin.rs`**: Add `--extract-images` CLI flag
   - `extract_and_save_images()` — create PdfiumExtractor, extract images, save as PNG
   - `insert_image_references()` — scan markdown for page markers, insert image refs
   - `parse_page_marker()` — detect both `## Page N` and `<!-- Page N -->` formats

## Risk Assessment

- **Low risk**: Changes are additive — new CLI flag, new extraction method
- **No regression**: Existing pipeline untouched, images only extracted when `--extract-images` used
- **Feature-gated**: All new code behind `#[cfg(feature = "pdfium")]`
