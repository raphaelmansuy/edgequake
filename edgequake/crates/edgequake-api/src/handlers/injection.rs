//! Knowledge injection handlers — PUT, GET, LIST, DELETE.
//!
//! @implements SPEC-0002 (Knowledge Injection for Enhanced Search)
//!
//! Injection entries are stored in KV as `injection::{workspace_id}::{injection_id}-metadata`
//! and processed through the standard pipeline with `source_type = "injection"` tagging.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::Multipart;
use chrono::Utc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;

pub use super::injection_types::*;

/// Stable document ID prefix for injection artifacts.
fn injection_doc_id(workspace_id: &str, injection_id: &str) -> String {
    format!("injection::{}::{}", workspace_id, injection_id)
}

/// KV metadata key for an injection entry.
fn injection_meta_key(workspace_id: &str, injection_id: &str) -> String {
    format!("injection::{}::{}-metadata", workspace_id, injection_id)
}

// ─────────────────────────────────────────────────────────────────────────────
// DRY Primitives — validation, serialization, background task
// ─────────────────────────────────────────────────────────────────────────────

/// Extract the effective workspace ID from the tenant context.
fn workspace_id_from_tenant(ctx: &TenantContext) -> String {
    ctx.workspace_id_or_default()
}

/// Validate injection name: non-empty, max 100 chars.
fn validate_name(name: &str) -> ApiResult<()> {
    if name.is_empty() || name.len() > 100 {
        return Err(ApiError::BadRequest(
            "Name must be between 1 and 100 characters".to_string(),
        ));
    }
    Ok(())
}

/// Validate injection content: non-empty, under global size limit.
fn validate_content(content: &str) -> ApiResult<()> {
    if content.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Injection content cannot be empty".to_string(),
        ));
    }
    if content.len() > MAX_INJECTION_CONTENT_BYTES {
        return Err(ApiError::BadRequest(format!(
            "Injection content exceeds {}KB limit",
            MAX_INJECTION_CONTENT_BYTES / 1024
        )));
    }
    Ok(())
}

/// Get a string field from a JSON value, defaulting to `""`.
#[inline]
fn str_field(val: &serde_json::Value, key: &str) -> String {
    val.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Get a string field from a JSON value with a custom default.
#[inline]
fn str_field_or(val: &serde_json::Value, key: &str, default: &str) -> String {
    val.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

/// Build the canonical JSON metadata record for an injection KV entry.
///
/// WHY: All three create/update paths (text PUT, file PUT, PATCH) produce the same
/// metadata shape.  A single builder prevents fields drifting out of sync across copies.
#[allow(clippy::too_many_arguments)]
fn build_meta(
    injection_id: &str,
    name: &str,
    content: &str,
    workspace_id: &str,
    source_type: &str,
    source_filename: Option<&str>,
    status: &str,
    version: u32,
    entity_count: u32,
    chunk_ids: Option<&[String]>,
    doc_id: &str,
    created_at: &str,
    updated_at: &str,
    error: Option<&str>,
) -> serde_json::Value {
    let mut v = serde_json::json!({
        "id": injection_id,
        "name": name,
        "content": content,
        "workspace_id": workspace_id,
        "source_type": source_type,
        "status": status,
        "version": version,
        "entity_count": entity_count,
        "source_document_id": doc_id,
        "created_at": created_at,
        "updated_at": updated_at,
    });
    if let Some(ids) = chunk_ids {
        v["chunk_ids"] = serde_json::json!(ids);
    }
    if let Some(fname) = source_filename {
        v["source_filename"] = serde_json::json!(fname);
    }
    if let Some(err) = error {
        v["error"] = serde_json::json!(err);
    }
    v
}

/// Deserialize a JSON KV value into an `InjectionSummary`.
fn summary_from_meta(val: &serde_json::Value) -> InjectionSummary {
    InjectionSummary {
        injection_id: str_field(val, "id"),
        name: str_field(val, "name"),
        status: str_field_or(val, "status", "unknown"),
        entity_count: val
            .get("entity_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        source_type: str_field_or(val, "source_type", "text"),
        error: val
            .get("error")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        created_at: str_field(val, "created_at"),
        updated_at: str_field(val, "updated_at"),
    }
}

/// Deserialize a JSON KV value into an `InjectionDetailResponse`.
fn detail_from_meta(val: &serde_json::Value) -> InjectionDetailResponse {
    InjectionDetailResponse {
        injection_id: str_field(val, "id"),
        name: str_field(val, "name"),
        content: str_field(val, "content"),
        version: val.get("version").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
        status: str_field_or(val, "status", "unknown"),
        entity_count: val
            .get("entity_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        source_type: str_field_or(val, "source_type", "text"),
        error: val
            .get("error")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        created_at: str_field(val, "created_at"),
        updated_at: str_field(val, "updated_at"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Background task helpers
// ─────────────────────────────────────────────────────────────────────────────

/// All context needed to run an injection pipeline task and record its result.
struct InjectionTaskContext {
    pipeline: std::sync::Arc<edgequake_pipeline::Pipeline>,
    graph_storage: std::sync::Arc<dyn edgequake_storage::traits::GraphStorage>,
    vector_storage: std::sync::Arc<dyn edgequake_storage::traits::VectorStorage>,
    kv_storage: std::sync::Arc<dyn edgequake_storage::traits::KVStorage>,
    doc_id: String,
    content: String,
    workspace_id: String,
    data_tenant_id: Option<String>,
    meta_key: String,
    injection_id: String,
    name: String,
    source_type: String,
    source_filename: Option<String>,
    version: u32,
    created_at: String,
}

/// Spawn a background task that runs the injection pipeline and writes back
/// success/failure metadata to KV.
///
/// WHY: All three create paths (text PUT, file PUT, PATCH content-changed) share
/// the same spawn-and-record pattern.  Centralising it prevents the copies from
/// drifting (e.g. one forgetting to persist `chunk_ids`).
fn spawn_injection_processing(ctx: InjectionTaskContext) {
    tokio::spawn(async move {
        match process_injection_pipeline(
            &ctx.pipeline,
            ctx.graph_storage,
            ctx.vector_storage,
            &ctx.doc_id,
            &ctx.content,
            &ctx.workspace_id,
            ctx.data_tenant_id,
        )
        .await
        {
            Ok((entity_count, chunk_ids)) => {
                let meta = build_meta(
                    &ctx.injection_id,
                    &ctx.name,
                    &ctx.content,
                    &ctx.workspace_id,
                    &ctx.source_type,
                    ctx.source_filename.as_deref(),
                    "completed",
                    ctx.version,
                    entity_count,
                    Some(&chunk_ids),
                    &ctx.doc_id,
                    &ctx.created_at,
                    &Utc::now().to_rfc3339(),
                    None,
                );
                let _ = ctx.kv_storage.upsert(&[(ctx.meta_key, meta)]).await;
                info!(
                    injection_id = %ctx.injection_id,
                    entity_count,
                    "Injection processing completed"
                );
            }
            Err(e) => {
                warn!(
                    injection_id = %ctx.injection_id,
                    error = %e,
                    "Injection processing failed"
                );
                let meta = build_meta(
                    &ctx.injection_id,
                    &ctx.name,
                    &ctx.content,
                    &ctx.workspace_id,
                    &ctx.source_type,
                    ctx.source_filename.as_deref(),
                    "failed",
                    ctx.version,
                    0,
                    None,
                    &ctx.doc_id,
                    &ctx.created_at,
                    &Utc::now().to_rfc3339(),
                    Some(&e.to_string()),
                );
                let _ = ctx.kv_storage.upsert(&[(ctx.meta_key, meta)]).await;
            }
        }
    });
}

// ============================================================================
// PUT /api/v1/workspaces/:workspace_id/injection  — Create or replace
// ============================================================================

/// Create or update a knowledge injection entry.
///
/// Processes the content through the standard pipeline with `source_type = "injection"` tagging.
/// Injection entities enrich the knowledge graph but are excluded from query source citations.
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

    // WHY: Resolve workspace-specific vector storage BEFORE the spawn so injection
    // vectors land in the same table the query engine searches (SPEC-033).
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

    // WHY: Use workspace-specific pipeline to ensure embedding dimensions match the
    // workspace's vector storage table. Using the global pipeline (e.g. Ollama 768-dim)
    // with a workspace configured for OpenAI (1536-dim) causes silent dimension mismatch
    // errors in merge_entity, producing entity_count=0 despite successful LLM extraction.
    let workspace_pipeline = state.create_workspace_pipeline(&workspace_id).await;
    let inj_ctx = resolve_injection_context(&state, &workspace_id).await;
    spawn_injection_processing(InjectionTaskContext {
        pipeline: workspace_pipeline,
        graph_storage: state.storage.graph_storage.clone(),
        vector_storage: inj_ctx.vector_storage,
        kv_storage: state.storage.kv_storage.clone(),
        doc_id,
        content: request.content,
        workspace_id: workspace_id.clone(),
        data_tenant_id: inj_ctx.data_tenant_id,
        meta_key,
        injection_id: injection_id.clone(),
        name,
        source_type: "text".to_string(),
        source_filename: None,
        version: 1,
        created_at: now,
    });

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

// ============================================================================
// GET /api/v1/workspaces/:workspace_id/injections  — List all
// ============================================================================

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
) -> ApiResult<Json<ListInjectionsResponse>> {
    let workspace_id = workspace_id_from_tenant(&tenant_ctx);
    let prefix = format!("injection::{}", workspace_id);
    let keys = state.storage.kv_storage.keys().await?;

    let mut items: Vec<InjectionSummary> = Vec::new();
    for key in keys
        .iter()
        .filter(|k| k.starts_with(&prefix) && k.ends_with("-metadata"))
    {
        if let Ok(Some(val)) = state.storage.kv_storage.get_by_id(key).await {
            items.push(summary_from_meta(&val));
        }
    }

    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let total = items.len();
    Ok(Json(ListInjectionsResponse { items, total }))
}

// ============================================================================
// GET /api/v1/workspaces/:workspace_id/injections/:injection_id  — Detail
// ============================================================================

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

// ============================================================================
// DELETE /api/v1/workspaces/:workspace_id/injections/:injection_id
// ============================================================================

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

    // Load metadata first — needed for chunk_ids before KV deletion.
    let meta_val = state
        .storage
        .kv_storage
        .get_by_id(&meta_key)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Injection {} not found", injection_id)))?;

    let doc_id = injection_doc_id(&workspace_id, &injection_id);

    // Resolve workspace-specific vector storage once (SPEC-033).
    let vector_storage = resolve_injection_context(&state, &workspace_id)
        .await
        .vector_storage;

    // 1. Clean graph entities/relationships sourced from this injection and their
    //    entity vector embeddings.
    //    WHY: Injection pipeline writes graph nodes with source_ids pointing to doc_id.
    //    Leaving those nodes pollutes future queries with stale knowledge (TS-004).
    //    This reuses the same cleanup path as document deletion (SRP/DRY).
    if let Err(e) = crate::handlers::documents::storage_helpers::cleanup_document_graph_data(
        &doc_id,
        &state.storage.graph_storage,
        Some(&vector_storage),
    )
    .await
    {
        warn!(
            injection_id = %injection_id,
            error = %e,
            "Graph cleanup during injection delete had errors (continuing)"
        );
    }

    // 2. Delete chunk vector embeddings (separate from entity embeddings above).
    //    WHY: chunk vectors are stored under chunk IDs, not entity names, so
    //    cleanup_document_graph_data does not remove them.
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

    // 3. Delete all KV entries (metadata + chunks + content).
    let keys = state.storage.kv_storage.keys().await?;
    let kv_ids_to_delete: Vec<String> = keys
        .into_iter()
        .filter(|k| k.starts_with(&doc_id) || *k == meta_key)
        .collect();
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

// ============================================================================
// PATCH /api/v1/workspaces/:workspace_id/injections/:injection_id — Update
// ============================================================================

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
        // WHY: Resolve workspace-specific storage BEFORE the spawn (SPEC-033, OODA-231.1).
        // WHY: Use workspace-specific pipeline to match embedding dimensions (prevents
        // silent entity_count=0 when global embedder dimension != workspace vector table dim).
        let workspace_pipeline = state.create_workspace_pipeline(&workspace_id).await;
        let inj_ctx = resolve_injection_context(&state, &workspace_id).await;
        spawn_injection_processing(InjectionTaskContext {
            pipeline: workspace_pipeline,
            graph_storage: state.storage.graph_storage.clone(),
            vector_storage: inj_ctx.vector_storage,
            kv_storage: state.storage.kv_storage.clone(),
            doc_id,
            content: new_content,
            workspace_id: workspace_id.clone(),
            data_tenant_id: inj_ctx.data_tenant_id,
            meta_key,
            injection_id: injection_id.clone(),
            name: new_name,
            source_type,
            source_filename,
            version: new_version,
            created_at,
        });
    }

    Ok(Json(PutInjectionResponse {
        injection_id,
        workspace_id,
        version: new_version,
        status: status.to_string(),
    }))
}

// ============================================================================
// PUT /api/v1/workspaces/:workspace_id/injection/file — Upload file
// ============================================================================

/// Create a knowledge injection from an uploaded file (plain-text formats).
///
/// Accepts multipart form with a single "file" field.
/// Supported: .txt, .md, .csv (plain-text, max 10 MB).
/// Content is UTF-8 decoded, then processed through the standard pipeline.
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

    // Max 10 MB per SPEC-0002 UX spec
    const MAX_FILE_BYTES: usize = 10 * 1024 * 1024;

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
            _ => {} // Ignore unknown fields
        }
    }

    if file_bytes.is_empty() {
        return Err(ApiError::BadRequest("No file provided".to_string()));
    }

    // Validate file size
    if file_bytes.len() > MAX_FILE_BYTES {
        return Err(ApiError::BadRequest(format!(
            "File exceeds 10 MB limit ({} bytes)",
            file_bytes.len()
        )));
    }

    // Validate extension: only plain-text formats (no PDF — needs vision)
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    const ALLOWED: [&str; 4] = ["txt", "md", "csv", "json"];
    if !ALLOWED.contains(&ext.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Unsupported file type '.{ext}'. Allowed: txt, md, csv, json"
        )));
    }

    // Decode UTF-8
    let content = String::from_utf8(file_bytes)
        .map_err(|_| ApiError::BadRequest("File must be valid UTF-8 text".to_string()))?;
    validate_content(&content)?;

    // Fall back to filename stem if no name provided
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

    // WHY: Resolve workspace-specific storage BEFORE the spawn (SPEC-033, OODA-231.1).
    // WHY: Use workspace-specific pipeline to match embedding dimensions (prevents
    // silent entity_count=0 when global embedder dimension != workspace vector table dim).
    let workspace_pipeline = state.create_workspace_pipeline(&workspace_id).await;
    let inj_ctx = resolve_injection_context(&state, &workspace_id).await;
    spawn_injection_processing(InjectionTaskContext {
        pipeline: workspace_pipeline,
        graph_storage: state.storage.graph_storage.clone(),
        vector_storage: inj_ctx.vector_storage,
        kv_storage: state.storage.kv_storage.clone(),
        doc_id,
        content,
        workspace_id: workspace_id.clone(),
        data_tenant_id: inj_ctx.data_tenant_id,
        meta_key,
        injection_id: injection_id.clone(),
        name,
        source_type: "file".to_string(),
        source_filename: Some(filename),
        version: 1,
        created_at: now,
    });

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

// ============================================================================
// Workspace Context Resolution (internal)
// ============================================================================

/// Resolved workspace context for injection pipeline processing.
///
/// Bundles the workspace-specific vector storage AND the workspace's authoritative
/// `tenant_id` (from the database) so both can be passed to the pipeline in a
/// single workspace lookup.
struct InjectionWorkspaceContext {
    /// Workspace-specific vector storage (or global fallback).
    vector_storage: std::sync::Arc<dyn edgequake_storage::traits::VectorStorage>,
    /// Workspace's authoritative tenant_id from the database.
    ///
    /// WHY: The header `X-Tenant-ID` is an auth hint from the frontend and may
    /// differ from the UUID stored on the workspace row.  Queries use the DB
    /// tenant_id (OODA-231.1), so injected vectors must carry the same value.
    data_tenant_id: Option<String>,
}

/// Resolve workspace context (vector storage + data tenant_id) for injection.
///
/// WHY: Injection must write vectors to the workspace-specific table (via
/// `vector_registry`) so that the query engine (which reads from that table)
/// can find them.  Additionally, vectors must carry the workspace's authoritative
/// `tenant_id` so the query-time metadata filter does not silently exclude them.
///
/// @implements SPEC-0002 (Knowledge Injection for Enhanced Search)
/// @implements SPEC-033 (Workspace vector isolation)
/// @implements OODA-231.1 (Correct tenant_id for data storage)
///
/// Falls back gracefully: if workspace lookup fails, uses global vector storage and
/// `None` tenant_id so the injection still proceeds with reduced isolation.
async fn resolve_injection_context(
    state: &AppState,
    workspace_id: &str,
) -> InjectionWorkspaceContext {
    use edgequake_storage::traits::WorkspaceVectorConfig;

    let fallback = InjectionWorkspaceContext {
        vector_storage: state.storage.vector_storage.clone(),
        data_tenant_id: None,
    };

    // Non-UUID workspace IDs (e.g. "default") have no per-workspace table.
    let workspace_uuid = match Uuid::parse_str(workspace_id) {
        Ok(uuid) => uuid,
        Err(_) => return fallback,
    };

    let workspace = match state.workspace_service.get_workspace(workspace_uuid).await {
        Ok(Some(ws)) => ws,
        Ok(None) => {
            warn!(
                workspace_id,
                "Workspace not found; using global vector storage for injection"
            );
            return fallback;
        }
        Err(e) => {
            warn!(
                workspace_id,
                error = %e,
                "Failed to look up workspace; using global vector storage for injection"
            );
            return fallback;
        }
    };

    let data_tenant_id = Some(workspace.tenant_id.to_string());

    let config = WorkspaceVectorConfig {
        workspace_id: workspace_uuid,
        dimension: workspace.embedding_dimension,
        namespace: "default".to_string(),
    };

    let vector_storage = match state.storage.vector_registry.get_or_create(config).await {
        Ok(storage) => {
            debug!(
                workspace_id,
                dimension = workspace.embedding_dimension,
                "Resolved workspace-specific vector storage for injection"
            );
            storage
        }
        Err(e) => {
            warn!(
                workspace_id,
                error = %e,
                "Failed to get workspace vector storage; using global fallback"
            );
            state.storage.vector_storage.clone()
        }
    };

    InjectionWorkspaceContext {
        vector_storage,
        data_tenant_id,
    }
}

// ============================================================================
// Pipeline Processing (internal)
// ============================================================================

/// Process injection content through the standard pipeline with injection tagging.
///
/// Uses `Pipeline::process()` to chunk + extract entities, then merges into graph
/// with source_type=injection metadata so citations can be filtered.
async fn process_injection_pipeline(
    pipeline: &std::sync::Arc<edgequake_pipeline::Pipeline>,
    graph_storage: std::sync::Arc<dyn edgequake_storage::traits::GraphStorage>,
    vector_storage: std::sync::Arc<dyn edgequake_storage::traits::VectorStorage>,
    doc_id: &str,
    content: &str,
    workspace_id: &str,
    tenant_id: Option<String>,
) -> std::result::Result<(u32, Vec<String>), Box<dyn std::error::Error + Send + Sync>> {
    use edgequake_pipeline::{KnowledgeGraphMerger, MergerConfig};

    // Process through standard pipeline
    let result = pipeline.process(doc_id, content).await?;

    let merger_config = MergerConfig::default();
    let merger = KnowledgeGraphMerger::new(merger_config, graph_storage, vector_storage.clone())
        .with_tenant_context(tenant_id.clone(), Some(workspace_id.to_string()));

    // Tag and merge entities with injection source tracking
    let mut tagged_extractions = Vec::new();
    for extraction in &result.extractions {
        let mut tagged = extraction.clone();
        for entity in &mut tagged.entities {
            entity.source_document_id = Some(doc_id.to_string());
            entity.source_file_path = Some("injection".to_string());
            if entity.source_chunk_ids.is_empty() {
                entity.source_chunk_ids = vec![format!("{}-chunk-0", doc_id)];
            }
        }
        for rel in &mut tagged.relationships {
            rel.source_document_id = Some(doc_id.to_string());
            rel.source_file_path = Some("injection".to_string());
            if rel.source_chunk_id.is_none() {
                rel.source_chunk_id = Some(format!("{}-chunk-0", doc_id));
            }
        }
        tagged_extractions.push(tagged);
    }

    let merge_stats = merger.merge(tagged_extractions).await?;
    let entity_count = (merge_stats.entities_created + merge_stats.entities_updated) as u32;

    // Store chunk embeddings in vector storage with injection metadata
    let mut stored_chunk_ids = Vec::new();
    for chunk in &result.chunks {
        if let Some(ref embedding) = chunk.embedding {
            let chunk_id = chunk.id.clone();
            let metadata = serde_json::json!({
                "type": "chunk",
                "source": "injection",
                "source_type": "injection",
                "source_document_id": doc_id,
                "source_file_path": "injection",
                "content": chunk.content.chars().take(500).collect::<String>(),
                "workspace_id": workspace_id,
                // WHY: tenant_id is required by MetadataFilter SQL (AND semantics); omitting it
                // causes zero results when querying workspace-specific vector storage (OODA-231.1).
                "tenant_id": tenant_id.as_deref().unwrap_or(""),
            });
            if let Err(e) = vector_storage
                .upsert(&[(chunk_id.clone(), embedding.clone(), metadata)])
                .await
            {
                warn!(error = %e, "Failed to store injection chunk embedding");
            } else {
                stored_chunk_ids.push(chunk_id);
            }
        }
    }

    info!(
        entity_count,
        chunk_count = stored_chunk_ids.len(),
        "Injection pipeline processing complete"
    );
    Ok((entity_count, stored_chunk_ids))
}
