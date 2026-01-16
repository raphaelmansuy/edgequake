//! Workspace Provider Resolution Module
//!
//! This module provides unified provider resolution for LLM and embedding
//! providers across all EdgeQuake API handlers.
//!
//! ## Design Goals
//!
//! 1. **Eliminate Duplication**: Single source of truth for provider resolution
//! 2. **Consistent Safety**: All providers wrapped with timeout/limit protection
//! 3. **Clear Error Handling**: Structured errors with actionable information
//! 4. **Testability**: Resolver can be mocked for unit testing
//!
//! ## Usage
//!
//! ```rust,ignore
//! use edgequake_api::providers::{WorkspaceProviderResolver, LlmResolutionRequest};
//!
//! // Create resolver with workspace service
//! let resolver = WorkspaceProviderResolver::new(workspace_service);
//!
//! // Resolve LLM provider for a request
//! let request = LlmResolutionRequest::from_provider_string(
//!     req.provider.clone(),
//!     req.model.clone(),
//! );
//! let resolved = resolver.resolve_llm_provider(
//!     workspace_id.as_deref(),
//!     &request,
//! ).await?;
//!
//! // Use the resolved provider
//! if let Some(resolved) = resolved {
//!     let response = resolved.provider.complete("Hello").await?;
//! }
//! ```
//!
//! ## Migration Guide
//!
//! Replace direct calls to `ProviderFactory::create_llm_provider` with:
//!
//! ```rust,ignore
//! // Before (duplicated in multiple handlers):
//! let (provider_name, model_name) = if let Some((p, m)) = provider_id.split_once('/') {
//!     (p.to_string(), m.to_string())
//! } else {
//!     // ... complex fallback logic
//! };
//! match ProviderFactory::create_llm_provider(&provider_name, &model_name) {
//!     // ... error handling
//! }
//!
//! // After (single call):
//! let request = LlmResolutionRequest::from_provider_string(provider_id, model);
//! let resolved = resolver.resolve_llm_provider(workspace_id, &request).await?;
//! ```
//!
//! @implements OODA-226: Unified provider resolution module

mod error;
mod resolver;

pub use error::ProviderResolutionError;
pub use resolver::{
    LlmResolutionRequest, ProviderSource, ResolvedEmbeddingProvider, ResolvedLlmProvider,
    WorkspaceProviderResolver,
};
