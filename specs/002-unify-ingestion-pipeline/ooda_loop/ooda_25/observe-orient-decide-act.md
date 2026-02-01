# OODA-25: Swagger/OpenAPI Documentation

**Date**: 2025-01-27  
**Focus**: API Documentation Updates

## OBSERVE

### Current API Endpoints (PDF-related)

```rust
// edgequake/crates/edgequake-api/src/handlers/pdf_upload.rs

// POST /api/v1/documents/upload
pub async fn upload_document(...)

// GET /api/v1/documents/{id}/pdf/download
pub async fn download_pdf(...)

// GET /api/v1/documents/{id}/pdf/content
pub async fn get_pdf_content(...)
```

### Current OpenAPI Status

```rust
// edgequake/crates/edgequake-api/src/main.rs
// Uses utoipa for OpenAPI generation
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::documents::list_documents,
        handlers::documents::get_document,
        // ... more paths
    ),
    // ...
)]
```

### Documentation Coverage Check

| Endpoint                           | OpenAPI Doc | Status |
| ---------------------------------- | ----------- | ------ |
| `POST /documents/upload`           | To verify   | Check  |
| `GET /documents/{id}/pdf/download` | To verify   | Check  |
| `GET /documents/{id}/pdf/content`  | To verify   | Check  |
| `GET /documents/{id}/markdown`     | To verify   | Check  |

## ORIENT

### First Principle: Self-Documenting API

- API consumers should understand endpoints without external docs
- Request/response schemas must be accurate
- Examples help users get started quickly

### utoipa Integration Pattern

```rust
/// Download PDF for a document
///
/// Returns the original PDF file for the specified document.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/pdf/download",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "PDF file stream", content_type = "application/pdf"),
        (status = 404, description = "Document not found"),
        (status = 403, description = "Workspace access denied")
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn download_pdf(...) -> ...
```

## DECIDE

**Decision**: Audit and verify utoipa annotations on PDF handlers

### Verification Steps

1. Check all PDF handlers have `#[utoipa::path]` annotations
2. Verify response types include error cases
3. Confirm multi-tenancy (workspace isolation) documented
4. Test generated `/api-docs` endpoint

### Key Documentation Points

- **Multi-tenancy**: All endpoints require workspace context (X-Workspace-ID header)
- **Authentication**: ApiKey header required
- **Content Types**: PDF endpoints return `application/pdf`

## ACT

### Investigation

Need to read actual handler code to verify OpenAPI annotations.

### Documentation Priorities

1. Response error codes (403, 404, 500)
2. Multi-tenancy header requirements
3. File upload multipart schema
4. Rate limiting information

### Next Steps

- Read pdf_upload.rs handlers
- Verify utoipa annotations present
- Add any missing documentation

**Status**: IN PROGRESS - Needs code verification

### Evidence Required

- [ ] Screenshot of /api-docs showing PDF endpoints
- [ ] Verify request/response schemas accurate
- [ ] Test API through Swagger UI
