//! Application state.

use std::sync::Arc;

use edgequake_auth::{AuthConfig, JwtService, PasswordService, RbacService};
use edgequake_core::{InMemoryWorkspaceService, WorkspaceService};
use edgequake_llm::OpenAIProvider;
use edgequake_pipeline::Pipeline;
use edgequake_query::{QueryEngine, QueryEngineConfig};
use edgequake_storage::adapters::memory::{
    MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
};
use edgequake_tasks::{PipelineState, SharedTaskQueue, SharedTaskStorage};

/// Type alias for the shared workspace service.
pub type SharedWorkspaceService = Arc<dyn WorkspaceService>;

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

    /// Workspace service for tenant/workspace management.
    pub workspace_service: SharedWorkspaceService,

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
            workspace_service,
            config: AppConfig::default(),
            auth_config,
            jwt_service,
            password_service,
            rbac_service,
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
            workspace_service,
            config: AppConfig::default(),
            auth_config,
            jwt_service,
            password_service,
            rbac_service,
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
            workspace_service,
            config: AppConfig::default(),
            auth_config,
            jwt_service,
            password_service,
            rbac_service,
        }
    }

    /// Initialize default tenant and workspace for non-authenticated mode.
    /// This ensures that the system is usable without authentication.
    pub async fn initialize_defaults(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use edgequake_core::{CreateWorkspaceRequest, Tenant, TenantPlan};

        // Check if default tenant already exists
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

        tracing::info!(
            workspace_id = %workspace.workspace_id,
            tenant_id = %tenant.tenant_id,
            "Created default workspace"
        );

        Ok(())
    }
}
