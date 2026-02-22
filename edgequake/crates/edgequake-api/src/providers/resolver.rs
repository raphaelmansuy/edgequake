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
//!  │       └── Neither? ──► Return None → caller uses sota_engine      │
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
//! ## Priority Order
//!
//! For LLM provider resolution:
//! 1. Request-specified provider/model (explicit user selection)
//! 2. Workspace-configured provider/model (workspace settings)
//! 3. Server default (fallback)
//!
//! @implements OODA-226: Unified provider resolution to eliminate code duplication

use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::safety_limits::{
    create_safe_embedding_provider, create_safe_llm_provider, default_model_for_provider,
};
use edgequake_core::{Workspace, WorkspaceService};
use edgequake_query::{EmbeddingProvider, LLMProvider};

use crate::providers::error::ProviderResolutionError;

/// Configuration for LLM provider resolution from a request.
#[derive(Debug, Clone, Default)]
pub struct LlmResolutionRequest {
    /// Provider name from request (e.g., "openai", "ollama")
    pub provider: Option<String>,
    /// Model name from request (e.g., "gpt-4o-mini", "gemma3:12b")
    pub model: Option<String>,
}

impl LlmResolutionRequest {
    /// Create from provider string that may include model (legacy format).
    ///
    /// Supports both formats:
    /// - "openai/gpt-4o-mini" (legacy)
    /// - "openai" with separate model field (new)
    pub fn from_provider_string(provider: Option<String>, model: Option<String>) -> Self {
        Self { provider, model }
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
    /// Server default fallback
    ServerDefault,
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
}

impl WorkspaceProviderResolver {
    /// Create a new resolver with the given workspace service.
    pub fn new(workspace_service: Arc<dyn WorkspaceService>) -> Self {
        Self { workspace_service }
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

        // Case 1: Explicit provider in request
        if let (Some(provider), Some(model)) = (&provider_name, &model_name) {
            return self
                .create_llm_provider(provider, model, ProviderSource::Request)
                .map(Some);
        }

        // Case 2: Get from workspace if provided
        if let Some(ws_id) = workspace_id {
            if let Some(workspace) = self.get_workspace(ws_id).await? {
                if !workspace.llm_provider.is_empty() {
                    return match self.create_llm_provider(
                        &workspace.llm_provider,
                        &workspace.llm_model,
                        ProviderSource::Workspace,
                    ) {
                        Ok(resolved) => Ok(Some(resolved)),
                        Err(e) => {
                            // Workspace provider failed - log but don't error
                            warn!(
                                workspace_id = ws_id,
                                provider = %workspace.llm_provider,
                                model = %workspace.llm_model,
                                error = %e,
                                "Workspace LLM provider failed, falling back to server default"
                            );
                            Ok(None)
                        }
                    };
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
    pub fn resolve_llm_provider_with_workspace(
        &self,
        workspace: Option<&Workspace>,
        request: &LlmResolutionRequest,
    ) -> Result<Option<ResolvedLlmProvider>, ProviderResolutionError> {
        // Parse provider/model from request (supports legacy format)
        let (provider_name, model_name) = self.parse_provider_model(request);

        // Case 1: Explicit provider in request
        if let (Some(provider), Some(model)) = (&provider_name, &model_name) {
            return self
                .create_llm_provider(provider, model, ProviderSource::Request)
                .map(Some);
        }

        // Case 2: Use workspace's LLM config if available
        if let Some(ws) = workspace {
            if !ws.llm_provider.is_empty() {
                return match self.create_llm_provider(
                    &ws.llm_provider,
                    &ws.llm_model,
                    ProviderSource::Workspace,
                ) {
                    Ok(resolved) => Ok(Some(resolved)),
                    Err(e) => {
                        // Workspace provider failed - log but don't error
                        warn!(
                            workspace_id = %ws.workspace_id,
                            provider = %ws.llm_provider,
                            model = %ws.llm_model,
                            error = %e,
                            "Workspace LLM provider failed, falling back to server default"
                        );
                        Ok(None)
                    }
                };
            }
        }

        // Case 3: No explicit provider, no workspace - use server default
        Ok(None)
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

        if workspace.embedding_provider.is_empty() {
            return Err(ProviderResolutionError::InvalidProviderName(
                "Workspace embedding provider is not configured".to_string(),
            ));
        }

        debug!(
            workspace_id = workspace_id,
            provider = %workspace.embedding_provider,
            model = %workspace.embedding_model,
            dimension = workspace.embedding_dimension,
            "Creating workspace embedding provider"
        );

        let provider = create_safe_embedding_provider(
            &workspace.embedding_provider,
            &workspace.embedding_model,
            workspace.embedding_dimension,
        )
        .map_err(|e| {
            ProviderResolutionError::from_creation_error(
                &workspace.embedding_provider,
                &workspace.embedding_model,
                &e.to_string(),
            )
        })?;

        info!(
            workspace_id = workspace_id,
            provider = %workspace.embedding_provider,
            model = %workspace.embedding_model,
            dimension = workspace.embedding_dimension,
            "Workspace embedding provider created"
        );

        Ok(ResolvedEmbeddingProvider {
            provider,
            provider_name: workspace.embedding_provider,
            model_name: workspace.embedding_model,
            dimension: workspace.embedding_dimension,
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

        // If embedding provider is not configured, return None for fallback
        if workspace.embedding_provider.is_empty() {
            debug!(
                workspace_id = workspace_id,
                "Workspace has no embedding provider configured, using server default"
            );
            return Ok(None);
        }

        debug!(
            workspace_id = workspace_id,
            provider = %workspace.embedding_provider,
            model = %workspace.embedding_model,
            dimension = workspace.embedding_dimension,
            "Creating workspace embedding provider"
        );

        match create_safe_embedding_provider(
            &workspace.embedding_provider,
            &workspace.embedding_model,
            workspace.embedding_dimension,
        ) {
            Ok(provider) => {
                info!(
                    workspace_id = workspace_id,
                    provider = %workspace.embedding_provider,
                    model = %workspace.embedding_model,
                    dimension = workspace.embedding_dimension,
                    "Workspace embedding provider created"
                );

                Ok(Some(ResolvedEmbeddingProvider {
                    provider,
                    provider_name: workspace.embedding_provider,
                    model_name: workspace.embedding_model,
                    dimension: workspace.embedding_dimension,
                }))
            }
            Err(e) => {
                // Log warning with actionable message
                let error_str = e.to_string();
                if error_str.contains("OPENAI_API_KEY") {
                    warn!(
                        workspace_id = workspace_id,
                        provider = %workspace.embedding_provider,
                        model = %workspace.embedding_model,
                        "Workspace embedding provider requires OPENAI_API_KEY - using server default"
                    );
                } else {
                    warn!(
                        workspace_id = workspace_id,
                        provider = %workspace.embedding_provider,
                        model = %workspace.embedding_model,
                        error = %e,
                        "Failed to create workspace embedding provider - using server default"
                    );
                }
                // Return None to allow fallback instead of hard error
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

            // Just provider name - use default model
            let default_model = default_model_for_provider(provider_id);
            (Some(provider_id.clone()), Some(default_model.to_string()))
        } else {
            (None, None)
        }
    }

    /// Create an LLM provider with safety limits.
    fn create_llm_provider(
        &self,
        provider: &str,
        model: &str,
        source: ProviderSource,
    ) -> Result<ResolvedLlmProvider, ProviderResolutionError> {
        debug!(
            provider = provider,
            model = model,
            ?source,
            "Creating LLM provider"
        );

        let provider_arc = create_safe_llm_provider(provider, model).map_err(|e| {
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
            CreateWorkspaceRequest, InMemoryWorkspaceService, Tenant, WorkspaceService,
        };
        use std::sync::Arc;

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
            };

            let workspace = service
                .create_workspace(tenant.tenant_id, request)
                .await
                .expect("Failed to create workspace");

            (workspace.workspace_id, tenant.tenant_id)
        }

        #[tokio::test]
        async fn test_resolve_explicit_provider() {
            let service: Arc<dyn WorkspaceService> = Arc::new(InMemoryWorkspaceService::new());
            let resolver = WorkspaceProviderResolver::new(service);

            let request = LlmResolutionRequest::from_provider_string(
                Some("mock".to_string()),
                Some("test-model".to_string()),
            );

            let result = resolver
                .resolve_llm_provider_with_workspace(None, &request)
                .expect("Should resolve provider");

            assert!(result.is_some());
            let resolved = result.unwrap();
            assert_eq!(resolved.provider_name, "mock");
            assert_eq!(resolved.model_name, "test-model");
            assert_eq!(resolved.source, ProviderSource::Request);
        }

        #[tokio::test]
        async fn test_resolve_from_workspace() {
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
        async fn test_explicit_overrides_workspace() {
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
