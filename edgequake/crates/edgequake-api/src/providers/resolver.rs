//! Workspace Provider Resolver
//!
//! This module provides a unified interface for resolving LLM and embedding
//! providers based on workspace configuration, with proper fallback logic
//! and error handling.
//!
//! # WHY: This Is the QUERY-TIME Provider Resolver
//!
//! This resolver is used ONLY for chat query requests — NOT for pipeline
//! document extraction. The pipeline uses a completely different path
//! (see processor.rs `get_workspace_pipeline_strict`).
//!
//! ```text
//!  ┌──────────────────────────────────────────────────────────────────┐
//!  │  QUERY-TIME RESOLUTION (this module)                            │
//!  │                                                                  │
//!  │  request.provider + request.model                                │
//!  │       │                                                          │
//!  │       ├── Both present? ──► Create provider → source=Request     │
//!  │       │                                                          │
//!  │       ├── Absent? Check workspace.llm_provider                   │
//!  │       │   └── Present? ──► Create provider → source=Workspace    │
//!  │       │                                                          │
//!  │       └── Neither? ──► Return None → caller uses engine_impl      │
//!  │                         default (from_env() at startup)           │
//!  └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Design Principles
//!
//! 1. **Single Source of Truth**: All provider resolution logic goes through this module
//! 2. **Always Safe**: Uses safety-limited providers with timeouts
//! 3. **Result-Based**: Returns errors for caller to handle appropriately
//! 4. **API Key Detection**: Automatically detects and flags API key issues
//!
//! ## First-Principles Resolution Ladder
//!
//! **Goal:** answer the query with an LLM that can authenticate in *this* runtime.
//!
//! 1. **Request override** — only if credentials for that provider are configured;
//!    auth/creation failures fall through (not a hard error).
//! 2. **Workspace override** — same credential gate and fall-through.
//! 3. **Server default** — `None` → `engine_impl` startup provider (`from_env()`).
//! 4. **Runtime auth rejection** — `execute_sota_query_*_with_auth_fallback` retries
//!    without override when the upstream API rejects the key.
//!
//! @implements OODA-226: Unified provider resolution to eliminate code duplication

use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::attribution::build_application_context;
use crate::safety_limits::{create_safe_embedding_provider, create_safe_llm_provider_with_context};
use edgequake_core::{Workspace, WorkspaceService};
use edgequake_llm::ModelsConfig;
use edgequake_query::{EmbeddingProvider, LLMProvider};

use crate::providers::credentials::llm_provider_credentials_configured;
use crate::providers::error::ProviderResolutionError;

/// Configuration for LLM provider resolution from a request.
#[derive(Debug, Clone, Default)]
pub struct LlmResolutionRequest {
    /// Provider name from request (e.g., "openai", "ollama")
    pub provider: Option<String>,
    /// Model name from request (e.g., "gpt-4o-mini", "gemma3:12b")
    pub model: Option<String>,
    /// Optional HTTP headers to propagate to the upstream LLM provider call.
    ///
    /// Enables B2B / multi-tenant metadata (`x-request-id`, `x-tenant-id`,
    /// `x-correlation-id`, `traceparent`, HMAC tokens) to flow through to the
    /// LLM API. Reserved headers are silently dropped by the provider.
    pub extra_headers: Option<std::collections::HashMap<String, String>>,
}

impl LlmResolutionRequest {
    /// Create from provider string that may include model (legacy format).
    ///
    /// Supports both formats:
    /// - "openai/gpt-4o-mini" (legacy)
    /// - "openai" with separate model field (new)
    pub fn from_provider_string(provider: Option<String>, model: Option<String>) -> Self {
        Self {
            provider,
            model,
            extra_headers: None,
        }
    }

    /// Check if this request has an explicit provider selection.
    pub fn has_explicit_provider(&self) -> bool {
        self.provider
            .as_ref()
            .map(|p| !p.is_empty())
            .unwrap_or(false)
    }
}

/// Result of LLM provider resolution.
pub struct ResolvedLlmProvider {
    /// The resolved LLM provider (safety-limited)
    pub provider: Arc<dyn LLMProvider>,
    /// The provider name used
    pub provider_name: String,
    /// The model name used
    pub model_name: String,
    /// How the provider was resolved
    pub source: ProviderSource,
}

// Manual Debug impl since Arc<dyn LLMProvider> doesn't implement Debug
impl std::fmt::Debug for ResolvedLlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedLlmProvider")
            .field("provider_name", &self.provider_name)
            .field("model_name", &self.model_name)
            .field("source", &self.source)
            .finish()
    }
}

/// Result of embedding provider resolution.
pub struct ResolvedEmbeddingProvider {
    /// The resolved embedding provider (safety-limited)
    pub provider: Arc<dyn EmbeddingProvider>,
    /// The provider name used
    pub provider_name: String,
    /// The model name used
    pub model_name: String,
    /// The embedding dimension
    pub dimension: usize,
}

// Manual Debug impl since Arc<dyn EmbeddingProvider> doesn't implement Debug
impl std::fmt::Debug for ResolvedEmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedEmbeddingProvider")
            .field("provider_name", &self.provider_name)
            .field("model_name", &self.model_name)
            .field("dimension", &self.dimension)
            .finish()
    }
}

/// Indicates how a provider was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderSource {
    /// Explicitly selected in the request
    Request,
    /// From workspace configuration
    Workspace,
    /// From tenant defaults (SPEC-123)
    Tenant,
    /// Server default fallback
    ServerDefault,
}

impl From<edgequake_core::ModelResolutionSource> for ProviderSource {
    fn from(source: edgequake_core::ModelResolutionSource) -> Self {
        match source {
            edgequake_core::ModelResolutionSource::Request => Self::Request,
            edgequake_core::ModelResolutionSource::Workspace => Self::Workspace,
            edgequake_core::ModelResolutionSource::Tenant => Self::Tenant,
            edgequake_core::ModelResolutionSource::Env
            | edgequake_core::ModelResolutionSource::Default => Self::ServerDefault,
        }
    }
}

/// Unified provider resolver for workspace-aware provider creation.
///
/// This resolver encapsulates all the logic for determining which LLM or
/// embedding provider to use based on:
/// - Explicit request parameters
/// - Workspace configuration
/// - Server defaults
///
/// ## Usage
///
/// ```rust,ignore
/// let resolver = WorkspaceProviderResolver::new(workspace_service);
///
/// // Resolve LLM provider
/// let request = LlmResolutionRequest::from_provider_string(
///     Some("openai".to_string()),
///     Some("gpt-4o-mini".to_string()),
/// );
/// let resolved = resolver.resolve_llm_provider(
///     Some("workspace-123"),
///     &request,
/// ).await?;
///
/// // Resolve embedding provider
/// let embed = resolver.resolve_embedding_provider("workspace-123").await?;
/// ```
pub struct WorkspaceProviderResolver {
    workspace_service: Arc<dyn WorkspaceService>,
    models_config: Option<Arc<ModelsConfig>>,
    #[cfg(feature = "postgres")]
    pg_pool: Option<sqlx::PgPool>,
}

impl WorkspaceProviderResolver {
    /// Create a new resolver with the given workspace service.
    pub fn new(workspace_service: Arc<dyn WorkspaceService>) -> Self {
        Self {
            workspace_service,
            models_config: None,
            #[cfg(feature = "postgres")]
            pg_pool: None,
        }
    }

    /// Attach models.toml config for first-principles credential gating.
    pub fn with_models_config(mut self, models_config: Arc<ModelsConfig>) -> Self {
        self.models_config = Some(models_config);
        self
    }

    /// Resolver wired from [`AppState`] (workspace service + models config).
    pub fn from_app_state(state: &crate::state::AppState) -> Self {
        let resolver = Self::new(state.workspace_service.clone())
            .with_models_config(state.query.models_config.clone());
        #[cfg(feature = "postgres")]
        {
            return resolver.with_pg_pool(state.pg_pool.clone());
        }
        #[cfg(not(feature = "postgres"))]
        {
            resolver
        }
    }

    #[cfg(feature = "postgres")]
    pub fn with_pg_pool(mut self, pool: Option<sqlx::PgPool>) -> Self {
        self.pg_pool = pool;
        self
    }

    async fn get_tenant_for_workspace(
        &self,
        workspace: &Workspace,
    ) -> Option<edgequake_core::Tenant> {
        self.workspace_service
            .get_tenant(workspace.tenant_id)
            .await
            .ok()
            .flatten()
    }

    /// Resolve LLM provider based on request and workspace configuration.
    ///
    /// ## Priority Order
    ///
    /// 1. If request has explicit provider/model, use that
    /// 2. If workspace_id provided, use workspace's LLM config
    /// 3. Return None to indicate server default should be used
    ///
    /// ## Error Handling
    ///
    /// - If explicit provider is requested but creation fails, returns error
    /// - If workspace provider fails, logs warning and returns None
    ///
    /// @implements OODA-226: Unified LLM resolution with safety limits
    pub async fn resolve_llm_provider(
        &self,
        workspace_id: Option<&str>,
        request: &LlmResolutionRequest,
    ) -> Result<Option<ResolvedLlmProvider>, ProviderResolutionError> {
        // Parse provider/model from request (supports legacy format)
        let (provider_name, model_name) = self.parse_provider_model(request);

        // Case 1: Explicit request fields — SPEC-123 SSOT gap-fills from workspace/env.
        if provider_name.as_ref().is_some_and(|p| !p.is_empty())
            || model_name.as_ref().is_some_and(|m| !m.is_empty())
        {
            let workspace = if let Some(ws_id) = workspace_id {
                self.get_workspace(ws_id).await?
            } else {
                None
            };
            let tenant = if let Some(ref ws) = workspace {
                self.get_tenant_for_workspace(ws).await
            } else {
                None
            };
            let choice = edgequake_core::resolve_llm_choice(
                provider_name.as_deref(),
                model_name.as_deref(),
                workspace.as_ref(),
                tenant.as_ref(),
            );
            if let Some(resolved) = self.try_create_llm_provider(
                &choice.provider,
                &choice.model,
                ProviderSource::from(choice.source),
                request.extra_headers.clone(),
            )? {
                return Ok(Some(resolved));
            }
        }

        // Case 2: Workspace / tenant / env via SPEC-123 SSOT (Query role after choice).
        if let Some(ws_id) = workspace_id {
            if let Some(workspace) = self.get_workspace(ws_id).await? {
                if let Some(from_conn) = self
                    .try_llm_from_workspace_connection(&workspace, edgequake_core::LlmRole::Query)
                    .await?
                {
                    return Ok(Some(from_conn));
                }
                let tenant = self.get_tenant_for_workspace(&workspace).await;
                let role =
                    edgequake_core::resolve_role_llm(&workspace, edgequake_core::LlmRole::Query);
                // Prefer deliberate query role when set; else resolve_llm_choice.
                let (provider, model, source) = if workspace
                    .metadata
                    .get("llm_roles")
                    .and_then(|v| v.get("query"))
                    .is_some()
                {
                    (role.provider, role.model, ProviderSource::Workspace)
                } else {
                    let choice = edgequake_core::resolve_llm_choice(
                        None,
                        None,
                        Some(&workspace),
                        tenant.as_ref(),
                    );
                    (
                        choice.provider,
                        choice.model,
                        ProviderSource::from(choice.source),
                    )
                };
                if !provider.is_empty() {
                    if let Some(resolved) =
                        self.try_create_llm_provider(&provider, &model, source, None)?
                    {
                        return Ok(Some(resolved));
                    }
                }
            }
        }

        // Case 3: No explicit provider, no workspace - use server default
        Ok(None)
    }

    /// Resolve LLM provider with an already-loaded workspace.
    ///
    /// Use this when the workspace has already been fetched by the handler.
    /// This avoids duplicate database queries.
    ///
    /// ## Priority Order
    ///
    /// 1. If request has explicit provider/model, use that
    /// 2. If workspace provided, use workspace's LLM config
    /// 3. Return None to indicate server default should be used
    ///
    /// @implements OODA-227: Efficient resolution with pre-loaded workspace
    /// SPEC-123: sync resolve with optional tenant (pass real tenant when loaded).
    pub fn resolve_llm_provider_with_workspace(
        &self,
        workspace: Option<&Workspace>,
        tenant: Option<&edgequake_core::Tenant>,
        request: &LlmResolutionRequest,
    ) -> Result<Option<ResolvedLlmProvider>, ProviderResolutionError> {
        let (provider_name, model_name) = self.parse_provider_model(request);

        // Case 1: Explicit request — SSOT gap-fills via workspace/tenant/env.
        if provider_name.as_ref().is_some_and(|p| !p.is_empty())
            || model_name.as_ref().is_some_and(|m| !m.is_empty())
        {
            let choice = edgequake_core::resolve_llm_choice(
                provider_name.as_deref(),
                model_name.as_deref(),
                workspace,
                tenant,
            );
            if let Some(resolved) = self.try_create_llm_provider(
                &choice.provider,
                &choice.model,
                ProviderSource::from(choice.source),
                request.extra_headers.clone(),
            )? {
                return Ok(Some(resolved));
            }
        }

        // Case 2: Query role override, else SPEC-123 resolve_llm_choice.
        if let Some(ws) = workspace {
            if ws
                .metadata
                .get("llm_roles")
                .and_then(|v| v.get("query"))
                .is_some()
            {
                let role = edgequake_core::resolve_role_llm(ws, edgequake_core::LlmRole::Query);
                if !role.provider.is_empty() {
                    if let Some(resolved) = self.try_create_llm_provider(
                        &role.provider,
                        &role.model,
                        ProviderSource::Workspace,
                        None,
                    )? {
                        return Ok(Some(resolved));
                    }
                }
            } else {
                let choice = edgequake_core::resolve_llm_choice(None, None, Some(ws), tenant);
                if !choice.provider.is_empty() {
                    if let Some(resolved) = self.try_create_llm_provider(
                        &choice.provider,
                        &choice.model,
                        ProviderSource::from(choice.source),
                        None,
                    )? {
                        return Ok(Some(resolved));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Async helper: load tenant then resolve (LAW-123-2).
    pub async fn resolve_llm_provider_for_workspace(
        &self,
        workspace: Option<&Workspace>,
        request: &LlmResolutionRequest,
    ) -> Result<Option<ResolvedLlmProvider>, ProviderResolutionError> {
        let tenant = match workspace {
            Some(ws) => self.get_tenant_for_workspace(ws).await,
            None => None,
        };
        if let Some(ws) = workspace {
            if let Some(from_conn) = self
                .try_llm_from_workspace_connection(ws, edgequake_core::LlmRole::Query)
                .await?
            {
                return Ok(Some(from_conn));
            }
        }
        self.resolve_llm_provider_with_workspace(workspace, tenant.as_ref(), request)
    }

    async fn try_llm_from_workspace_connection(
        &self,
        workspace: &Workspace,
        role: edgequake_core::LlmRole,
    ) -> Result<Option<ResolvedLlmProvider>, ProviderResolutionError> {
        let Some(cfg) = edgequake_core::role_config_from_workspace(workspace, role) else {
            return Ok(None);
        };
        let Some(raw_id) = cfg.connection_id.filter(|s| !s.is_empty()) else {
            return Ok(None);
        };
        let Some(id) = crate::providers::connection_store::parse_connection_id(&raw_id) else {
            return Ok(None);
        };
        #[cfg(feature = "postgres")]
        {
            let Some(pool) = self.pg_pool.as_ref() else {
                return Ok(None);
            };
            let resolved = edgequake_core::resolve_role_llm(workspace, role);
            match crate::providers::connection_store::load_connection_spec(
                pool,
                id,
                &resolved.model,
            )
            .await
            {
                Ok(spec) => {
                    match crate::providers::connection_factory::llm_from_connection(&spec) {
                        Ok(provider) => Ok(Some(ResolvedLlmProvider {
                            provider,
                            provider_name: spec.shape,
                            model_name: resolved.model,
                            source: ProviderSource::Workspace,
                        })),
                        Err(_) => Ok(None),
                    }
                }
                Err(_) => Ok(None),
            }
        }
        #[cfg(not(feature = "postgres"))]
        {
            let _ = id;
            Ok(None)
        }
    }

    /// Try to build an LLM provider; returns `None` to fall through to server default.
    ///
    /// Skips providers without configured credentials. Auth/key creation failures also
    /// fall through so query-time resolution prefers a working server default.
    fn try_create_llm_provider(
        &self,
        provider: &str,
        model: &str,
        source: ProviderSource,
        extra_headers: Option<std::collections::HashMap<String, String>>,
    ) -> Result<Option<ResolvedLlmProvider>, ProviderResolutionError> {
        let credentials_ok = self
            .models_config
            .as_ref()
            .map(|cfg| llm_provider_credentials_configured(cfg, provider))
            .unwrap_or_else(|| {
                crate::providers::credentials::llm_provider_credentials_configured_by_name(provider)
            });

        if !credentials_ok {
            warn!(
                provider = provider,
                model = model,
                ?source,
                "LLM provider skipped: credentials not configured for this runtime"
            );
            return Ok(None);
        }

        match self.create_llm_provider_with_headers(provider, model, source, extra_headers) {
            Ok(resolved) => Ok(Some(resolved)),
            Err(e) if e.is_api_key_error() => {
                warn!(
                    provider = provider,
                    model = model,
                    ?source,
                    error = %e,
                    "LLM provider skipped: credential/auth failure; using server default"
                );
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Resolve embedding provider for a workspace.
    ///
    /// Unlike LLM providers, embedding providers are always workspace-specific
    /// because the embedding dimension must match the vector storage.
    ///
    /// **NOTE**: Similar logic exists in `handlers/query.rs::get_workspace_embedding_provider`.
    /// The query.rs version returns `Option` for fallback semantics while this returns
    /// an error if the workspace has no embedding provider configured.
    /// See OODA-235 for duplication analysis.
    ///
    /// @implements OODA-226: Unified embedding resolution with safety limits
    pub async fn resolve_embedding_provider(
        &self,
        workspace_id: &str,
    ) -> Result<ResolvedEmbeddingProvider, ProviderResolutionError> {
        let workspace = self.get_workspace(workspace_id).await?.ok_or_else(|| {
            ProviderResolutionError::WorkspaceNotFound {
                workspace_id: workspace_id.to_string(),
            }
        })?;

        // SPEC-123: Workspace → Tenant → Env (tenant via workspace inherit on load).
        let choice = {
            let tenant = self.get_tenant_for_workspace(&workspace).await;
            edgequake_core::resolve_embedding_choice(
                None,
                None,
                None,
                Some(&workspace),
                tenant.as_ref(),
            )
        };

        if choice.provider.is_empty() {
            return Err(ProviderResolutionError::InvalidProviderName(
                "Workspace embedding provider is not configured".to_string(),
            ));
        }

        debug!(
            workspace_id = workspace_id,
            provider = %choice.provider,
            model = %choice.model,
            dimension = choice.dimension,
            "Creating workspace embedding provider"
        );

        let provider =
            create_safe_embedding_provider(&choice.provider, &choice.model, choice.dimension)
                .map_err(|e| {
                    ProviderResolutionError::from_creation_error(
                        &choice.provider,
                        &choice.model,
                        &e.to_string(),
                    )
                })?;

        info!(
            workspace_id = workspace_id,
            provider = %choice.provider,
            model = %choice.model,
            dimension = choice.dimension,
            "Workspace embedding provider created"
        );

        Ok(ResolvedEmbeddingProvider {
            provider,
            provider_name: choice.provider,
            model_name: choice.model,
            dimension: choice.dimension,
        })
    }

    /// Resolve embedding provider for query execution, with optional fallback.
    ///
    /// Returns `Ok(None)` if workspace has no embedding provider configured,
    /// allowing the caller to fall back to the server default.
    ///
    /// This consolidates the logic from `handlers/query.rs::get_workspace_embedding_provider`.
    ///
    /// @implements OODA-259: Single source of truth for embedding resolution
    pub async fn resolve_embedding_provider_optional(
        &self,
        workspace_id: &str,
    ) -> Result<Option<ResolvedEmbeddingProvider>, ProviderResolutionError> {
        let workspace = match self.get_workspace(workspace_id).await? {
            Some(ws) => ws,
            None => {
                return Err(ProviderResolutionError::WorkspaceNotFound {
                    workspace_id: workspace_id.to_string(),
                })
            }
        };

        // SPEC-123 SSOT (tenant applied via workspace inherit).
        let choice = {
            let tenant = self.get_tenant_for_workspace(&workspace).await;
            edgequake_core::resolve_embedding_choice(
                None,
                None,
                None,
                Some(&workspace),
                tenant.as_ref(),
            )
        };

        if choice.provider.is_empty() {
            debug!(
                workspace_id = workspace_id,
                "Workspace has no embedding provider configured, using server default"
            );
            return Ok(None);
        }

        debug!(
            workspace_id = workspace_id,
            provider = %choice.provider,
            model = %choice.model,
            dimension = choice.dimension,
            "Creating workspace embedding provider"
        );

        match create_safe_embedding_provider(&choice.provider, &choice.model, choice.dimension) {
            Ok(provider) => {
                info!(
                    workspace_id = workspace_id,
                    provider = %choice.provider,
                    model = %choice.model,
                    dimension = choice.dimension,
                    "Workspace embedding provider created"
                );

                Ok(Some(ResolvedEmbeddingProvider {
                    provider,
                    provider_name: choice.provider,
                    model_name: choice.model,
                    dimension: choice.dimension,
                }))
            }
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("OPENAI_API_KEY") {
                    warn!(
                        workspace_id = workspace_id,
                        provider = %choice.provider,
                        model = %choice.model,
                        "Workspace embedding provider requires OPENAI_API_KEY - using server default"
                    );
                } else {
                    warn!(
                        workspace_id = workspace_id,
                        provider = %choice.provider,
                        model = %choice.model,
                        error = %e,
                        "Failed to create workspace embedding provider - using server default"
                    );
                }
                Ok(None)
            }
        }
    }

    /// Parse provider and model from request, supporting legacy format.
    fn parse_provider_model(
        &self,
        request: &LlmResolutionRequest,
    ) -> (Option<String>, Option<String>) {
        if let Some(ref provider_id) = request.provider {
            if provider_id.is_empty() {
                return (None, None);
            }

            // If explicit model provided, use that
            if let Some(ref explicit_model) = request.model {
                return (Some(provider_id.clone()), Some(explicit_model.clone()));
            }

            // Check for legacy format: "provider/model"
            if let Some((p, m)) = provider_id.split_once('/') {
                return (Some(p.to_string()), Some(m.to_string()));
            }

            // Just provider name — leave model unset; SPEC-123 SSOT gap-fills (LAW-123-8).
            (Some(provider_id.clone()), None)
        } else {
            (None, None)
        }
    }

    /// Create an LLM provider with safety limits and optional caller-supplied headers.
    fn create_llm_provider_with_headers(
        &self,
        provider: &str,
        model: &str,
        source: ProviderSource,
        extra_headers: Option<std::collections::HashMap<String, String>>,
    ) -> Result<ResolvedLlmProvider, ProviderResolutionError> {
        debug!(
            provider = provider,
            model = model,
            ?source,
            "Creating LLM provider"
        );

        let ctx = build_application_context(extra_headers.as_ref(), None);
        let provider_arc =
            create_safe_llm_provider_with_context(provider, model, ctx).map_err(|e| {
                ProviderResolutionError::from_creation_error(provider, model, &e.to_string())
            })?;

        info!(
            provider = provider,
            model = model,
            ?source,
            "LLM provider created with safety limits"
        );

        Ok(ResolvedLlmProvider {
            provider: provider_arc,
            provider_name: provider.to_string(),
            model_name: model.to_string(),
            source,
        })
    }

    /// Get workspace by ID.
    async fn get_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Option<Workspace>, ProviderResolutionError> {
        let uuid = Uuid::parse_str(workspace_id)
            .map_err(|e| ProviderResolutionError::InvalidWorkspaceId(e.to_string()))?;

        self.workspace_service
            .get_workspace(uuid)
            .await
            .map_err(|e| ProviderResolutionError::WorkspaceServiceError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_legacy_format() {
        // This test doesn't need async or workspace service
        let request =
            LlmResolutionRequest::from_provider_string(Some("ollama/gemma3:12b".to_string()), None);
        assert!(request.has_explicit_provider());
    }

    #[test]
    fn test_parse_new_format() {
        let request = LlmResolutionRequest::from_provider_string(
            Some("openai".to_string()),
            Some("gpt-4o-mini".to_string()),
        );
        assert!(request.has_explicit_provider());
    }

    #[test]
    fn test_empty_provider() {
        let request = LlmResolutionRequest::from_provider_string(Some("".to_string()), None);
        assert!(!request.has_explicit_provider());
    }

    #[test]
    fn test_no_provider() {
        let request = LlmResolutionRequest::default();
        assert!(!request.has_explicit_provider());
    }

    // Integration tests with InMemoryWorkspaceService
    mod integration {
        use super::*;
        use edgequake_core::{
            CreateWorkspaceRequest, InMemoryWorkspaceService, Tenant, Workspace, WorkspaceService,
        };
        use serial_test::serial;
        use std::sync::Arc;

        /// Mock is forbidden at runtime unless tests opt in (SPEC-043).
        fn allow_mock_provider() {
            // SAFETY: tests are `#[serial]` so env mutation is exclusive.
            unsafe {
                std::env::set_var(crate::provider_visibility::ALLOW_MOCK_PROVIDER_ENV, "1");
            }
        }

        async fn create_test_workspace(
            service: &Arc<dyn WorkspaceService>,
        ) -> (uuid::Uuid, uuid::Uuid) {
            // Create a tenant first
            let tenant = Tenant::new("Test Tenant", "test-tenant");
            let tenant = service
                .create_tenant(tenant)
                .await
                .expect("Failed to create tenant");

            // Create a workspace with LLM config
            let request = CreateWorkspaceRequest {
                name: "Test Workspace".to_string(),
                slug: Some("test-workspace".to_string()),
                description: None,
                max_documents: None,
                llm_provider: Some("mock".to_string()),
                llm_model: Some("mock-model".to_string()),
                embedding_provider: Some("mock".to_string()),
                embedding_model: Some("mock-embedding".to_string()),
                embedding_dimension: Some(1536),
                vision_llm_model: None,
                vision_llm_provider: None,
                pdf_parser_backend: None,
                entity_types: None,
                entity_types_strict: None,
                extraction_language: None,
                chunking_mode: None,
                chunk_token_size: None,
                chunk_overlap_token_size: None,
                extract_budget_mode: None,
                extract_max_entities: None,
                extract_max_records: None,
                extraction_mode: None,
                decision_gate_preset: None,
                decision_model: None,
                decision_pack_size: None,
                decision_enabled: None,
                entity_type_colors: None,
                relation_types: None,
                relation_types_strict: None,
                kg_schema_preset: None,
                relation_edges: None,
                default_reasoning_effort: None,
                llm_roles: None,
                vision_extract_images: None,
                vision_extract_charts: None,
                vision_extract_figures: None,
                vision_page_system_prompt: None,
                vision_image_system_prompt: None,
                vision_chart_system_prompt: None,
                vision_figure_system_prompt: None,
            };

            let workspace = service
                .create_workspace(tenant.tenant_id, request)
                .await
                .expect("Failed to create workspace");

            (workspace.workspace_id, tenant.tenant_id)
        }

        #[tokio::test]
        #[serial]
        async fn test_resolve_explicit_provider() {
            allow_mock_provider();
            let service: Arc<dyn WorkspaceService> = Arc::new(InMemoryWorkspaceService::new());
            let resolver = WorkspaceProviderResolver::new(service);

            let request = LlmResolutionRequest::from_provider_string(
                Some("mock".to_string()),
                Some("test-model".to_string()),
            );

            let result = resolver
                .resolve_llm_provider_with_workspace(None, None, &request)
                .expect("Should resolve provider");

            assert!(result.is_some());
            let resolved = result.unwrap();
            assert_eq!(resolved.provider_name, "mock");
            assert_eq!(resolved.model_name, "test-model");
            assert_eq!(resolved.source, ProviderSource::Request);
        }

        #[tokio::test]
        #[serial]
        async fn test_resolve_from_workspace() {
            allow_mock_provider();
            let service: Arc<dyn WorkspaceService> = Arc::new(InMemoryWorkspaceService::new());
            let (workspace_id, _) = create_test_workspace(&service).await;
            let resolver = WorkspaceProviderResolver::new(service.clone());

            // No explicit provider in request
            let request = LlmResolutionRequest::default();

            let result = resolver
                .resolve_llm_provider(Some(&workspace_id.to_string()), &request)
                .await
                .expect("Should resolve provider");

            assert!(result.is_some());
            let resolved = result.unwrap();
            assert_eq!(resolved.provider_name, "mock");
            assert_eq!(resolved.model_name, "mock-model");
            assert_eq!(resolved.source, ProviderSource::Workspace);
        }

        #[tokio::test]
        #[serial]
        async fn test_explicit_overrides_workspace() {
            allow_mock_provider();
            let service: Arc<dyn WorkspaceService> = Arc::new(InMemoryWorkspaceService::new());
            let (workspace_id, _) = create_test_workspace(&service).await;
            let resolver = WorkspaceProviderResolver::new(service.clone());

            // Explicit provider should override workspace config
            let request = LlmResolutionRequest::from_provider_string(
                Some("mock".to_string()),
                Some("explicit-model".to_string()),
            );

            let result = resolver
                .resolve_llm_provider(Some(&workspace_id.to_string()), &request)
                .await
                .expect("Should resolve provider");

            assert!(result.is_some());
            let resolved = result.unwrap();
            assert_eq!(resolved.model_name, "explicit-model");
            assert_eq!(resolved.source, ProviderSource::Request);
        }

        #[tokio::test]
        async fn test_no_workspace_no_provider() {
            let service: Arc<dyn WorkspaceService> = Arc::new(InMemoryWorkspaceService::new());
            let resolver = WorkspaceProviderResolver::new(service);

            let request = LlmResolutionRequest::default();

            let result = resolver
                .resolve_llm_provider(None, &request)
                .await
                .expect("Should return None for server default");

            assert!(result.is_none());
        }

        #[tokio::test]
        #[serial]
        async fn test_resolve_tenant_via_for_workspace_when_no_ws_override() {
            allow_mock_provider();
            let service: Arc<dyn WorkspaceService> = Arc::new(InMemoryWorkspaceService::new());
            let mut tenant = Tenant::new("Tenant LLM", "tenant-llm-spec123");
            tenant.default_llm_provider = "mock".to_string();
            tenant.default_llm_model = "tenant-model".to_string();
            let tenant = service.create_tenant(tenant).await.expect("tenant");

            let mut ws = Workspace::new(tenant.tenant_id, "Bare WS", "bare-ws");
            // Painted concrete values without metadata → LAW-123-8: not an override.
            ws.llm_provider = "ollama".to_string();
            ws.llm_model = "painted-not-override".to_string();
            ws.metadata.clear();
            let ws = service.insert_workspace(ws).await.expect("workspace");

            let resolver = WorkspaceProviderResolver::new(service);
            let request = LlmResolutionRequest::default();
            let result = resolver
                .resolve_llm_provider_for_workspace(Some(&ws), &request)
                .await
                .expect("resolve");
            let resolved = result.expect("provider");
            assert_eq!(resolved.provider_name, "mock");
            assert_eq!(resolved.model_name, "tenant-model");
            assert_eq!(resolved.source, ProviderSource::Tenant);
        }

        #[tokio::test]
        async fn test_invalid_workspace_id() {
            let service: Arc<dyn WorkspaceService> = Arc::new(InMemoryWorkspaceService::new());
            let resolver = WorkspaceProviderResolver::new(service);

            let request = LlmResolutionRequest::default();

            let result = resolver
                .resolve_llm_provider(Some("not-a-uuid"), &request)
                .await;

            assert!(result.is_err());
            match result.unwrap_err() {
                ProviderResolutionError::InvalidWorkspaceId(_) => {}
                other => panic!("Expected InvalidWorkspaceId, got {:?}", other),
            }
        }
    }
}
