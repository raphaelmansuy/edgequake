# Observe – OODA-23: Document Remaining TODOs as Known Limitations

## Current State

4 remaining TODOs in edgequake-pdf:
1. `extractor.rs:539` - `images: Vec::new()` // TODO: Extract images
2. `layout/pymupdf_grouper.rs:163` - TODO: Implement proper vertical text detection
3. `layout/pymupdf_renderer.rs:156` - TODO: Implement proper table detection
4. `backend/pdfium_backend.rs:296` - `has_images: false` // TODO: Could scan for images

## Analysis

These TODOs represent legitimate known limitations:
- **Image extraction**: Complex, requires vision/multimodal support
- **Vertical text detection**: Complex, needs character sequence analysis
- **Table detection**: Complex, needs cell boundary detection
- **Image scanning**: Moderately complex, requires PDF object traversal

## Approach

Instead of implementing these complex features, document them as known limitations with WHY comments explaining the technical constraints.
