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
//! │   ├── QueryEngine (SOTA RAG search)
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

mod auth_runtime;
pub(crate) mod bundled_models;
mod compliance_runtime;
mod config;
mod graph_query_runtime;
mod memory;
#[cfg(feature = "postgres")]
pub mod migration_bootstrap;
#[cfg(feature = "postgres")]
mod postgres;
mod postgres_runtime;
mod provider_setup;
mod query_bootstrap;
mod query_runtime;
mod resource_runtime;
pub mod runtime_extractors;
pub mod security_config;
mod storage_runtime;
mod task_runtime;

pub use auth_runtime::AuthRuntime;
pub use compliance_runtime::ComplianceRuntime;
pub use config::*;
pub use graph_query_runtime::GraphQueryRuntime;
pub use postgres_runtime::PostgresRuntime;
pub use query_runtime::QueryRuntime;
pub use security_config::ApiSecurityConfig;
pub use storage_runtime::StorageRuntime;
pub use task_runtime::TaskRuntime;

use std::sync::Arc;

use edgequake_core::{
    GraphMaterializationSemaphore, PdfVisionSemaphore, ResourceBudgetConfig, ResourceGuard,
};

use crate::cache_manager::CacheManager;
use edgequake_pipeline::Pipeline;
use edgequake_rate_limiter::RateLimiter;

#[cfg(feature = "postgres")]
use sqlx::PgPool;

// ── AppState ──────────────────────────────────────────────────────────────

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// Storage adapters (KV, vector, graph, PDF).
    pub storage: StorageRuntime,

    /// Query engines, pipeline, and default LLM providers.
    pub query: QueryRuntime,

    /// Authentication services (JWT, password, RBAC).
    pub auth: AuthRuntime,

    /// Background task runtime (queue, storage, progress, cancellation).
    pub tasks: TaskRuntime,

    /// Workspace service for tenant/workspace management.
    pub workspace_service: SharedWorkspaceService,

    /// Conversation service for managing chat sessions.
    pub conversation_service: SharedConversationService,

    /// Configuration.
    pub config: AppConfig,

    /// Cache manager for conversations and messages.
    pub cache_manager: CacheManager,

    /// Rate limiter for tenant-based rate limiting.
    pub rate_limiter: RateLimiter,

    /// PostgreSQL pool (only available when using postgres feature).
    #[cfg(feature = "postgres")]
    pub pg_pool: Option<PgPool>,

    /// Server start time for uptime calculation.
    pub start_time: std::time::Instant,

    /// Path validation configuration for filesystem access security (OODA-248).
    pub path_validation_config: crate::path_validation::PathValidationConfig,

    /// Compliance audit logger (PostgreSQL deployments).
    pub audit_logger: Option<edgequake_audit::AuditLogger>,

    /// SPEC-006: centralized admission guard (DRY — single budget authority).
    pub resource_guard: ResourceGuard,

    /// SPEC-006: caps concurrent graph materialization queries.
    pub graph_materialize: Arc<GraphMaterializationSemaphore>,

    /// P-G13: caps concurrent vision PDF conversions process-wide.
    pub pdf_vision: Arc<PdfVisionSemaphore>,

    /// Bootstrap migration report (PostgreSQL only).
    #[cfg(feature = "postgres")]
    pub migration_bootstrap: Option<crate::state::migration_bootstrap::MigrationBootstrapReport>,

    /// SPEC-042-E: PostgreSQL runtime capabilities (uuidv7, halfvec, AGE RLS/COPY gates).
    #[cfg(feature = "postgres")]
    pub postgres_capabilities: Option<edgequake_storage::adapters::postgres::PostgresCapabilities>,

    /// SPEC-027: runtime security flags (auth hardening, CORS, rate limits).
    pub security: ApiSecurityConfig,
}

// ── Operational Methods ───────────────────────────────────────────────────

impl AppState {
    /// SPEC-006 SSOT accessor — handlers must use this instead of ad-hoc `default()` / `from_env()`.
    #[inline]
    pub fn resource_budget(&self) -> &ResourceBudgetConfig {
        self.resource_guard.budget()
    }

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
        let default_user_id = crate::middleware::default_user_uuid();

        // Define default tenant ID for consistency
        let default_tenant_id = crate::middleware::default_tenant_uuid();

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
        let mut workspace_request = CreateWorkspaceRequest::new("Default Workspace")
            .with_embedding_model("text-embedding-3-small");
        workspace_request.slug = Some("default".to_string());

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
        let factory = crate::workspace_pipeline_factory::WorkspacePipelineFactory::new(
            Arc::clone(&self.workspace_service),
            Arc::clone(&self.query.pipeline),
        );
        factory
            .resolve(
                workspace_id,
                crate::workspace_pipeline_factory::PipelineFallbackPolicy::LenientGlobal,
            )
            .await
            .unwrap_or_else(|_| Arc::clone(&self.query.pipeline))
    }

    /// Persist a task and enqueue it for background processing (SPEC-017 P2-01).
    pub async fn enqueue_task(&self, task: edgequake_tasks::Task) -> crate::error::ApiResult<()> {
        self.tasks.enqueue(task).await
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
        assert!(state.storage.is_memory());
        assert_eq!(state.config.workspace_id, "default");
        assert_eq!(state.query.embedding_provider.dimension(), 1536);
        #[cfg(feature = "postgres")]
        assert!(state.storage.pdf_storage.is_some());
    }

    /// SPEC-017: memory AppState uses ConversationServiceImpl + MemoryConversationStorage.
    #[cfg(feature = "postgres")]
    #[tokio::test]
    async fn test_memory_conversation_service_roundtrip() {
        use edgequake_core::CreateConversationRequest;
        use uuid::Uuid;

        let state = AppState::test_state();
        let tenant = Uuid::new_v4();
        let user = Uuid::new_v4();

        let conv = state
            .conversation_service
            .create_conversation(
                tenant,
                user,
                None,
                CreateConversationRequest {
                    title: Some("SPEC-017 memory".into()),
                    ..Default::default()
                },
            )
            .await
            .expect("create conversation");

        let listed = state
            .conversation_service
            .list_conversations(
                tenant,
                user,
                edgequake_core::ConversationFilter::default(),
                edgequake_core::ConversationSortField::CreatedAt,
                false,
                None,
                10,
            )
            .await
            .expect("list conversations");

        assert!(listed
            .items
            .iter()
            .any(|c| c.conversation_id == conv.conversation_id));
    }

    #[tokio::test]
    async fn test_enqueue_task_persists_and_queues() {
        use edgequake_tasks::{Task, TaskType};
        use uuid::Uuid;

        let state = AppState::test_state();
        let tenant_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let task = Task::new(
            tenant_id,
            workspace_id,
            TaskType::Insert,
            serde_json::json!({"content": "enqueue test"}),
        );
        let track_id = task.track_id.clone();

        state
            .enqueue_task(task)
            .await
            .expect("enqueue should succeed");

        let stored = state
            .tasks
            .storage
            .get_task(&track_id)
            .await
            .expect("storage read")
            .expect("task must exist");
        assert_eq!(stored.track_id, track_id);
    }
}
