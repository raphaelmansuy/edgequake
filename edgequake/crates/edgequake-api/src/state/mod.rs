//! Application state and storage mode configuration.
//!
//! This module manages the central application state shared across all handlers,
//! including storage backends, service instances, and configuration.
//!
//! ## Implements
//!
//! - [`FEAT0460`]: Centralized application state
//! - [`FEAT0461`]: Storage mode selection (Memory/PostgreSQL)
//! - [`FEAT0462`]: Service instance management
//!
//! ## Use Cases
//!
//! - [`UC2060`]: System initializes storage adapters
//! - [`UC2061`]: Handlers access shared services
//!
//! ## Enforces
//!
//! - [`BR0460`]: Thread-safe state via Arc
//! - [`BR0461`]: Configurable storage backends
//!
//! # Storage Modes
//!
//! EdgeQuake supports two storage modes:
//!
//! - **Memory**: In-memory storage (ephemeral, for testing)
//! - **PostgreSQL**: Persistent storage with AGE graph extensions
//!
//! # State Components
//!
//! ```text
//! AppState
//! ├── Storage Adapters
//! │   ├── KV Storage (documents, metadata)
//! │   ├── Vector Storage (embeddings)
//! │   └── Graph Storage (entities, relationships)
//! ├── Services
//! │   ├── QueryEngine (hybrid search)
//! │   ├── Pipeline (document processing)
//! │   ├── ConversationService
//! │   └── WorkspaceService
//! ├── Infrastructure
//! │   ├── TaskQueue (async processing)
//! │   ├── CacheManager (hot data)
//! │   └── ProgressBroadcaster (real-time updates)
//! └── Configuration
//!     ├── AuthConfig
//!     ├── RateLimitConfig
//!     └── AppConfig
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use edgequake_api::{AppState, StorageMode, AppConfig};
//!
//! let config = AppConfig {
//!     storage_mode: StorageMode::Memory,
//!     max_document_size: 10_000_000, // 10MB
//!     max_query_length: 10_000,
//!     ..Default::default()
//! };
//!
//! let state = AppState::new(config).await?;
//! ```
//!
//! # Thread Safety
//!
//! All state components use Arc for shared ownership and are designed
//! for concurrent access across multiple request handlers.

mod config;
mod memory;
#[cfg(feature = "postgres")]
mod postgres;

pub use config::*;

use std::sync::Arc;

use crate::cache_manager::CacheManager;
use crate::handlers::ProgressBroadcaster;
use edgequake_auth::AuthConfig;
use edgequake_llm::ModelsConfig;
use edgequake_pipeline::Pipeline;
use edgequake_query::{QueryEngine, SOTAQueryEngine};
use edgequake_rate_limiter::RateLimiter;
use edgequake_tasks::{PipelineState, SharedTaskQueue, SharedTaskStorage};

#[cfg(feature = "postgres")]
use sqlx::PgPool;

// ── Shared Utility ────────────────────────────────────────────────────────

/// Create the configured BM25 reranker.
///
/// Enhanced mode (default) adds:
/// - Porter2 stemming: "running" matches "run", "runner"
/// - NFKD Unicode normalization: "café" matches "cafe"
/// - Stop word filtering: Removes noise words like "the", "and"
///
/// Set `BM25_ENHANCED=false` to disable enhanced features.
fn create_bm25_reranker() -> Arc<dyn edgequake_llm::Reranker> {
    if std::env::var("BM25_ENHANCED").unwrap_or_default() == "false" {
        tracing::info!("Using minimal BM25 reranker (BM25_ENHANCED=false)");
        Arc::new(edgequake_llm::reranker::BM25Reranker::new())
    } else {
        tracing::info!("Using enhanced BM25 reranker with stemming and Unicode normalization");
        Arc::new(edgequake_llm::reranker::BM25Reranker::new_enhanced())
    }
}

// ── AppState ──────────────────────────────────────────────────────────────

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// KV storage.
    pub kv_storage: Arc<dyn edgequake_storage::traits::KVStorage>,

    /// Vector storage (default, for backward compatibility).
    pub vector_storage: Arc<dyn edgequake_storage::traits::VectorStorage>,

    /// Workspace vector registry for per-workspace vector storage.
    /// Each workspace can have its own dimension based on its embedding provider.
    pub vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry>,

    /// Graph storage.
    pub graph_storage: Arc<dyn edgequake_storage::traits::GraphStorage>,

    /// PDF document storage (SPEC-007).
    #[cfg(feature = "postgres")]
    pub pdf_storage: Option<Arc<dyn edgequake_storage::PdfDocumentStorage>>,

    /// LLM provider.
    pub llm_provider: Arc<dyn edgequake_llm::traits::LLMProvider>,

    /// Embedding provider.
    pub embedding_provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider>,

    /// Query engine.
    pub query_engine: Arc<QueryEngine>,

    /// SOTA Query engine with LightRAG-style enhancements.
    pub sota_engine: Arc<SOTAQueryEngine>,

    /// Processing pipeline.
    pub pipeline: Arc<Pipeline>,

    /// Task storage.
    pub task_storage: SharedTaskStorage,

    /// Task queue.
    pub task_queue: SharedTaskQueue,

    /// Pipeline state for real-time progress tracking (Phase 3).
    pub pipeline_state: PipelineState,

    /// Progress broadcaster for WebSocket clients (Phase 5).
    pub progress_broadcaster: ProgressBroadcaster,

    /// Workspace service for tenant/workspace management.
    pub workspace_service: SharedWorkspaceService,

    /// Conversation service for managing chat sessions.
    pub conversation_service: SharedConversationService,

    /// Configuration.
    pub config: AppConfig,

    /// Auth configuration.
    pub auth_config: AuthConfig,

    /// JWT service.
    pub jwt_service: Arc<edgequake_auth::JwtService>,

    /// Password service.
    pub password_service: Arc<edgequake_auth::PasswordService>,

    /// RBAC service.
    pub rbac_service: Arc<edgequake_auth::RbacService>,

    /// Cache manager for conversations and messages.
    pub cache_manager: CacheManager,

    /// Rate limiter for tenant-based rate limiting.
    pub rate_limiter: RateLimiter,

    /// Storage mode indicator (memory or postgresql).
    pub storage_mode: StorageMode,

    /// Models configuration (providers, model cards, capabilities).
    pub models_config: Arc<ModelsConfig>,

    /// PostgreSQL pool (only available when using postgres feature).
    #[cfg(feature = "postgres")]
    pub pg_pool: Option<PgPool>,

    /// Server start time for uptime calculation.
    pub start_time: std::time::Instant,

    /// Path validation configuration for filesystem access security (OODA-248).
    /// WHY: Prevents directory traversal attacks in scan_directory endpoint.
    pub path_validation_config: crate::path_validation::PathValidationConfig,

    /// Per-task cancellation registry.
    /// WHY: The cancel_task handler triggers cooperative cancellation of in-flight
    /// tasks by signalling the token registered here. Workers check the token
    /// at every pipeline stage boundary to stop processing promptly.
    pub cancellation_registry: edgequake_tasks::CancellationRegistry,
}

// ── Operational Methods ───────────────────────────────────────────────────

impl AppState {
    /// Initialize default tenant and workspace for non-authenticated mode.
    /// This ensures that the system is usable without authentication.
    ///
    /// When using PostgreSQL, the WorkspaceServiceImpl already ensures
    /// defaults exist during construction, so this primarily handles the
    /// in-memory case and ensures the default user exists.
    pub async fn initialize_defaults(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use edgequake_core::{CreateWorkspaceRequest, Tenant, TenantPlan};

        // Define default user ID for anonymous/unauthenticated access
        // WHY: Used only in postgres feature block, suppressed warning with allow
        #[allow(unused_variables)]
        let default_user_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001")
            .expect("Invalid default user UUID");

        // Define default tenant ID for consistency
        let default_tenant_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002")
            .expect("Invalid default tenant UUID");

        // When using PostgreSQL, just ensure the default user exists
        // The WorkspaceServiceImpl already creates default tenant/workspace
        #[cfg(feature = "postgres")]
        if let Some(ref pool) = self.pg_pool {
            // Ensure default user exists in PostgreSQL (with tenant_id for FK constraints)
            sqlx::query(
                r#"
                INSERT INTO users (user_id, tenant_id, username, email, password_hash, role, is_active, created_at, updated_at)
                VALUES ($1, $2, 'default_user', 'default@edgequake.local', 'not_a_real_hash', 'user', TRUE, NOW(), NOW())
                ON CONFLICT (user_id) DO NOTHING
                "#,
            )
            .bind(default_user_id)
            .bind(default_tenant_id)
            .execute(pool)
            .await?;

            tracing::info!(
                user_id = %default_user_id,
                tenant_id = %default_tenant_id,
                "Ensured default user exists in PostgreSQL"
            );

            // PostgreSQL mode: tenant and workspace already created by WorkspaceServiceImpl
            tracing::info!("PostgreSQL mode: defaults already ensured by WorkspaceServiceImpl");
            return Ok(());
        }

        // In-memory mode: Check if default tenant already exists
        let existing = self.workspace_service.list_tenants(10, 0).await?;

        if !existing.is_empty() {
            tracing::info!(
                "Found {} existing tenant(s), skipping default initialization",
                existing.len()
            );
            return Ok(());
        }

        // Create default tenant for in-memory mode
        let mut default_tenant = Tenant::new("Default", "default")
            .with_plan(TenantPlan::Pro)
            .with_description("Default tenant for EdgeQuake");
        default_tenant.tenant_id = default_tenant_id;

        let tenant = self.workspace_service.create_tenant(default_tenant).await?;

        tracing::info!(
            tenant_id = %tenant.tenant_id,
            "Created default tenant"
        );

        // Create default workspace within the tenant
        // SPEC-032: Uses server defaults for embedding configuration
        let workspace_request = CreateWorkspaceRequest::new("Default Workspace")
            .with_embedding_model("text-embedding-3-small");

        let workspace = self
            .workspace_service
            .create_workspace(tenant.tenant_id, workspace_request)
            .await?;

        tracing::info!(
            workspace_id = %workspace.workspace_id,
            tenant_id = %tenant.tenant_id,
            "Created default workspace"
        );

        Ok(())
    }

    /// Create a workspace-specific pipeline with the workspace's LLM configuration.
    ///
    /// @implements SPEC-032: Workspace-specific LLM for ingestion
    ///
    /// This method creates a temporary pipeline configured with the workspace's
    /// LLM and embedding providers. Used during document ingestion to ensure
    /// that each workspace can use its own model configuration.
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - The workspace ID to look up configuration for
    ///
    /// # Returns
    ///
    /// Returns a `Pipeline` configured with the workspace's LLM and embedding providers.
    /// Falls back to the global pipeline's providers if workspace config lookup fails.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let workspace_pipeline = state.create_workspace_pipeline("workspace-123").await;
    /// let result = workspace_pipeline.process(&doc_id, &content).await?;
    /// ```
    pub async fn create_workspace_pipeline(&self, workspace_id: &str) -> Arc<Pipeline> {
        use crate::safety_limits::{create_safe_embedding_provider, create_safe_llm_provider};
        use edgequake_pipeline::LLMExtractor;

        // Parse workspace_id to UUID
        let workspace_uuid = match uuid::Uuid::parse_str(workspace_id) {
            Ok(uuid) => uuid,
            Err(e) => {
                tracing::warn!(
                    workspace_id = workspace_id,
                    error = %e,
                    "Invalid workspace ID format, using global pipeline"
                );
                return Arc::clone(&self.pipeline);
            }
        };

        // Lookup workspace configuration
        let workspace_result = self.workspace_service.get_workspace(workspace_uuid).await;

        match workspace_result {
            Ok(Some(ws)) => {
                // Try to create workspace-specific LLM provider with safety limits
                // @implements FEAT0779: Safety limits for LLM calls (AppState)
                // @implements BR0777: Hard max_tokens limit enforcement
                // @implements BR0778: Request timeout enforcement
                let llm_provider = create_safe_llm_provider(&ws.llm_provider, &ws.llm_model);

                // Try to create workspace-specific embedding provider with safety limits
                let embedding_provider = create_safe_embedding_provider(
                    &ws.embedding_provider,
                    &ws.embedding_model,
                    ws.embedding_dimension,
                );

                // If both providers were created successfully, use them
                if let (Ok(llm), Ok(embedding)) = (llm_provider, embedding_provider) {
                    tracing::info!(
                        workspace_id = workspace_id,
                        llm_model = %ws.llm_full_id(),
                        embedding_model = %ws.embedding_full_id(),
                        "Using workspace-specific LLM configuration for pipeline (with safety limits)"
                    );

                    let extractor = Arc::new(LLMExtractor::new(llm));
                    return Arc::new(
                        Pipeline::default_pipeline()
                            .with_extractor(extractor)
                            .with_embedding_provider(embedding),
                    );
                }

                // Log warning and fall back to global pipeline
                tracing::warn!(
                    workspace_id = workspace_id,
                    llm_config = %ws.llm_full_id(),
                    embedding_config = %ws.embedding_full_id(),
                    "Failed to create workspace-specific providers, using global pipeline"
                );
            }
            Ok(None) => {
                tracing::warn!(
                    workspace_id = workspace_id,
                    "Workspace not found, using global pipeline"
                );
            }
            Err(e) => {
                tracing::warn!(
                    workspace_id = workspace_id,
                    error = %e,
                    "Failed to lookup workspace, using global pipeline"
                );
            }
        }

        // Fall back to global pipeline
        Arc::clone(&self.pipeline)
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_mode_as_str() {
        assert_eq!(StorageMode::Memory.as_str(), "memory");
        assert_eq!(StorageMode::PostgreSQL.as_str(), "postgresql");
    }

    #[test]
    fn test_storage_mode_is_memory() {
        assert!(StorageMode::Memory.is_memory());
        assert!(!StorageMode::PostgreSQL.is_memory());
    }

    #[test]
    fn test_storage_mode_is_postgresql() {
        assert!(StorageMode::PostgreSQL.is_postgresql());
        assert!(!StorageMode::Memory.is_postgresql());
    }

    #[test]
    fn test_storage_mode_serialization() {
        let memory = StorageMode::Memory;
        let json = serde_json::to_string(&memory).unwrap();
        assert_eq!(json, "\"memory\"");

        let postgresql = StorageMode::PostgreSQL;
        let json = serde_json::to_string(&postgresql).unwrap();
        assert_eq!(json, "\"postgresql\"");
    }

    #[test]
    fn test_storage_mode_deserialization() {
        let memory: StorageMode = serde_json::from_str("\"memory\"").unwrap();
        assert_eq!(memory, StorageMode::Memory);

        let postgresql: StorageMode = serde_json::from_str("\"postgresql\"").unwrap();
        assert_eq!(postgresql, StorageMode::PostgreSQL);
    }

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.workspace_id, "default");
        // SPEC-028: 50MB document size limit
        assert_eq!(config.max_document_size, 50 * 1024 * 1024); // 50 MB
        assert_eq!(config.max_query_length, 10000);
    }

    #[test]
    fn test_app_config_custom() {
        let config = AppConfig {
            workspace_id: "custom-workspace".to_string(),
            max_document_size: 5 * 1024 * 1024, // 5 MB
            max_query_length: 5000,
        };
        assert_eq!(config.workspace_id, "custom-workspace");
        assert_eq!(config.max_document_size, 5 * 1024 * 1024);
        assert_eq!(config.max_query_length, 5000);
    }

    #[tokio::test]
    async fn test_app_state_test_state() {
        let state = AppState::test_state();
        assert!(state.storage_mode.is_memory());
        assert_eq!(state.config.workspace_id, "default");
    }
}
