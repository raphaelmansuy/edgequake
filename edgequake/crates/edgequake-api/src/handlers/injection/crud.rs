//! Knowledge injection CRUD handlers.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::{cleanup_document_graph_data, injection_doc_id, injection_meta_key};
use crate::state::AppState;

use super::helpers::{
    build_meta, detail_from_meta, enqueue_injection_processing, resolve_injection_context,
    str_field, str_field_or, validate_content, validate_name, workspace_id_from_tenant,
    InjectionEnqueueParams,
};
use crate::handlers::injection_types::*;

/// Create or update a knowledge injection entry.
#[utoipa::path(
    put,
    path = "/api/v1/workspaces/{workspace_id}/injection",
    tag = "Knowledge Injection",
    request_body = PutInjectionRequest,
    responses(
        (status = 202, description = "Injection accepted for processing", body = PutInjectionResponse),
        (status = 400, description = "Invalid request"),
        (status = 413, description = "Content too large")
    )
)]
pub async fn put_injection(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<PutInjectionRequest>,
) -> ApiResult<(StatusCode, Json<PutInjectionResponse>)> {
    let workspace_id = workspace_id_from_tenant(&tenant_ctx);
    let name = request.name.trim().to_string();
    validate_name(&name)?;
    validate_content(&request.content)?;

    let injection_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let doc_id = injection_doc_id(&workspace_id, &injection_id);

    let meta = build_meta(
        &injection_id,
        &name,
        &request.content,
        &workspace_id,
        "text",
        None,
        "processing",
        1,
        0,
        None,
        &doc_id,
        &now,
        &now,
        None,
    );

    let meta_key = injection_meta_key(&workspace_id, &injection_id);
    state
        .storage
        .kv_storage
        .upsert(&[(meta_key.clone(), meta)])
        .await?;

    info!(
        workspace_id = %workspace_id,
        injection_id = %injection_id,
        content_len = request.content.len(),
        "Created knowledge injection entry"
    );

    let data_tenant_id = resolve_injection_context(&state, &workspace_id)
        .await
        .data_tenant_id;

    enqueue_injection_processing(
        &state,
        &tenant_ctx,
        InjectionEnqueueParams {
            doc_id,
            content: request.content,
            workspace_id: workspace_id.clone(),
            meta_key,
            injection_id: injection_id.clone(),
            name,
            source_type: "text".to_string(),
            source_filename: None,
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

/// List all injection entries for a workspace.
#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/injections",
    tag = "Knowledge Injection",
    responses(
        (status = 200, description = "Injection entries listed", body = ListInjectionsResponse)
    )
)]
pub async fn list_injections(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Query(query): Query<ListInjectionsQuery>,
) -> ApiResult<Json<ListInjectionsResponse>> {
    let workspace_id = workspace_id_from_tenant(&tenant_ctx);
    let page = crate::services::list_injections_paged(
        &state.storage.kv_storage,
        &workspace_id,
        query.limit,
        query.offset,
    )
    .await
    .map_err(|e| ApiError::Internal(format!("Injection list failed: {e}")))?;

    let has_more = page.has_more();
    Ok(Json(ListInjectionsResponse {
        items: page.items,
        total: page.total,
        limit: page.limit,
        offset: page.offset,
        has_more,
    }))
}

/// Get a single injection entry detail.
#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/injections/{injection_id}",
    tag = "Knowledge Injection",
    responses(
        (status = 200, description = "Injection detail", body = InjectionDetailResponse),
        (status = 404, description = "Injection not found")
    )
)]
pub async fn get_injection(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path((_workspace_id_path, injection_id)): Path<(String, String)>,
) -> ApiResult<Json<InjectionDetailResponse>> {
    let workspace_id = workspace_id_from_tenant(&tenant_ctx);
    let meta_key = injection_meta_key(&workspace_id, &injection_id);
    let val = state
        .storage
        .kv_storage
        .get_by_id(&meta_key)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Injection {} not found", injection_id)))?;
    Ok(Json(detail_from_meta(&val)))
}

/// Delete an injection entry and all its artifacts.
#[utoipa::path(
    delete,
    path = "/api/v1/workspaces/{workspace_id}/injections/{injection_id}",
    tag = "Knowledge Injection",
    responses(
        (status = 200, description = "Injection deleted", body = DeleteInjectionResponse),
        (status = 404, description = "Injection not found")
    )
)]
pub async fn delete_injection(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path((_workspace_id_path, injection_id)): Path<(String, String)>,
) -> ApiResult<Json<DeleteInjectionResponse>> {
    let workspace_id = workspace_id_from_tenant(&tenant_ctx);
    let meta_key = injection_meta_key(&workspace_id, &injection_id);

    let meta_val = state
        .storage
        .kv_storage
        .get_by_id(&meta_key)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Injection {} not found", injection_id)))?;

    let doc_id = injection_doc_id(&workspace_id, &injection_id);

    let vector_storage = resolve_injection_context(&state, &workspace_id)
        .await
        .vector_storage;

    if let Err(e) =
        cleanup_document_graph_data(&doc_id, &state.storage.graph_storage, Some(&vector_storage))
            .await
    {
        warn!(
            injection_id = %injection_id,
            error = %e,
            "Graph cleanup during injection delete had errors (continuing)"
        );
    }

    if let Some(chunk_ids) = meta_val.get("chunk_ids").and_then(|v| v.as_array()) {
        let ids: Vec<String> = chunk_ids
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        if !ids.is_empty() {
            if let Err(e) = vector_storage.delete(&ids).await {
                warn!(injection_id = %injection_id, error = %e, "Failed to delete injection chunk vectors");
            }
        }
    }

    let keys = state.storage.kv_storage.keys_with_prefix(&doc_id).await?;
    let mut kv_ids_to_delete: Vec<String> = keys;
    if !kv_ids_to_delete.iter().any(|k| k == &meta_key) {
        kv_ids_to_delete.push(meta_key);
    }
    if !kv_ids_to_delete.is_empty() {
        debug!(
            count = kv_ids_to_delete.len(),
            "Deleting injection KV entries"
        );
        let _ = state.storage.kv_storage.delete(&kv_ids_to_delete).await;
    }

    info!(
        injection_id = %injection_id,
        workspace_id = %workspace_id,
        "Injection deleted: graph, entity vectors, chunk vectors, and KV entries purged"
    );

    Ok(Json(DeleteInjectionResponse {
        deleted: true,
        message: format!("Injection {} deleted", injection_id),
    }))
}

/// Update an existing injection entry. Re-processes if content changes.
#[utoipa::path(
    patch,
    path = "/api/v1/workspaces/{workspace_id}/injections/{injection_id}",
    tag = "Knowledge Injection",
    request_body = UpdateInjectionRequest,
    responses(
        (status = 200, description = "Injection updated", body = PutInjectionResponse),
        (status = 404, description = "Injection not found")
    )
)]
pub async fn update_injection(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path((_workspace_id_path, injection_id)): Path<(String, String)>,
    Json(request): Json<UpdateInjectionRequest>,
) -> ApiResult<Json<PutInjectionResponse>> {
    let workspace_id = workspace_id_from_tenant(&tenant_ctx);
    let meta_key = injection_meta_key(&workspace_id, &injection_id);

    let existing = state
        .storage
        .kv_storage
        .get_by_id(&meta_key)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Injection {} not found", injection_id)))?;

    let old_name = str_field(&existing, "name");
    let old_content = str_field(&existing, "content");
    let old_version = existing
        .get("version")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;
    let created_at = str_field(&existing, "created_at");
    let source_type = str_field_or(&existing, "source_type", "text");
    let source_filename: Option<String> = existing
        .get("source_filename")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let new_name = request
        .name
        .as_deref()
        .map(|n| n.trim().to_string())
        .unwrap_or(old_name);
    validate_name(&new_name)?;

    let content_changed = request.content.is_some();
    let new_content = request.content.unwrap_or(old_content);
    validate_content(&new_content)?;

    let new_version = if content_changed {
        old_version + 1
    } else {
        old_version
    };
    let doc_id = injection_doc_id(&workspace_id, &injection_id);
    let status = if content_changed {
        "processing"
    } else {
        existing
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("completed")
    };

    let now = Utc::now().to_rfc3339();
    let meta = build_meta(
        &injection_id,
        &new_name,
        &new_content,
        &workspace_id,
        &source_type,
        source_filename.as_deref(),
        status,
        new_version,
        existing
            .get("entity_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        None,
        &doc_id,
        &created_at,
        &now,
        None,
    );
    state
        .storage
        .kv_storage
        .upsert(&[(meta_key.clone(), meta)])
        .await?;

    info!(injection_id = %injection_id, content_changed, new_version, "Updated injection entry");

    if content_changed {
        let data_tenant_id = resolve_injection_context(&state, &workspace_id)
            .await
            .data_tenant_id;
        enqueue_injection_processing(
            &state,
            &tenant_ctx,
            InjectionEnqueueParams {
                doc_id,
                content: new_content,
                workspace_id: workspace_id.clone(),
                meta_key,
                injection_id: injection_id.clone(),
                name: new_name,
                source_type,
                source_filename,
                version: new_version,
                created_at,
                data_tenant_id,
            },
        )
        .await?;
    }

    Ok(Json(PutInjectionResponse {
        injection_id,
        workspace_id,
        version: new_version,
        status: status.to_string(),
    }))
}
