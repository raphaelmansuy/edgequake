# Decide – OODA-23: Update All 4 TODOs with KNOWN LIMITATION Comments

## Decision

Update each TODO with a detailed KNOWN LIMITATION comment.

## Changes

### 1. extractor.rs:539

```rust
images: Vec::new(),
// KNOWN LIMITATION: Image extraction not implemented in text mode
// WHY: Requires vision/multimodal LLM for OCR and image understanding
// WORKAROUND: Use Vision mode (ExtractionMode::Vision) for documents with images
// FUTURE: Could extract image bytes and use ImageOcrConfig for LLM-based OCR
```

### 2. pymupdf_grouper.rs:163

```rust
// KNOWN LIMITATION: Vertical text detection not implemented
// WHY: PDFium character bboxes don't indicate text direction reliably
// Aspect ratio heuristics fail because normal chars often have height > width
// WORKAROUND: ArXiv watermarks are filtered by margin position instead
// FUTURE: Analyze character sequence patterns to detect vertical runs
```

### 3. pymupdf_renderer.rs:156

```rust
// KNOWN LIMITATION: Proper table rendering not implemented
// WHY: Requires cell boundary detection which is complex:
// - May need PDF line/rect detection for borders
// - Cell content alignment detection
// - Table structure inference from spatial relationships
// WORKAROUND: Tables are rendered as paragraphs (text preserved)
// FUTURE: Use backend/lattice.rs for table structure detection
```

### 4. pdfium_backend.rs:296

```rust
has_images: false,
// KNOWN LIMITATION: Image presence detection not implemented
// WHY: Would require scanning PDF page objects for XObject/Image types
// PDFium API can enumerate page objects but adds complexity
// WORKAROUND: Assume images present if Vision mode requested
// FUTURE: Could use pdfium_render's page_objects() iterator
```
