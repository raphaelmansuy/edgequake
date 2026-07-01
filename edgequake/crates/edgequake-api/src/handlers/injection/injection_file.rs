//! Knowledge injection file upload handler.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::Multipart;
use chrono::Utc;
use tracing::{debug, info};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::{injection_doc_id, injection_meta_key};
use crate::state::AppState;

use super::helpers::{
    build_meta, enqueue_injection_processing, resolve_injection_context, validate_content,
    workspace_id_from_tenant, InjectionEnqueueParams,
};
use crate::handlers::injection_types::PutInjectionResponse;

/// Create a knowledge injection from an uploaded file (plain-text formats).
#[utoipa::path(
    put,
    path = "/api/v1/workspaces/{workspace_id}/injection/file",
    tag = "Knowledge Injection",
    request_body(content_type = "multipart/form-data", description = "File to inject"),
    responses(
        (status = 202, description = "Injection accepted for processing", body = PutInjectionResponse),
        (status = 400, description = "Invalid file or request"),
        (status = 413, description = "File too large")
    )
)]
pub async fn put_injection_file(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<PutInjectionResponse>)> {
    let workspace_id = workspace_id_from_tenant(&tenant_ctx);

    const MAX_FILE_BYTES: usize = edgequake_core::MAX_UPLOAD_BYTES;

    let mut filename = String::new();
    let mut name = String::new();
    let mut file_bytes: Vec<u8> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {e}")))?
    {
        match field.name().unwrap_or("") {
            "file" => {
                filename = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "injection.txt".to_string());
                file_bytes = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {e}")))?
                    .to_vec();
            }
            "name" => {
                name = field
                    .text()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read name: {e}")))?
                    .trim()
                    .to_string();
            }
            _ => {}
        }
    }

    if file_bytes.is_empty() {
        return Err(ApiError::BadRequest("No file provided".to_string()));
    }

    if file_bytes.len() > MAX_FILE_BYTES {
        return Err(ApiError::BadRequest(format!(
            "File exceeds 10 MB limit ({} bytes)",
            file_bytes.len()
        )));
    }

    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    const ALLOWED: [&str; 4] = ["txt", "md", "csv", "json"];
    if !ALLOWED.contains(&ext.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Unsupported file type '.{ext}'. Allowed: txt, md, csv, json"
        )));
    }

    let content = String::from_utf8(file_bytes)
        .map_err(|_| ApiError::BadRequest("File must be valid UTF-8 text".to_string()))?;
    validate_content(&content)?;

    if name.is_empty() {
        name = filename
            .rsplit('/')
            .next()
            .unwrap_or(&filename)
            .rsplit('.')
            .nth(1)
            .or_else(|| filename.rsplit('/').next())
            .unwrap_or("Injection")
            .to_string();
    }

    if name.len() > 100 {
        name.truncate(100);
    }

    debug!(
        workspace_id = %workspace_id,
        filename = %filename,
        content_len = content.len(),
        "Creating file injection"
    );

    let injection_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let doc_id = injection_doc_id(&workspace_id, &injection_id);

    let meta_key = injection_meta_key(&workspace_id, &injection_id);
    let meta = build_meta(
        &injection_id,
        &name,
        &content,
        &workspace_id,
        "file",
        Some(&filename),
        "processing",
        1,
        0,
        None,
        &doc_id,
        &now,
        &now,
        None,
    );
    state
        .storage
        .kv_storage
        .upsert(&[(meta_key.clone(), meta)])
        .await?;

    info!(
        workspace_id = %workspace_id,
        injection_id = %injection_id,
        filename = %filename,
        "Created file injection entry"
    );

    let data_tenant_id = resolve_injection_context(&state, &workspace_id)
        .await
        .data_tenant_id;
    enqueue_injection_processing(
        &state,
        &tenant_ctx,
        InjectionEnqueueParams {
            doc_id,
            content,
            workspace_id: workspace_id.clone(),
            meta_key,
            injection_id: injection_id.clone(),
            name,
            source_type: "file".to_string(),
            source_filename: Some(filename),
            version: 1,
            created_at: now,
            data_tenant_id,
        },
    )
    .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(PutInjectionResponse {
            injection_id,
            workspace_id,
            version: 1,
            status: "processing".to_string(),
        }),
    ))
}
