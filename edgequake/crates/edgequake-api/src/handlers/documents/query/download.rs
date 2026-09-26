//! Document download handlers (original bytes + markdown).

use axum::extract::{Path, State};
use axum::http::header;
use axum::response::IntoResponse;
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::document_body_loader::load_document_body;
use crate::services::document_metadata_scan::metadata_key_for_document;
use crate::state::AppState;
use edgequake_storage::PdfDocumentStorage;

#[cfg(feature = "postgres")]
fn get_pdf_storage(state: &AppState) -> ApiResult<std::sync::Arc<dyn PdfDocumentStorage>> {
    if state.storage.is_postgresql() {
        state
            .storage
            .validate_postgres_adapters()
            .map_err(ApiError::Internal)?;
    }
    state
        .storage
        .pdf_storage
        .as_ref()
        .cloned()
        .ok_or_else(|| ApiError::Internal("PDF storage not initialized".into()))
}

#[cfg(not(feature = "postgres"))]
fn get_pdf_storage(_state: &AppState) -> ApiResult<std::sync::Arc<dyn PdfDocumentStorage>> {
    Err(ApiError::Internal(
        "PDF storage not available (postgres feature disabled)".into(),
    ))
}

#[cfg(feature = "postgres")]
fn get_original_storage(
    state: &AppState,
) -> ApiResult<std::sync::Arc<dyn edgequake_storage::DocumentOriginalStorage>> {
    if state.storage.is_postgresql() {
        state
            .storage
            .validate_postgres_adapters()
            .map_err(ApiError::Internal)?;
    }
    state
        .storage
        .original_storage
        .as_ref()
        .cloned()
        .ok_or_else(|| ApiError::Internal("Original storage not initialized".into()))
}

async fn load_document_metadata(state: &AppState, document_id: &str) -> ApiResult<Value> {
    let metadata_key = metadata_key_for_document(document_id);

    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        if let Some(value) =
            edgequake_storage::adapters::postgres::document_shell::shell_value_by_key(
                pool,
                &metadata_key,
            )
            .await
            .map_err(ApiError::from)?
        {
            return Ok(value);
        }
    }

    let metadata = state
        .storage
        .kv_storage
        .get_by_ids(&[metadata_key])
        .await
        .map_err(ApiError::from)?
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::NotFound(format!("Document not found: {document_id}")))?;
    Ok(metadata)
}

fn metadata_pdf_id(metadata: &Value) -> Option<Uuid> {
    metadata
        .get("pdf_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

fn attachment_filename(name: &str) -> String {
    format!("attachment; filename=\"{}\"", name.replace('"', ""))
}

/// Download the original document bytes (PDF delegation or stored upload).
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/download/original",
    params(("document_id" = String, Path, description = "Document identifier")),
    responses(
        (status = 200, description = "Original file bytes"),
        (status = 404, description = "Document or original not found"),
        (status = 403, description = "Not authorized"),
    ),
    tag = "Documents"
)]
pub async fn download_document_original(
    State(state): State<AppState>,
    context: TenantContext,
    Path(document_id): Path<String>,
) -> ApiResult<axum::response::Response<axum::body::Body>> {
    let workspace_id = Uuid::parse_str(&context.workspace_id_or_default())
        .map_err(|_| ApiError::BadRequest("Invalid workspace id".into()))?;

    let metadata = load_document_metadata(&state, &document_id).await?;

    if let Some(pdf_id) = metadata_pdf_id(&metadata) {
        let pdf_storage = get_pdf_storage(&state)?;
        let pdf = pdf_storage
            .get_pdf(&pdf_id)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {e}")))?
            .ok_or_else(|| ApiError::NotFound("PDF not found".into()))?;

        if pdf.workspace_id != workspace_id {
            return Err(ApiError::forbidden());
        }

        info!(
            document_id = %document_id,
            pdf_id = %pdf_id,
            filename = %pdf.filename,
            "Document original download via PDF storage"
        );

        return Ok((
            [
                (header::CONTENT_TYPE, pdf.content_type.as_str()),
                (
                    header::CONTENT_DISPOSITION,
                    attachment_filename(&pdf.filename).as_str(),
                ),
                (header::CACHE_CONTROL, "private, max-age=3600"),
            ],
            pdf.pdf_data,
        )
            .into_response());
    }

    #[cfg(feature = "postgres")]
    {
        let document_uuid = Uuid::parse_str(&document_id)
            .map_err(|_| ApiError::BadRequest("Invalid document id".into()))?;
        let original_storage = get_original_storage(&state)?;
        let original = original_storage
            .get_original(&workspace_id, &document_uuid)
            .await
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::NotFound("Original file not found".into()))?;

        info!(
            document_id = %document_id,
            filename = %original.filename,
            "Document original download via document_originals"
        );

        Ok((
            [
                (header::CONTENT_TYPE, original.content_type.as_str()),
                (
                    header::CONTENT_DISPOSITION,
                    attachment_filename(&original.filename).as_str(),
                ),
                (header::CACHE_CONTROL, "private, max-age=3600"),
            ],
            original.original_data,
        )
            .into_response())
    }

    #[cfg(not(feature = "postgres"))]
    Err(ApiError::NotFound("Original file not found".into()))
}

/// Download extracted markdown for a document.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/download/markdown",
    params(("document_id" = String, Path, description = "Document identifier")),
    responses(
        (status = 200, description = "Markdown file", content_type = "text/markdown"),
        (status = 404, description = "Document or markdown not found"),
    ),
    tag = "Documents"
)]
pub async fn download_document_markdown(
    State(state): State<AppState>,
    _context: TenantContext,
    Path(document_id): Path<String>,
) -> ApiResult<axum::response::Response<axum::body::Body>> {
    let metadata = load_document_metadata(&state, &document_id).await?;
    let body = load_document_body(&state.storage, &document_id, &metadata)
        .await
        .ok_or_else(|| ApiError::NotFound("Markdown content not found".into()))?;

    let filename = metadata
        .get("file_name")
        .or_else(|| metadata.get("title"))
        .and_then(|v| v.as_str())
        .map(|name| {
            if name.to_lowercase().ends_with(".md") {
                name.to_string()
            } else {
                format!("{name}.md")
            }
        })
        .unwrap_or_else(|| format!("{document_id}.md"));

    info!(document_id = %document_id, filename = %filename, "Document markdown download");

    Ok((
        [
            (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                attachment_filename(&filename).as_str(),
            ),
            (header::CACHE_CONTROL, "private, max-age=3600"),
        ],
        body.markdown,
    )
        .into_response())
}
