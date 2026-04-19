//! Workspace-aware provider resolution for queries.
//!
//! @implements SPEC-032 (Workspace-specific embedding in query process)
//! @implements SPEC-033 (Workspace vector isolation)
//! @implements OODA-231.1 (Correct tenant_id for data queries)

use tracing::debug;

use crate::error::ApiError;
use crate::providers::WorkspaceProviderResolver;
use crate::state::AppState;

/// Get workspace by ID for tenant isolation.
///
/// @implements OODA-231.1: Correct tenant_id for data queries
pub(super) async fn get_workspace(
    state: &AppState,
    workspace_id: &str,
) -> Result<Option<edgequake_core::Workspace>, ApiError> {
    use uuid::Uuid;

    let workspace_uuid = Uuid::parse_str(workspace_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid workspace ID: {}", e)))?;

    state
        .workspace_service
        .get_workspace(workspace_uuid)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get workspace: {}", e)))
}

/// Resolve the query workspace when the caller supplied an explicit workspace ID.
///
/// WHY: An explicit workspace selector is a hard isolation boundary. If the
/// caller asks for workspace A and that workspace is invalid or missing, falling
/// back to the server default would silently turn a scoped query into an
/// unscoped one. Query handlers should therefore fail closed for explicit
/// workspace requests and only use default behavior when no workspace was sent.
pub(super) async fn resolve_query_workspace(
    state: &AppState,
    workspace_id: Option<&str>,
) -> Result<Option<edgequake_core::Workspace>, ApiError> {
    match workspace_id {
        Some(workspace_id) => {
            let workspace = get_workspace(state, workspace_id).await?.ok_or_else(|| {
                ApiError::NotFound(format!("Workspace not found: {}", workspace_id))
            })?;
            Ok(Some(workspace))
        }
        None => Ok(None),
    }
}

/// Get workspace-specific embedding provider for query execution.
///
/// @implements SPEC-032: Workspace-specific embedding in query process
/// @implements OODA-259: Delegates to WorkspaceProviderResolver to eliminate duplication
///
/// This function delegates to [`WorkspaceProviderResolver::resolve_embedding_provider_optional`]
/// which provides the canonical implementation for workspace-aware embedding provider creation.
///
/// # Arguments
///
/// * `state` - Application state containing workspace service
/// * `workspace_id` - ID of the workspace to get embedding config for
///
/// # Returns
///
/// - `Ok(Some(provider))` - Workspace-specific embedding provider
/// - `Ok(None)` - Workspace uses default embedding, no override needed
/// - `Err(_)` - Error looking up workspace or creating provider
pub async fn get_workspace_embedding_provider(
    state: &AppState,
    workspace_id: &str,
) -> Result<Option<std::sync::Arc<dyn edgequake_query::EmbeddingProvider>>, ApiError> {
    // OODA-259: Delegate to resolver to eliminate code duplication
    // The resolver now provides `resolve_embedding_provider_optional` which returns
    // Ok(None) for fallback semantics (workspace has no embedding config)
    let resolver = WorkspaceProviderResolver::new(state.workspace_service.clone());
    let result = resolver
        .resolve_embedding_provider_optional(workspace_id)
        .await
        .map_err(ApiError::from)?;

    // Extract just the Arc<dyn EmbeddingProvider> from ResolvedEmbeddingProvider
    Ok(result.map(|resolved| resolved.provider))
}

/// Get workspace-specific vector storage for query execution.
///
/// SPEC-033: Workspace vector isolation.
///
/// This function looks up the workspace's embedding dimension and gets or creates
/// a workspace-specific vector storage instance. If the workspace uses the default
/// dimension, returns None to indicate the default should be used.
///
/// @implements OODA-228: Fix dimension mismatch in chat handler
///
/// # Arguments
///
/// * `state` - Application state containing workspace service and vector registry
/// * `workspace_id` - ID of the workspace to get vector storage for
///
/// # Returns
///
/// - `Ok(Some(storage))` - Workspace-specific vector storage
/// - `Ok(None)` - Workspace uses default storage, no override needed
/// - `Err(_)` - Error looking up workspace or creating storage
pub async fn get_workspace_vector_storage(
    state: &AppState,
    workspace_id: &str,
) -> Result<Option<std::sync::Arc<dyn edgequake_storage::traits::VectorStorage>>, ApiError> {
    use edgequake_storage::traits::WorkspaceVectorConfig;
    use uuid::Uuid;

    // Parse workspace ID
    let workspace_uuid = Uuid::parse_str(workspace_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid workspace ID: {}", e)))?;

    // Get workspace from service
    let workspace = state
        .workspace_service
        .get_workspace(workspace_uuid)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get workspace: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace not found: {}", workspace_id)))?;

    // Create workspace-specific vector storage config
    let config = WorkspaceVectorConfig {
        workspace_id: workspace_uuid,
        dimension: workspace.embedding_dimension,
        namespace: "default".to_string(),
    };

    debug!(
        workspace_id = %workspace_id,
        dimension = workspace.embedding_dimension,
        "Getting workspace-specific vector storage"
    );

    // Get or create workspace vector storage
    // OODA-225: Auto-evict and retry on dimension mismatch
    // WHY: When embedding provider changes (e.g., Ollama 768 → OpenAI 1536), the cached
    // vector storage instance may hold the old dimension. If get_or_create fails due to
    // dimension mismatch, we evict the cache and retry with the new dimension.
    let storage = match state.vector_registry.get_or_create(config.clone()).await {
        Ok(s) => s,
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("Dimension mismatch") || error_msg.contains("cached=") {
                // Dimension mismatch detected - evict cache and retry
                tracing::warn!(
                    workspace_id = %workspace_id,
                    error = %error_msg,
                    "Dimension mismatch detected, evicting cache and retrying"
                );
                state.vector_registry.evict(&workspace_uuid).await;

                // Retry after eviction
                state
                    .vector_registry
                    .get_or_create(config)
                    .await
                    .map_err(|e2| {
                        ApiError::Internal(format!(
                            "Failed to create vector storage for workspace {} after cache eviction: {}",
                            workspace_id, e2
                        ))
                    })?
            } else {
                return Err(ApiError::Internal(format!(
                    "Failed to create vector storage for workspace {}: {}",
                    workspace_id, e
                )));
            }
        }
    };

    Ok(Some(storage))
}

/// Get workspace LLM provider and model info for lineage tracking.
///
/// @implements SPEC-032 Item 22: Display model used after tokens/second
///
/// # Returns
///
/// Tuple of (Option<provider>, Option<model>) from workspace config or defaults.
pub(super) async fn get_workspace_llm_info(
    state: &AppState,
    workspace_id: Option<&str>,
) -> (Option<String>, Option<String>) {
    use edgequake_core::types::{DEFAULT_LLM_MODEL, DEFAULT_LLM_PROVIDER};
    use uuid::Uuid;

    // If no workspace, return defaults
    let workspace_id = match workspace_id {
        Some(id) => id,
        None => {
            return (
                Some(DEFAULT_LLM_PROVIDER.to_string()),
                Some(DEFAULT_LLM_MODEL.to_string()),
            );
        }
    };

    // Try to get workspace config
    let workspace_uuid = match Uuid::parse_str(workspace_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                Some(DEFAULT_LLM_PROVIDER.to_string()),
                Some(DEFAULT_LLM_MODEL.to_string()),
            );
        }
    };

    match state.workspace_service.get_workspace(workspace_uuid).await {
        Ok(Some(workspace)) => {
            let provider = if workspace.llm_provider.is_empty() {
                Some(DEFAULT_LLM_PROVIDER.to_string())
            } else {
                Some(workspace.llm_provider)
            };
            let model = if workspace.llm_model.is_empty() {
                Some(DEFAULT_LLM_MODEL.to_string())
            } else {
                Some(workspace.llm_model)
            };
            (provider, model)
        }
        _ => (
            Some(DEFAULT_LLM_PROVIDER.to_string()),
            Some(DEFAULT_LLM_MODEL.to_string()),
        ),
    }
}
