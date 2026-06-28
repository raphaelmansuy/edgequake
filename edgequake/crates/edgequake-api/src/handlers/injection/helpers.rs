//! Shared injection handler primitives (validation, enqueue, workspace context).

use tracing::{debug, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::build_injection_metadata;
use crate::state::AppState;

use crate::handlers::injection_types::*;

pub(crate) fn workspace_id_from_tenant(ctx: &TenantContext) -> String {
    ctx.workspace_id_or_default()
}

pub(crate) fn validate_name(name: &str) -> ApiResult<()> {
    if name.is_empty() || name.len() > 100 {
        return Err(ApiError::BadRequest(
            "Name must be between 1 and 100 characters".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_content(content: &str) -> ApiResult<()> {
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

#[inline]
pub(crate) fn str_field(val: &serde_json::Value, key: &str) -> String {
    val.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

#[inline]
pub(crate) fn str_field_or(val: &serde_json::Value, key: &str, default: &str) -> String {
    val.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

#[inline]
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_meta(
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
    build_injection_metadata(
        injection_id,
        name,
        content,
        workspace_id,
        source_type,
        source_filename,
        status,
        version,
        entity_count,
        chunk_ids,
        doc_id,
        created_at,
        updated_at,
        error,
    )
}

pub(crate) struct InjectionEnqueueParams {
    pub doc_id: String,
    pub content: String,
    pub workspace_id: String,
    pub meta_key: String,
    pub injection_id: String,
    pub name: String,
    pub source_type: String,
    pub source_filename: Option<String>,
    pub version: u32,
    pub created_at: String,
    pub data_tenant_id: Option<String>,
}

pub(crate) async fn enqueue_injection_processing(
    state: &AppState,
    tenant_ctx: &TenantContext,
    params: InjectionEnqueueParams,
) -> ApiResult<()> {
    use edgequake_tasks::{KnowledgeInjectionData, Task, TaskType};

    let tenant_id = uuid::Uuid::parse_str(&tenant_ctx.tenant_id_or_default())
        .map_err(|_| ApiError::ValidationError("Invalid tenant ID".to_string()))?;
    let workspace_uuid = crate::middleware::resolve_workspace_uuid(Some(&params.workspace_id))
        .ok_or_else(|| {
            ApiError::ValidationError(format!("Invalid workspace ID: {}", params.workspace_id))
        })?;

    let task_data = KnowledgeInjectionData {
        doc_id: params.doc_id,
        content: params.content,
        workspace_id: params.workspace_id,
        meta_key: params.meta_key,
        injection_id: params.injection_id,
        name: params.name,
        source_type: params.source_type,
        source_filename: params.source_filename,
        version: params.version,
        created_at: params.created_at,
        data_tenant_id: params.data_tenant_id,
    };

    let task = Task::new(
        tenant_id,
        workspace_uuid,
        TaskType::KnowledgeInjection,
        serde_json::to_value(task_data).map_err(|e| {
            ApiError::BadRequest(format!("Failed to serialize injection task: {e}"))
        })?,
    );

    state.enqueue_task(task).await?;
    Ok(())
}

pub(crate) fn detail_from_meta(val: &serde_json::Value) -> InjectionDetailResponse {
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

pub(crate) struct InjectionWorkspaceContext {
    pub vector_storage: std::sync::Arc<dyn edgequake_storage::traits::VectorStorage>,
    pub data_tenant_id: Option<String>,
}

pub(crate) async fn resolve_injection_context(
    state: &AppState,
    workspace_id: &str,
) -> InjectionWorkspaceContext {
    use edgequake_storage::traits::WorkspaceVectorConfig;

    let fallback = InjectionWorkspaceContext {
        vector_storage: state.storage.vector_storage.clone(),
        data_tenant_id: None,
    };

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
