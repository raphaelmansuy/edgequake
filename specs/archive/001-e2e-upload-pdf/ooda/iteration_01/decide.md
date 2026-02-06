# OODA Iteration 1 – Decide

## Date: 2026-02-06

## Decision: Fix PDF Extraction Path Discovery + Error Propagation

### Priority 1 (CRITICAL): Fix PdfiumExtractor library discovery

- **File**: `edgequake/crates/edgequake-pdf/src/backend/pdfium.rs`
- **Change**: Add workspace-relative paths to the library search list
- **Why**: The bundled `libpdfium.dylib` at `edgequake/crates/edgequake-pdf/lib/lib/` must be discoverable

### Priority 2 (HIGH): Set PDFIUM_DYNAMIC_LIB_PATH in Makefile

- **File**: `Makefile`
- **Change**: Add `PDFIUM_DYNAMIC_LIB_PATH` to `backend-dev`, `backend-db`, `backend-bg` targets
- **Why**: Explicit env var is most reliable approach

### Priority 3 (HIGH): Remove silent MockBackend fallback in production

- **File**: `edgequake/crates/edgequake-pdf/src/extractor.rs`
- **Change**: When PdfiumBackend fails, return an error instead of falling back to MockBackend
- **Why**: Silent empty results are worse than explicit errors

### Priority 4 (MEDIUM): Frontend error display for empty markdown

- **File**: `edgequake_webui/src/components/documents/document-viewer-dialog.tsx`
- **Change**: Show explicit message when PDF is "processed" but has no markdown content
- **Why**: Defense-in-depth — users see what happened even if backend fix misses edge cases

### Priority 5 (LOW): Update .env.example

- **File**: `.env.example`
- **Change**: Document `PDFIUM_DYNAMIC_LIB_PATH`
- **Why**: Future developers need to know about this requirement

### Verification Plan

1. Build backend with changes
2. Start server with PostgreSQL
3. Upload `zz_test_docs/lighrag_2410.05779v3.pdf`
4. Verify markdown content appears in viewer
5. Run `cargo test` to ensure no regressions
