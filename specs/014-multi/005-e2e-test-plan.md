# SPEC-014 — E2E Test Plan

## Rust API E2E (PostgreSQL feature)

File: `edgequake/crates/edgequake-api/tests/e2e_spec014_multi_upload.rs`

Tests:

1. `spec014_batch_text_upload_accepts_multiple_files`
   - Creates tenant/workspace
   - Uploads 2 text files in one multipart request to `/api/v1/documents/upload/batch`
   - Asserts `201`, `total_files=2`, `failed=0`

2. `spec014_batch_pdf_upload_accepts_multiple_files`
   - Creates tenant/workspace
   - Uploads 2 PDFs in one multipart request to `/api/v1/documents/pdf/batch`
   - Asserts `200`, `total_files=2`, `failed=0`, per-file statuses present

## Playwright E2E

File: `edgequake_webui/e2e/issue-236-batch-upload-api.spec.ts`

Tests:

1. Swagger contract visibility
   - Opens `/swagger-ui`
   - Asserts visible:
     - `/api/v1/documents/upload/batch`
     - `/api/v1/documents/pdf/batch`
   - Writes screenshot to `specs/014-multi/e2e/screenshosts/001-swagger-batch-endpoints.png`

2. API batch text upload from one request
3. API batch PDF upload from one request

## Gate command

```bash
cd edgequake
cargo test -p edgequake-api --features postgres --test e2e_spec014_multi_upload -- --nocapture

cd ../edgequake_webui
pnpm exec playwright test --config playwright.spec013-ui.config.ts issue-236-batch-upload-api.spec.ts
```
