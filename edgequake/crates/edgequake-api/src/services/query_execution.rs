//! Shared SOTA query execution (SPEC-017 P1-03).
//!
//! Single routing matrix for workspace embedding/vector/LLM overrides.
//! Used by `/query`, `/chat/completions`, and streaming handlers.

use std::sync::{Arc, OnceLock};

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_query::{QueryContext, QueryMode, QueryRequest as EngineQueryRequest, QueryResponse};
use edgequake_storage::traits::VectorStorage;
use futures::stream::BoxStream;
use tracing::{debug, warn};

use crate::error::{ApiError, ApiResult};
use crate::handlers::query::{get_workspace_embedding_provider, get_workspace_vector_storage};
use crate::state::AppState;

type TokenStream = BoxStream<'static, Result<String, edgequake_query::error::QueryError>>;
type StreamQueryResult =
    Result<(QueryContext, QueryMode, TokenStream), edgequake_query::error::QueryError>;

/// Optional workspace-scoped providers for query execution.
#[derive(Clone, Default)]
pub struct WorkspaceQueryResources {
    pub embedding: Option<Arc<dyn EmbeddingProvider>>,
    pub vector_storage: Option<Arc<dyn VectorStorage>>,
    /// LightRAG KEYWORD role LLM (SPEC-046 EQ-046-13); falls back to answer LLM.
    pub keyword_llm: Option<Arc<dyn LLMProvider>>,
}

/// Resolve workspace-scoped embedding/vector providers (SPEC-017 P2-02).
///
/// Returns defaults when `workspace_id` is `None`. Fails closed when a workspace
/// is explicitly requested but provider resolution fails.
/// Env KEYWORD role still applies without a workspace (LightRAG process-env law).
pub async fn resolve_workspace_query_resources(
    state: &AppState,
    workspace_id: Option<&str>,
) -> ApiResult<WorkspaceQueryResources> {
    let keyword_llm = resolve_keyword_llm(state, workspace_id).await;
    let Some(workspace_id) = workspace_id else {
        return Ok(WorkspaceQueryResources {
            keyword_llm,
            ..Default::default()
        });
    };

    let embedding = get_workspace_embedding_provider(state, workspace_id).await?;
    let vector_storage = get_workspace_vector_storage(state, workspace_id).await?;
    Ok(WorkspaceQueryResources {
        embedding,
        vector_storage,
        keyword_llm,
    })
}

/// Process-env KEYWORD LLM, cached for the process lifetime (env set at boot).
fn env_keyword_llm_cached() -> Option<Arc<dyn LLMProvider>> {
    static CACHE: OnceLock<Option<Arc<dyn LLMProvider>>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let role = edgequake_core::env_keyword_role_llm()?;
            match crate::safety_limits::create_safe_llm_provider(&role.provider, &role.model) {
                Ok(llm) => {
                    tracing::info!(
                        provider = %role.provider,
                        model = %role.model,
                        "KEYWORD role LLM cached from EDGEQUAKE_KEYWORD_LLM_* env"
                    );
                    Some(llm)
                }
                Err(e) => {
                    warn!(
                        provider = %role.provider,
                        model = %role.model,
                        error = %e,
                        "KEYWORD env LLM failed; falling back to workspace/Query"
                    );
                    None
                }
            }
        })
        .clone()
}

/// Resolve KEYWORD-role LLM (non-fatal — None falls back to Query).
///
/// Precedence (LightRAG KEYWORD role law / 061 Idea A):
/// 1. Process env `EDGEQUAKE_KEYWORD_LLM_*` (product / Acc latency packs)
/// 2. Workspace metadata `llm_roles.keyword`
/// 3. `None` → Query LLM (no silent double-create of the same model)
async fn resolve_keyword_llm(
    state: &AppState,
    workspace_id: Option<&str>,
) -> Option<Arc<dyn LLMProvider>> {
    if let Some(llm) = env_keyword_llm_cached() {
        return Some(llm);
    }

    let workspace_id = workspace_id?;
    let ws_uuid = crate::middleware::resolve_workspace_uuid(Some(workspace_id))?;
    let ws = state
        .workspace_service
        .get_workspace(ws_uuid)
        .await
        .ok()
        .flatten()?;
    // Only override when workspace metadata explicitly configures keyword role
    // (avoid doubling Query LLM when roles are unset).
    let _cfg = edgequake_core::role_config_from_workspace(&ws, edgequake_core::LlmRole::Keyword)?;
    let role = edgequake_core::resolve_role_llm(&ws, edgequake_core::LlmRole::Keyword);
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.pg_pool.as_ref() {
        if let Some(connected) = crate::providers::connection_store::llm_from_workspace_role(
            pool,
            &ws,
            edgequake_core::LlmRole::Keyword,
        )
        .await
        {
            return Some(connected);
        }
    }
    crate::safety_limits::create_safe_llm_provider(&role.provider, &role.model).ok()
}

type OptionalLlm = Option<Arc<dyn LLMProvider>>;

/// Resolve Keyword / Query role LLMs for a SOTA dispatch (DRY sync+stream).
fn resolve_role_llms_for_dispatch(
    resources: &WorkspaceQueryResources,
    llm_override: OptionalLlm,
) -> (OptionalLlm, OptionalLlm) {
    let keyword_llm = resources
        .keyword_llm
        .clone()
        .or_else(|| llm_override.clone());
    (keyword_llm, llm_override)
}

/// Resolve embedding + vector storage for the workspace matrix (SPEC-017 / SPEC-046 DRY).
fn resolve_sota_providers(
    state: &AppState,
    resources: WorkspaceQueryResources,
) -> (
    Arc<dyn EmbeddingProvider>,
    Arc<dyn VectorStorage>,
    bool, // used_defaults_only
) {
    let used_defaults = resources.embedding.is_none() && resources.vector_storage.is_none();
    let embedding = resources
        .embedding
        .unwrap_or_else(|| state.query.engine_impl.default_embedding_provider());
    let vector_storage = resources
        .vector_storage
        .unwrap_or_else(|| state.query.engine_impl.default_vector_storage());
    (embedding, vector_storage, used_defaults)
}

/// Execute a SOTA query using the standard workspace routing matrix.
pub async fn execute_sota_query(
    state: &AppState,
    request: EngineQueryRequest,
    resources: WorkspaceQueryResources,
    llm_override: Option<Arc<dyn LLMProvider>>,
) -> ApiResult<QueryResponse> {
    let has_llm_override = llm_override.is_some();
    let (keyword_llm, answer_llm) = resolve_role_llms_for_dispatch(&resources, llm_override);
    let (embedding, vector_storage, used_defaults) = resolve_sota_providers(state, resources);

    debug!(
        has_llm_override,
        used_defaults, "SOTA query: workspace routing matrix"
    );

    let result = if used_defaults && answer_llm.is_none() && keyword_llm.is_none() {
        state.query.engine_impl.query(request).await
    } else {
        state
            .query
            .engine_impl
            .query_with_role_llms(request, embedding, vector_storage, keyword_llm, answer_llm)
            .await
    };

    result.map_err(ApiError::from)
}

/// Returns true when the LLM rejected credentials (invalid or missing API key).
pub fn is_llm_auth_failure(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("invalid_api_key")
        || lower.contains("incorrect api key")
        || lower.contains("authentication error")
        || lower.contains("invalid api key")
        || (lower.contains("unauthorized") && lower.contains("api"))
}

/// Execute SOTA query; on auth failure with a request override, retry with server default.
pub async fn execute_sota_query_with_auth_fallback(
    state: &AppState,
    request: EngineQueryRequest,
    resources: WorkspaceQueryResources,
    llm_override: Option<Arc<dyn LLMProvider>>,
) -> ApiResult<QueryResponse> {
    let had_override = llm_override.is_some();
    match execute_sota_query(state, request.clone(), resources.clone(), llm_override).await {
        Ok(response) => Ok(response),
        Err(e) if had_override && is_llm_auth_failure(&e.to_string()) => {
            warn!(
                error = %e,
                "LLM auth failed for request override; retrying with server default"
            );
            execute_sota_query(state, request, resources, None).await
        }
        Err(e) => Err(e),
    }
}

/// Execute a streaming SOTA query using the same workspace routing matrix.
pub async fn execute_sota_query_stream(
    state: &AppState,
    request: EngineQueryRequest,
    resources: WorkspaceQueryResources,
    llm_override: Option<Arc<dyn LLMProvider>>,
) -> StreamQueryResult {
    let has_llm_override = llm_override.is_some();
    let (keyword_llm, answer_llm) = resolve_role_llms_for_dispatch(&resources, llm_override);
    let (embedding, vector_storage, used_defaults) = resolve_sota_providers(state, resources);

    debug!(
        has_llm_override,
        used_defaults, "SOTA stream: workspace routing matrix"
    );

    if used_defaults && answer_llm.is_none() && keyword_llm.is_none() {
        state
            .query
            .engine_impl
            .query_stream_with_context(request)
            .await
    } else {
        state
            .query
            .engine_impl
            .query_stream_with_role_llms(
                request,
                embedding,
                vector_storage,
                keyword_llm,
                answer_llm,
            )
            .await
    }
}

/// Streaming SOTA query with auth fallback to server default (SPEC-017 query regression).
pub async fn execute_sota_query_stream_with_auth_fallback(
    state: &AppState,
    request: EngineQueryRequest,
    resources: WorkspaceQueryResources,
    llm_override: Option<Arc<dyn LLMProvider>>,
) -> StreamQueryResult {
    let had_override = llm_override.is_some();
    match execute_sota_query_stream(state, request.clone(), resources.clone(), llm_override).await {
        Ok(stream) => Ok(stream),
        Err(e) if had_override && is_llm_auth_failure(&e.to_string()) => {
            warn!(
                error = %e,
                "LLM auth failed for streaming override; retrying with server default"
            );
            execute_sota_query_stream(state, request, resources, None).await
        }
        Err(e) => Err(e),
    }
}

/// Validate that LLM override fields are both present or both absent (SPEC-017 P1-07).
pub fn validate_llm_override_pair(provider: Option<&str>, model: Option<&str>) -> ApiResult<()> {
    match (
        provider.filter(|s| !s.is_empty()),
        model.filter(|s| !s.is_empty()),
    ) {
        (Some(_), Some(_)) | (None, None) => Ok(()),
        _ => Err(ApiError::BadRequest(
            "Both llm_provider and llm_model must be set for LLM override".to_string(),
        )),
    }
}

/// Build LLM override from explicit provider/model request fields.
pub fn llm_override_from_request(
    provider: Option<&str>,
    model: Option<&str>,
    extra_headers: Option<std::collections::HashMap<String, String>>,
) -> ApiResult<Option<Arc<dyn LLMProvider>>> {
    validate_llm_override_pair(provider, model)?;
    match (provider, model) {
        (Some(provider), Some(model)) => Ok(Some(
            crate::safety_limits::create_safe_llm_provider_with_headers(
                provider,
                model,
                extra_headers,
            )
            .map_err(|e| ApiError::Internal(format!("Failed to create LLM provider: {}", e)))?,
        )),
        (None, None) => Ok(None),
        _ => unreachable!("validate_llm_override_pair ensures both or neither"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_llm_override_pair_rejects_partial() {
        assert!(matches!(
            validate_llm_override_pair(Some("openai"), None),
            Err(ApiError::BadRequest(_))
        ));
    }

    #[test]
    fn llm_override_requires_both_fields() {
        assert!(matches!(
            llm_override_from_request(Some("openai"), None, None),
            Err(ApiError::BadRequest(_))
        ));
    }

    #[test]
    fn llm_override_none_when_unset() {
        assert!(llm_override_from_request(None, None, None)
            .unwrap()
            .is_none());
    }

    #[test]
    fn detects_openai_invalid_api_key_message() {
        let msg = "LLM error: Authentication error: invalid_request_error: Incorrect API key provided (code: invalid_api_key)";
        assert!(is_llm_auth_failure(msg));
    }

    #[test]
    fn ignores_unrelated_query_errors() {
        assert!(!is_llm_auth_failure("Storage error: connection refused"));
    }

    #[tokio::test]
    async fn resolve_resources_default_without_workspace() {
        let state = crate::state::AppState::test_state();
        let resources = resolve_workspace_query_resources(&state, None)
            .await
            .expect("default resolution");
        assert!(resources.embedding.is_none());
        assert!(resources.vector_storage.is_none());
    }
}
