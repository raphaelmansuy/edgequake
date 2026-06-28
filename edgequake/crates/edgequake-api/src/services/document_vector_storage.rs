//! Workspace vector storage resolution for document ingest/delete (SPEC-033).

use std::sync::Arc;

use edgequake_storage::traits::VectorStorage;
use tracing::warn;

use crate::error::ApiError;
use crate::state::AppState;

/// Get workspace-specific vector storage for document ingestion (STRICT mode).
pub async fn get_workspace_vector_storage_strict(
    state: &AppState,
    workspace_id: &str,
) -> Result<Arc<dyn VectorStorage>, ApiError> {
    use edgequake_core::{
        resolve_workspace_vector_storage, WorkspaceVectorResolveInput, WorkspaceVectorResolvePolicy,
    };

    let allow_fallback = state.storage.mode.is_memory();
    let policy = if allow_fallback {
        WorkspaceVectorResolvePolicy::AllowDefaultFallback
    } else {
        WorkspaceVectorResolvePolicy::Strict
    };

    let default_storage = state.storage.vector_registry.default_storage();
    let input = WorkspaceVectorResolveInput::new(Some(workspace_id), "default");

    resolve_workspace_vector_storage(
        state.storage.vector_registry.as_ref(),
        default_storage,
        Some(state.workspace_service.as_ref()),
        state.query.embedding_provider.dimension(),
        input,
        policy,
    )
    .await
    .map_err(|e| match e {
        edgequake_core::Error::Validation(msg) => ApiError::BadRequest(msg),
        edgequake_core::Error::NotFound(msg) => ApiError::NotFound(msg),
        edgequake_core::Error::Config(msg) | edgequake_core::Error::Internal(msg) => {
            ApiError::Internal(msg)
        }
        other => ApiError::Internal(other.to_string()),
    })
}

/// Get workspace-specific vector storage with fallback (LEGACY — read paths only).
#[allow(dead_code)]
pub async fn get_workspace_vector_storage_with_fallback(
    state: &AppState,
    workspace_id: &str,
) -> Arc<dyn VectorStorage> {
    match get_workspace_vector_storage_strict(state, workspace_id).await {
        Ok(storage) => storage,
        Err(e) => {
            warn!(
                workspace_id = %workspace_id,
                error = %e,
                "Falling back to default vector storage (READ ONLY operations)"
            );
            state.storage.vector_registry.default_storage()
        }
    }
}

/// Lenient vector storage lookup for deletion (never block delete on missing workspace).
pub async fn get_workspace_vector_storage_for_delete(
    state: &AppState,
    workspace_id: &str,
) -> Arc<dyn VectorStorage> {
    match get_workspace_vector_storage_strict(state, workspace_id).await {
        Ok(storage) => storage,
        Err(e) => {
            warn!(
                workspace_id = %workspace_id,
                error = %e,
                "Workspace not found or vector storage unavailable during document deletion. \
                 Proceeding with default storage. Orphaned vector rows (if any) can be \
                 cleaned up later via the vector storage maintenance API."
            );
            state.storage.vector_registry.default_storage()
        }
    }
}
