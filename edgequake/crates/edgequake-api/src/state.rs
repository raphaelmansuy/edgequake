//! Application state.

use std::sync::Arc;

use crate::cache_manager::CacheManager;
use crate::handlers::ProgressBroadcaster;
use edgequake_auth::{AuthConfig, JwtService, PasswordService, RbacService};
use edgequake_core::{
    ConversationService, InMemoryConversationService, InMemoryWorkspaceService, WorkspaceService,
};
use edgequake_llm::OpenAIProvider;
use edgequake_pipeline::Pipeline;
use edgequake_query::{QueryEngine, QueryEngineConfig};
use edgequake_rate_limiter::{RateLimitConfig as TokenBucketConfig, RateLimiter};
use edgequake_storage::adapters::memory::{
    MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
};
use edgequake_tasks::{PipelineState, SharedTaskQueue, SharedTaskStorage};
use serde::{Deserialize, Serialize};

/// Storage mode indicator for the application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageMode {
    /// In-memory storage (data lost on restart).
    Memory,
    /// PostgreSQL persistent storage.
    PostgreSQL,
}

impl StorageMode {
    /// Get the storage mode as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            StorageMode::Memory => "memory",
            StorageMode::PostgreSQL => "postgresql",
        }
    }

    /// Check if using PostgreSQL storage.
    pub fn is_postgresql(&self) -> bool {
        matches!(self, StorageMode::PostgreSQL)
    }

    /// Check if using in-memory storage.
    pub fn is_memory(&self) -> bool {
        matches!(self, StorageMode::Memory)
    }
}

#[cfg(feature = "postgres")]
use crate::PostgresConversationService;
#[cfg(feature = "postgres")]
use edgequake_storage::{
    GraphStorage, KVStorage, PgVectorStorage, PostgresAGEGraphStorage, PostgresKVStorage,
    VectorStorage,
};
#[cfg(feature = "postgres")]
use sqlx::PgPool;

/// Type alias for the shared workspace service.
pub type SharedWorkspaceService = Arc<dyn WorkspaceService>;

/// Type alias for the shared conversation service.
pub type SharedConversationService = Arc<dyn ConversationService>;

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// KV storage.
    pub kv_storage: Arc<dyn edgequake_storage::traits::KVStorage>,

    /// Vector storage.
    pub vector_storage: Arc<dyn edgequake_storage::traits::VectorStorage>,

    /// Graph storage.
    pub graph_storage: Arc<dyn edgequake_storage::traits::GraphStorage>,

    /// LLM provider.
    pub llm_provider: Arc<dyn edgequake_llm::traits::LLMProvider>,

    /// Embedding provider.
    pub embedding_provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider>,

    /// Query engine.
    pub query_engine: Arc<QueryEngine>,

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
    pub jwt_service: Arc<JwtService>,

    /// Password service.
    pub password_service: Arc<PasswordService>,

    /// RBAC service.
    pub rbac_service: Arc<RbacService>,

    /// Cache manager for conversations and messages.
    pub cache_manager: CacheManager,

    /// Rate limiter for tenant-based rate limiting.
    pub rate_limiter: RateLimiter,

    /// Storage mode indicator (memory or postgresql).
    pub storage_mode: StorageMode,

    /// PostgreSQL pool (only available when using postgres feature).
    #[cfg(feature = "postgres")]
    pub pg_pool: Option<PgPool>,
}

/// Application configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Workspace/tenant ID.
    pub workspace_id: String,

    /// Maximum document size in bytes.
    pub max_document_size: usize,

    /// Maximum query length.
    pub max_query_length: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            workspace_id: "default".to_string(),
            max_document_size: 10 * 1024 * 1024, // 10 MB
            max_query_length: 10000,
        }
    }
}

impl AppState {
    /// Create a new application state.
    pub fn new(
        kv_storage: Arc<dyn edgequake_storage::traits::KVStorage>,
        vector_storage: Arc<dyn edgequake_storage::traits::VectorStorage>,
        graph_storage: Arc<dyn edgequake_storage::traits::GraphStorage>,
        llm_provider: Arc<dyn edgequake_llm::traits::LLMProvider>,
        embedding_provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        query_engine: Arc<QueryEngine>,
        pipeline: Arc<Pipeline>,
        task_storage: SharedTaskStorage,
        task_queue: SharedTaskQueue,
        workspace_service: SharedWorkspaceService,
    ) -> Self {
        let auth_config = AuthConfig::default();
        let jwt_service = Arc::new(JwtService::new(auth_config.clone()));
        let password_service = Arc::new(PasswordService::new(auth_config.clone()));
        let rbac_service = Arc::new(RbacService::new());
        let conversation_service: SharedConversationService =
            Arc::new(InMemoryConversationService::new());

        Self {
            kv_storage,
            vector_storage,
            graph_storage,
            llm_provider,
            embedding_provider,
            query_engine,
            pipeline,
            task_storage,
            task_queue,
            pipeline_state: PipelineState::new(),
            progress_broadcaster: ProgressBroadcaster::default(),
            workspace_service,
            conversation_service,
            config: AppConfig::default(),
            auth_config,
            jwt_service,
            password_service,
            rbac_service,
            cache_manager: CacheManager::with_defaults(),
            rate_limiter: RateLimiter::new(TokenBucketConfig::default()),
            storage_mode: StorageMode::Memory, // Default to memory for generic constructor
            #[cfg(feature = "postgres")]
            pg_pool: None,
        }
    }

    /// Create a new application state with memory storage.
    pub fn new_memory(llm_api_key: impl Into<String>) -> Self {
        let kv_storage = Arc::new(MemoryKVStorage::new("default"));
        let vector_storage = Arc::new(MemoryVectorStorage::new("default", 1536));
        let graph_storage = Arc::new(MemoryGraphStorage::new("default"));
        let llm_provider = Arc::new(OpenAIProvider::new(llm_api_key));

        // Create workspace service with default tenant
        let workspace_service: SharedWorkspaceService = Arc::new(InMemoryWorkspaceService::new());

        // Create conversation service
        let conversation_service: SharedConversationService =
            Arc::new(InMemoryConversationService::new());

        // Create pipeline with LLM and embedding providers configured
        use edgequake_pipeline::LLMExtractor;
        let extractor = Arc::new(LLMExtractor::new(
            Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>
        ));
        let pipeline = Arc::new(
            Pipeline::default_pipeline()
                .with_extractor(extractor)
                .with_embedding_provider(
                    Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>
                ),
        );

        // Create task infrastructure
        let task_storage = Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new());
        let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));

        // Create query engine
        let query_engine = Arc::new(QueryEngine::new(
            QueryEngineConfig::default(),
            Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        ));

        // Create auth services
        let auth_config = AuthConfig::default();
        let jwt_service = Arc::new(JwtService::new(auth_config.clone()));
        let password_service = Arc::new(PasswordService::new(auth_config.clone()));
        let rbac_service = Arc::new(RbacService::new());

        Self {
            kv_storage: Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector_storage)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            graph_storage: Arc::clone(&graph_storage)
                as Arc<dyn edgequake_storage::traits::GraphStorage>,
            llm_provider: Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            embedding_provider: Arc::clone(&llm_provider)
                as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            query_engine,
            pipeline,
            task_storage,
            task_queue,
            pipeline_state: PipelineState::new(),
            progress_broadcaster: ProgressBroadcaster::default(),
            workspace_service,
            conversation_service,
            config: AppConfig::default(),
            auth_config,
            jwt_service,
            password_service,
            rbac_service,
            cache_manager: CacheManager::with_defaults(),
            rate_limiter: RateLimiter::new(TokenBucketConfig::default()),
            storage_mode: StorageMode::Memory,
            #[cfg(feature = "postgres")]
            pg_pool: None,
        }
    }

    /// Create a minimal state for testing.
    pub fn test_state() -> Self {
        use edgequake_llm::MockProvider;

        let mock_provider = Arc::new(MockProvider::new());
        let kv_storage = Arc::new(MemoryKVStorage::new("test"));
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 1536)); // Match MockProvider dimension
        let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
        let pipeline = Arc::new(Pipeline::default_pipeline());

        // Create workspace service
        let workspace_service: SharedWorkspaceService = Arc::new(InMemoryWorkspaceService::new());

        // Create conversation service
        let conversation_service: SharedConversationService =
            Arc::new(InMemoryConversationService::new());

        // Create task infrastructure
        let task_storage = Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new());
        let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));

        let query_config = QueryEngineConfig::default();
        let query_engine = Arc::new(QueryEngine::new(
            query_config,
            Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        ));

        // Create auth services with test configuration
        let auth_config = AuthConfig::default();
        let jwt_service = Arc::new(JwtService::new(auth_config.clone()));
        let password_service = Arc::new(PasswordService::new(auth_config.clone()));
        let rbac_service = Arc::new(RbacService::new());

        Self {
            kv_storage: Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector_storage)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            graph_storage: Arc::clone(&graph_storage)
                as Arc<dyn edgequake_storage::traits::GraphStorage>,
            llm_provider: Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            embedding_provider: Arc::clone(&mock_provider)
                as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            query_engine,
            pipeline,
            task_storage,
            task_queue,
            pipeline_state: PipelineState::new(),
            progress_broadcaster: ProgressBroadcaster::default(),
            workspace_service,
            conversation_service,
            config: AppConfig::default(),
            auth_config,
            jwt_service,
            password_service,
            rbac_service,
            cache_manager: CacheManager::with_defaults(),
            rate_limiter: RateLimiter::new(TokenBucketConfig::strict(100, 60)), // Strict limits for testing
            storage_mode: StorageMode::Memory,
            #[cfg(feature = "postgres")]
            pg_pool: None,
        }
    }

    /// Create a new application state with PostgreSQL storage.
    #[cfg(feature = "postgres")]
    pub async fn new_postgres(
        database_url: impl Into<String>,
        llm_api_key: impl Into<String>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let database_url = database_url.into();
        let llm_api_key = llm_api_key.into();

        // Parse database URL to create PostgreSQL configuration
        // Format: postgresql://username:password@host:port/database
        let url = url::Url::parse(&database_url)?;

        let host = url
            .host_str()
            .ok_or("Missing host in DATABASE_URL")?
            .to_string();
        let port = url.port().unwrap_or(5432);
        let database = url.path().trim_start_matches('/').to_string();
        let user = url.username().to_string();
        let password = url.password().unwrap_or("").to_string();

        // Create PostgreSQL configuration
        let pg_config = edgequake_storage::adapters::postgres::PostgresConfig::new(
            host, port, database, user, password,
        )
        .with_namespace("default")
        .with_max_connections(10);

        // Create PostgreSQL connection pool for conversation service
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await?;

        // Ensure required extensions are available (these should be created in Docker init.sql,
        // but we check and log if they're missing)
        tracing::info!("Checking required PostgreSQL extensions...");

        // Check if essential extensions exist (don't create them - that requires superuser)
        let extensions_result = sqlx::query_scalar::<_, String>(
            "SELECT extname FROM pg_extension WHERE extname IN ('vector', 'uuid-ossp')",
        )
        .fetch_all(&pool)
        .await;

        match extensions_result {
            Ok(exts) => {
                if exts.contains(&"vector".to_string()) {
                    tracing::info!("✓ pgvector extension available");
                } else {
                    tracing::warn!("⚠ pgvector extension not found - vector search may not work");
                }
                if exts.contains(&"uuid-ossp".to_string()) {
                    tracing::info!("✓ uuid-ossp extension available");
                } else {
                    tracing::warn!("⚠ uuid-ossp extension not found");
                }
            }
            Err(e) => {
                tracing::warn!("Could not check extensions: {}", e);
            }
        }

        // Run migrations from the workspace root migrations directory
        // SQLx migrations will create all required tables automatically
        tracing::info!("Running database migrations...");
        sqlx::migrate!("../../migrations").run(&pool).await?;
        tracing::info!("✓ Database migrations completed successfully");

        // Create PostgreSQL-backed storages
        let kv_storage = Arc::new(PostgresKVStorage::new(pg_config.clone()));
        let vector_storage = Arc::new(PgVectorStorage::with_dimension(pg_config.clone(), 1536));
        let graph_storage = Arc::new(PostgresAGEGraphStorage::new(pg_config.clone()));

        // Initialize storage backends to establish connections
        kv_storage.initialize().await?;
        vector_storage.initialize().await?;
        graph_storage.initialize().await?;

        tracing::info!("PostgreSQL storage backends initialized successfully");

        // Create LLM provider
        let llm_provider = Arc::new(OpenAIProvider::new(llm_api_key));

        // Create workspace service (still in-memory for now)
        let workspace_service: SharedWorkspaceService = Arc::new(InMemoryWorkspaceService::new());

        // Create PostgreSQL-backed conversation service
        let conversation_service: SharedConversationService =
            Arc::new(PostgresConversationService::new(pool.clone()));

        // Create pipeline with LLM and embedding providers configured
        use edgequake_pipeline::LLMExtractor;
        let extractor = Arc::new(LLMExtractor::new(
            Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>
        ));
        let pipeline = Arc::new(
            Pipeline::default_pipeline()
                .with_extractor(extractor)
                .with_embedding_provider(
                    Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>
                ),
        );

        // Create task infrastructure
        let task_storage = Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new());
        let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));

        // Create query engine
        let query_engine = Arc::new(QueryEngine::new(
            QueryEngineConfig::default(),
            Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        ));

        // Create auth services
        let auth_config = AuthConfig::default();
        let jwt_service = Arc::new(JwtService::new(auth_config.clone()));
        let password_service = Arc::new(PasswordService::new(auth_config.clone()));
        let rbac_service = Arc::new(RbacService::new());

        Ok(Self {
            kv_storage: Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector_storage)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            graph_storage: Arc::clone(&graph_storage)
                as Arc<dyn edgequake_storage::traits::GraphStorage>,
            llm_provider: Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            embedding_provider: Arc::clone(&llm_provider)
                as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            query_engine,
            pipeline,
            task_storage,
            task_queue,
            pipeline_state: PipelineState::new(),
            progress_broadcaster: ProgressBroadcaster::default(),
            workspace_service,
            conversation_service,
            config: AppConfig::default(),
            auth_config,
            jwt_service,
            password_service,
            rbac_service,
            cache_manager: CacheManager::with_defaults(),
            rate_limiter: RateLimiter::new(TokenBucketConfig::default()),
            storage_mode: StorageMode::PostgreSQL,
            pg_pool: Some(pool),
        })
    }

    /// Initialize default tenant and workspace for non-authenticated mode.
    /// This ensures that the system is usable without authentication.
    pub async fn initialize_defaults(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use edgequake_core::{CreateWorkspaceRequest, Tenant, TenantPlan};

        // Define default user ID for anonymous/unauthenticated access
        let default_user_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001")
            .expect("Invalid default user UUID");

        // When using PostgreSQL, first check if default tenant already exists in the database
        // and reuse it to maintain consistency across restarts
        #[cfg(feature = "postgres")]
        if let Some(ref pool) = self.pg_pool {
            // Ensure default user exists in PostgreSQL (for FK constraints)
            sqlx::query(
                r#"
                INSERT INTO users (user_id, username, email, password_hash, role, is_active, created_at, updated_at)
                VALUES ($1, 'default_user', 'default@edgequake.local', 'not_a_real_hash', 'user', TRUE, NOW(), NOW())
                ON CONFLICT (user_id) DO NOTHING
                "#,
            )
            .bind(default_user_id)
            .execute(pool)
            .await?;

            tracing::debug!(
                user_id = %default_user_id,
                "Ensured default user exists in PostgreSQL"
            );

            // Check if default tenant exists in PostgreSQL
            let existing_tenant: Option<(uuid::Uuid, String)> = sqlx::query_as(
                "SELECT tenant_id, name FROM tenants WHERE slug = 'default' LIMIT 1",
            )
            .fetch_optional(pool)
            .await?;

            if let Some((pg_tenant_id, pg_tenant_name)) = existing_tenant {
                // Get existing workspace too
                let existing_workspace: Option<(uuid::Uuid, String)> = sqlx::query_as(
                    "SELECT workspace_id, name FROM workspaces WHERE tenant_id = $1 AND slug = 'default' LIMIT 1"
                )
                .bind(pg_tenant_id)
                .fetch_optional(pool)
                .await?;

                // CRITICAL: Sync PostgreSQL tenant/workspace to InMemoryWorkspaceService
                // This ensures frontend-created tenants use the same IDs as PostgreSQL
                // Without this, conversations created via the frontend would fail FK constraints

                // Create tenant with PostgreSQL's existing ID
                let mut tenant = Tenant::new("Default", "default")
                    .with_plan(TenantPlan::Pro)
                    .with_description("Default tenant for EdgeQuake");
                tenant.tenant_id = pg_tenant_id; // Use PostgreSQL's ID

                // Insert into InMemoryWorkspaceService (ignore if already exists)
                if self
                    .workspace_service
                    .get_tenant(pg_tenant_id)
                    .await?
                    .is_none()
                {
                    self.workspace_service.create_tenant(tenant).await?;
                    tracing::info!(
                        tenant_id = %pg_tenant_id,
                        name = %pg_tenant_name,
                        "Synced PostgreSQL tenant to InMemoryWorkspaceService"
                    );
                } else {
                    tracing::debug!(
                        tenant_id = %pg_tenant_id,
                        "Tenant already exists in InMemoryWorkspaceService"
                    );
                }

                // Sync workspace if it exists
                if let Some((ws_id, ws_name)) = existing_workspace {
                    if self.workspace_service.get_workspace(ws_id).await?.is_none() {
                        // Create workspace with PostgreSQL's existing ID
                        use edgequake_core::Workspace;
                        let mut workspace = Workspace::new(pg_tenant_id, &ws_name, "default")
                            .with_description("Default workspace for EdgeQuake");
                        workspace.workspace_id = ws_id; // Use PostgreSQL's ID

                        // Use insert_workspace to preserve the specific ID
                        self.workspace_service.insert_workspace(workspace).await?;

                        tracing::info!(
                            workspace_id = %ws_id,
                            tenant_id = %pg_tenant_id,
                            "Synced PostgreSQL workspace to InMemoryWorkspaceService"
                        );
                    } else {
                        tracing::debug!(
                            workspace_id = %ws_id,
                            "Workspace already exists in InMemoryWorkspaceService"
                        );
                    }
                }

                tracing::info!(
                    tenant_id = %pg_tenant_id,
                    name = %pg_tenant_name,
                    "Synced PostgreSQL defaults to memory"
                );

                return Ok(());
            }
        }

        // Check if default tenant already exists in memory
        let existing = self.workspace_service.list_tenants(10, 0).await?;

        if !existing.is_empty() {
            tracing::info!(
                "Found {} existing tenant(s), skipping default initialization",
                existing.len()
            );
            return Ok(());
        }

        // Create default tenant
        let default_tenant = Tenant::new("Default", "default")
            .with_plan(TenantPlan::Pro)
            .with_description("Default tenant for EdgeQuake");

        let tenant = self.workspace_service.create_tenant(default_tenant).await?;

        // When using PostgreSQL, also insert the tenant into the database
        // This is needed because ConversationService uses PostgreSQL with foreign key constraints
        #[cfg(feature = "postgres")]
        if let Some(ref pool) = self.pg_pool {
            sqlx::query(
                r#"
                INSERT INTO tenants (tenant_id, name, slug, description, plan, is_active, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, TRUE, NOW(), NOW())
                ON CONFLICT (tenant_id) DO NOTHING
                "#,
            )
            .bind(tenant.tenant_id)
            .bind(&tenant.name)
            .bind(&tenant.slug)
            .bind(&tenant.description)
            .bind(tenant.plan.to_string())
            .execute(pool)
            .await?;

            tracing::debug!(
                tenant_id = %tenant.tenant_id,
                "Inserted tenant into PostgreSQL database"
            );
        }

        tracing::info!(
            tenant_id = %tenant.tenant_id,
            "Created default tenant"
        );

        // Create default workspace within the tenant
        let workspace_request = CreateWorkspaceRequest {
            name: "Default Workspace".to_string(),
            slug: Some("default".to_string()),
            description: Some("Default knowledge base".to_string()),
            max_documents: Some(10000),
        };

        let workspace = self
            .workspace_service
            .create_workspace(tenant.tenant_id, workspace_request)
            .await?;

        // When using PostgreSQL, also insert the workspace into the database
        #[cfg(feature = "postgres")]
        if let Some(ref pool) = self.pg_pool {
            sqlx::query(
                r#"
                INSERT INTO workspaces (workspace_id, tenant_id, name, slug, description, is_active, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, TRUE, NOW(), NOW())
                ON CONFLICT (workspace_id) DO NOTHING
                "#,
            )
            .bind(workspace.workspace_id)
            .bind(tenant.tenant_id)
            .bind(&workspace.name)
            .bind(&workspace.slug)
            .bind(&workspace.description)
            .execute(pool)
            .await?;

            tracing::debug!(
                workspace_id = %workspace.workspace_id,
                "Inserted workspace into PostgreSQL database"
            );
        }

        tracing::info!(
            workspace_id = %workspace.workspace_id,
            tenant_id = %tenant.tenant_id,
            "Created default workspace"
        );

        Ok(())
    }
}
