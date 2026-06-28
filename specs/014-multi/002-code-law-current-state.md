# SPEC-014 — Code Law (Before Fix)

## Ground truth from code

### Existing router behavior

- `routes.rs` already had:
  - `POST /api/v1/documents/upload/batch`
  - `POST /api/v1/documents/pdf` (single file)
- Missing:
  - `POST /api/v1/documents/pdf/batch`

### Existing OpenAPI behavior

`openapi.rs` included:
- `handlers::upload_document`
- `handlers::upload_pdf_document`

But omitted:
- `handlers::upload_file`
- `handlers::upload_files_batch`
- any PDF batch path/schema

Result: Swagger could appear to support single uploads only, even though some batch logic existed in router code.

### Existing document batch behavior

`handlers/documents/upload/batch_upload.rs` supports repeated multipart fields (`file`/`files`) and returns `BatchUploadResponse`.

### Existing PDF behavior

`handlers/pdf_upload/upload.rs` implemented only single-file multipart parsing and upload orchestration.

## Root causes

1. **API contract drift**: router paths and OpenAPI path list diverged.
2. **Capability gap**: PDF path had single-file implementation only.

## Constraint

Fix must preserve tenant/workspace scoping and duplicate semantics used by existing single upload flow.
