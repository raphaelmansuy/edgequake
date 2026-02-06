# OODA Iteration 1 – Observe

## Date: 2026-02-06

## Observation: PDF Upload Flow is Broken Due to Missing libpdfium Runtime

### Territory Map

```
PDF Upload E2E Flow:
═══════════════════

Frontend (Next.js)                          Backend (Rust/Axum)
─────────────────                          ────────────────────
1. User drops PDF on DropZone
      │
      ▼
2. uploadPdfDocument()
   POST /api/v1/documents/pdf        ──→  3. upload_pdf_document handler
   [multipart: file + options]             │  Store PDF in PostgreSQL
                                           │  Create Task(PdfProcessing)
                                           │  Queue task for worker
                                           ▼
                                      4. Worker pool picks up task
                                           │
                                           ▼
                                      5. DocumentTaskProcessor::process_pdf_processing()
                                           │  Load PDF bytes from storage
                                           │  PdfExtractor::new(llm_provider)
                                           │    └─ PdfiumBackend::with_config()  ← FAILS HERE
                                           │        └─ PdfiumExtractor::new()
                                           │            ├─ PDFIUM_DYNAMIC_LIB_PATH? → NOT SET
                                           │            ├─ /usr/local/lib/libpdfium.dylib? → NOT FOUND
                                           │            ├─ /opt/homebrew/lib/libpdfium.dylib? → NOT FOUND
                                           │            └─ Err("libpdfium not found")
                                           │
                                           │  ⚠️ SILENT FALLBACK to MockBackend
                                           │    └─ MockBackend::extract() → Document::new() (EMPTY)
                                           │
                                           ▼
                                      6. MarkdownRenderer::render(empty_doc) → ""
                                           │
                                           ▼
                                      7. pdf_storage.update_pdf_processing(markdown: "")
                                           │
                                           ▼
                                      8. get_pdf_content → { markdown_content: "" }

Frontend displays:
─────────────────
9. DocumentViewerDialog checks:
   isPdf = true, hasMarkdown = false ("" is falsy)
   → Shows PDF only, no side-by-side
   → No error displayed to user
```

### Key Files Examined

| File                                                                  | Lines     | Purpose                | Finding                                            |
| --------------------------------------------------------------------- | --------- | ---------------------- | -------------------------------------------------- |
| `edgequake/crates/edgequake-pdf/src/backend/pdfium.rs`                | 151-174   | PdfiumExtractor::new() | Searches for libpdfium in env var and system paths |
| `edgequake/crates/edgequake-pdf/src/extractor.rs`                     | 191-203   | Backend selection      | **SILENT fallback to MockBackend**                 |
| `edgequake/crates/edgequake-pdf/src/backend/mock.rs`                  | 1-50      | MockBackend            | Returns `Document::new()` (empty)                  |
| `edgequake/crates/edgequake-api/src/processor.rs`                     | 1616-1896 | PDF task processing    | Uses PdfExtractor which silently falls back        |
| `Makefile`                                                            | 299-312   | backend-dev target     | Does NOT set PDFIUM_DYNAMIC_LIB_PATH               |
| `edgequake_webui/src/components/documents/document-viewer-dialog.tsx` | 195-215   | Viewer rendering       | Empty markdown → PDF-only view, no error           |

### Environment State

- `PDFIUM_DYNAMIC_LIB_PATH`: **NOT SET**
- `/usr/local/lib/libpdfium.dylib`: **NOT FOUND**
- `/opt/homebrew/lib/libpdfium.dylib`: **NOT FOUND**
- Actual location: `edgequake/crates/edgequake-pdf/lib/lib/libpdfium.dylib` — **EXISTS but not discoverable**

### Impact Assessment

- **Severity**: CRITICAL — PDF processing is completely non-functional
- **User Experience**: Upload succeeds but produces no content, no error message
- **Blast radius**: ALL PDF uploads in ALL workspaces
