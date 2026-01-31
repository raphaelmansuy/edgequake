# Iteration 14: Decide

## Decision

Implement `GET /api/v1/documents/pdf/progress/{track_id}` endpoint that returns `PdfUploadProgress` JSON.

## Rationale

1. **Direct key lookup**: Progress is stored by `track_id`, so querying by `track_id` is O(1)
2. **Already Serialize**: `PdfUploadProgress` derives `Serialize`, can return directly
3. **Client has track_id**: Upload response includes `track_id`, client already has it
4. **Foundation for WebSocket**: Same data model will be used for `/ws/progress/{track_id}`

## Action Items

1. [x] Add `get_pdf_progress` handler function in `pdf_upload.rs`
2. [x] Add route in `routes.rs` (before `/documents/{document_id}` catch-all)
3. [x] Add utoipa OpenAPI documentation
4. [x] Return 404 if progress not found
5. [x] Add test for the new endpoint

## Success Metrics

- [x] `cargo test --package edgequake-api --lib` compiles
- [x] Handler returns `PdfUploadProgress` JSON on success
- [x] Handler returns 404 if track_id not found
- [x] OpenAPI docs generated correctly

## Testing Strategy

- Unit test: Create callback → fire events → query endpoint → verify response
- Need AppState with PipelineState that has progress
- Mock the flow or use integration test

## Handler Signature

```rust
/// Get PDF upload progress by track ID.
///
/// @implements SPEC-001-upload-pdf: Progress query endpoint
/// @implements OODA-14: GET progress endpoint
#[utoipa::path(
    get,
    path = "/api/v1/documents/pdf/progress/{track_id}",
    params(
        ("track_id" = String, Path, description = "Upload tracking ID")
    ),
    responses(
        (status = 200, description = "Progress data", body = PdfUploadProgress),
        (status = 404, description = "Progress not found (completed or not started)"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Documents"
)]
pub async fn get_pdf_progress(
    State(state): State<AppState>,
    Path(track_id): Path<String>,
) -> ApiResult<Json<PdfUploadProgress>> {
    let progress = state
        .pipeline_state
        .get_pdf_progress(&track_id)
        .await
        .ok_or_else(|| ApiError::NotFound(
            "Progress not found. Upload may have completed or not yet started.".to_string()
        ))?;

    Ok(Json(progress))
}
```
