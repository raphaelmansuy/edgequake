# OODA-44: Decide - Deprecation Implementation

## Date: 2026-02-05

## Decision

Add `#[deprecated]` attribute to `ExtractionEngine` struct with migration notes.

---

## Code Change

File: `backend/extraction_engine.rs`

Location: Before `pub struct ExtractionEngine`

```rust
#[deprecated(
    since = "0.2.0",
    note = "Use PdfiumBackend instead for accurate font style detection. \
            ExtractionEngine relies on font name patterns (unreliable). \
            PdfiumBackend uses font descriptor flags from PDFium (accurate)."
)]
pub struct ExtractionEngine {
    // ... existing fields
}
```

---

## Additional Documentation

Update `backend/mod.rs` module docs:

```rust
//! # Backend Selection
//!
//! ## Recommended: PdfiumBackend
//!
//! The `PdfiumBackend` provides higher quality extraction using PDFium's
//! accurate font descriptor flags for bold/italic detection.
//!
//! ## Legacy: ExtractionEngine (lopdf)
//!
//! The `ExtractionEngine` is deprecated and will be removed in a future version.
//! It relies on font name pattern matching which is unreliable for many PDFs.
```

---

## Acceptance Criteria

- [ ] ExtractionEngine has #[deprecated] attribute
- [ ] Compiles without errors
- [ ] Shows deprecation warning when ExtractionEngine is used directly
- [ ] No warning when PdfiumBackend is used (default path)
