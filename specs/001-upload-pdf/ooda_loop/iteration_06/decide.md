# Iteration 06: Decide

## Decision

Add `PdfPageProgress` event variant to `ProgressEvent` enum in websocket_types.rs.

## Rationale

- Specific event type provides clear semantics for PDF page progress
- Doesn't break existing WebSocket clients (additive change)
- Matches mission requirement for "page-by-page progress"

## Action Items

1. [x] Add `PdfPageProgress` variant to `ProgressEvent` enum
2. [x] Include fields: pdf_id, page_num, total_pages, phase, markdown_len
3. [x] Update OpenAPI documentation

## Success Metrics

- [x] New event variant compiles
- [x] Serializes to expected JSON structure
- [x] Existing tests still pass

## Event Schema

```rust
/// PDF page-level progress event.
///
/// @implements SPEC-001-upload-pdf: Page-by-page progress during PDF conversion
PdfPageProgress {
    /// PDF document ID.
    pdf_id: String,
    /// Task tracking ID.
    task_id: String,
    /// Current page number (1-indexed).
    page_num: u32,
    /// Total pages in PDF.
    total_pages: u32,
    /// Current phase of processing.
    phase: String,  // "extraction", "processing", "embedding"
    /// Markdown length for this page (0 if not yet rendered).
    markdown_len: usize,
    /// Whether this page completed successfully.
    success: bool,
    /// Error message if failed.
    error: Option<String>,
}
```

Expected JSON output:

```json
{
  "type": "PdfPageProgress",
  "data": {
    "pdf_id": "abc123",
    "task_id": "task456",
    "page_num": 5,
    "total_pages": 30,
    "phase": "extraction",
    "markdown_len": 0,
    "success": true,
    "error": null
  }
}
```
