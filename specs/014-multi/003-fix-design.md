# SPEC-014 — Fix Design

## Implemented changes

### 1) OpenAPI contract parity

Updated `edgequake-api/src/openapi.rs` to expose:

- `POST /api/v1/documents/upload` (`upload_file`)
- `POST /api/v1/documents/upload/batch` (`upload_files_batch`)
- `POST /api/v1/documents/pdf/batch` (`upload_pdf_batch_document`) **new**

and schemas:

- `FileUploadResponse`
- `BatchUploadResponse`
- `BatchFileResult`
- `PdfUploadResponse`
- `PdfMetadata`
- `PdfBatchUploadResponse` **new**
- `PdfBatchFileResult` **new**

### 2) New PDF batch endpoint

Added `upload_pdf_batch_document` in:
- `edgequake-api/src/handlers/pdf_upload/upload.rs`

Route:
- `/api/v1/documents/pdf/batch`

Behavior:
- accepts repeated multipart fields `file` or `files`
- applies common upload options (`enable_vision`, `vision_provider`, `vision_model`, `track_id`, `force_reindex`, `pdf_parser_backend`)
- processes each file using same validation/storage/task flow as single upload via shared helper `process_pdf_upload_parts`
- returns aggregate response (`PdfBatchUploadResponse`) plus per-file statuses

### 3) Route wiring

Added router entry in `edgequake-api/src/routes.rs`:
- `.route("/documents/pdf/batch", post(handlers::upload_pdf_batch_document))`

## DRY decisions

- Extracted single-file PDF processing core into `process_pdf_upload_parts` and reused from both single and batch handlers.
- Kept one duplicate/reindex logic path to avoid behavior drift between endpoints.
