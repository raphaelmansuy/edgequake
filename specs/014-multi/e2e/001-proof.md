# SPEC-014 — E2E Proof

Issue: [#236](https://github.com/raphaelmansuy/edgequake/issues/236)

## 1) Rust API E2E proof

Command:

```bash
cd edgequake
cargo test -p edgequake-api --features postgres --test e2e_spec014_multi_upload -- --nocapture
```

Result:

- `spec014_batch_text_upload_accepts_multiple_files` ✅
- `spec014_batch_pdf_upload_accepts_multiple_files` ✅
- Summary: `2 passed; 0 failed`

## 2) Playwright E2E proof

Command:

```bash
cd edgequake_webui
E2E_BACKEND_URL=http://localhost:8083 \
SPEC013_BACKEND_URL=http://localhost:8083 \
PLAYWRIGHT_BASE_URL=http://localhost:3001 \
pnpm exec playwright test --config playwright.spec013-ui.config.ts issue-236-batch-upload-api.spec.ts
```

Result:

- `openapi exposes batch upload endpoints` ✅
- `batch document upload ingests multiple text files in one request` ✅
- `batch PDF upload accepts multiple PDFs in one request` ✅
- Summary: `3 passed; 0 failed`

## 3) Screenshot artifacts

- `specs/014-multi/e2e/screenshosts/001-swagger-batch-endpoints.png`
- Validation: screenshot is captured from `http://localhost:8083/swagger-ui/` (backend Swagger UI), not frontend `/swagger-ui` route.

## 4) Contract verification

OpenAPI includes:

- `/api/v1/documents/upload/batch`
- `/api/v1/documents/pdf/batch`
