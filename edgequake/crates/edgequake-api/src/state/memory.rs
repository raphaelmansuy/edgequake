//! In-memory storage constructors for `AppState`.
//!
//! Provides factory methods that wire up memory-backed adapters for development,
//! testing, and lightweight deployments.

use std::sync::Arc;

use edgequake_auth::AuthConfig;
use edgequake_core::env::apply_model_env_aliases;
#[cfg(feature = "postgres")]
use edgequake_core::ConversationServiceImpl;
#[cfg(not(feature = "postgres"))]
use edgequake_core::InMemoryConversationService;
use edgequake_core::InMemoryWorkspaceService;
use edgequake_llm::ModelsConfig;
use edgequake_pipeline::Pipeline;
use edgequake_query::{QueryEngine, QueryEngineConfig};
use edgequake_rate_limiter::{RateLimitConfig as TokenBucketConfig, RateLimiter};
#[cfg(feature = "postgres")]
use edgequake_storage::adapters::memory::{MemoryConversationStorage, MemoryPdfStorage};
use edgequake_storage::adapters::memory::{
    MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage, MemoryWorkspaceVectorRegistry,
};
#[cfg(feature = "postgres")]
use edgequake_storage::ConversationStorage;

use super::config::{AppConfig, SharedConversationService, SharedWorkspaceService, StorageMode};
use super::{ApiSecurityConfig, AppState, AuthRuntime, QueryRuntime, StorageRuntime, TaskRuntime};
use crate::cache_manager::CacheManager;

#[cfg(feature = "postgres")]
fn memory_pdf_storage() -> Option<Arc<dyn edgequake_storage::PdfDocumentStorage>> {
    Some(Arc::new(MemoryPdfStorage::new()))
}

/// Memory-mode conversation service: storage trait adapter when postgres feature is enabled.
fn memory_conversation_service() -> SharedConversationService {
    #[cfg(feature = "postgres")]
    {
        let storage: Arc<dyn ConversationStorage> = Arc::new(MemoryConversationStorage::new());
        Arc::new(ConversationServiceImpl::from_storage(storage))
    }
    #[cfg(not(feature = "postgres"))]
    {
        Arc::new(InMemoryConversationService::new())
    }
}

impl AppState {
    /// Create a new application state.
    ///
    /// WHY: This constructor takes many arguments because AppState is the central
    /// application container that wires together all major subsystems (storage, LLM,
    /// query engines, pipeline, auth). Grouping these into intermediate structs would
    /// add complexity without improving API clarity.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kv_storage: Arc<dyn edgequake_storage::traits::KVStorage>,
        vector_storage: Arc<dyn edgequake_storage::traits::VectorStorage>,
        vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry>,
        graph_storage: Arc<dyn edgequake_storage::traits::GraphStorage>,
        llm_provider: Arc<dyn edgequake_llm::traits::LLMProvider>,
        embedding_provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        engine_impl: Arc<QueryEngine>,
        pipeline: Arc<Pipeline>,
        task_storage: edgequake_tasks::SharedTaskStorage,
        task_queue: edgequake_tasks::SharedTaskQueue,
        workspace_service: SharedWorkspaceService,
    ) -> Self {
        let conversation_service = memory_conversation_service();
        let (resource_guard, graph_materialize, pdf_vision) =
            super::resource_runtime::build_resource_runtime();

        Self {
            storage: StorageRuntime {
                kv_storage,
                vector_storage,
                vector_registry,
                graph_storage,
                auth_memory: Arc::new(crate::services::auth_memory_store::AuthMemoryStore::new()),
                #[cfg(feature = "postgres")]
                pdf_storage: memory_pdf_storage(),
                mode: StorageMode::Memory,
            },
            query: QueryRuntime {
                llm_provider,
                vision_llm_provider: None,
                embedding_provider,
                engine_impl,
                pipeline,
                models_config: super::bundled_models::bundled_models_config(),
            },
            auth: AuthRuntime::from_env(),
            tasks: TaskRuntime::new(task_storage, task_queue),
            workspace_service,
            conversation_service,
            config: AppConfig::default(),
            cache_manager: CacheManager::with_defaults(),
            rate_limiter: RateLimiter::new(TokenBucketConfig::default()),
            #[cfg(feature = "postgres")]
            pg_pool: None,
            start_time: std::time::Instant::now(),
            path_validation_config: crate::path_validation::PathValidationConfig::default(),
            audit_logger: None,
            resource_guard,
            graph_materialize,
            pdf_vision,
            #[cfg(feature = "postgres")]
            migration_bootstrap: None,
            #[cfg(feature = "postgres")]
            postgres_capabilities: None,
            security: ApiSecurityConfig::from_env(),
        }
    }

    /// Create a new application state with memory storage.
    ///
    /// # Arguments
    ///
    /// * `llm_api_key` - Optional API key override. If provided, sets OPENAI_API_KEY
    ///   environment variable. Otherwise uses ProviderFactory auto-detection.
    ///
    /// # Provider Selection
    ///
    /// Uses ProviderFactory::from_env() which auto-detects based on:
    /// 1. EDGEQUAKE_LLM_PROVIDER environment variable
    /// 2. OLLAMA_HOST or OLLAMA_MODEL (selects Ollama)
    /// 3. OPENAI_API_KEY (selects OpenAI)
    /// 4. Fallback to Mock provider
    pub fn new_memory(llm_api_key: Option<impl Into<String>>) -> Self {
        use edgequake_llm::ProviderFactory;

        apply_model_env_aliases();

        // If API key provided, set it in environment for factory to use
        if let Some(key) = llm_api_key {
            std::env::set_var("OPENAI_API_KEY", key.into());
        }

        // FIX #166: Recognize EDGEQUAKE_CHAT_* as aliases for the standard LLM env vars.
        super::provider_setup::apply_chat_env_aliases();
        super::provider_setup::normalize_local_provider_hosts_for_docker();

        // Use ProviderFactory for auto-detection
        let (llm_provider, embedding_provider) =
            ProviderFactory::from_env().expect("Failed to create LLM provider from environment");

        // Allow a dedicated embedding provider / host to override the default
        // (OLLAMA_EMBEDDING_HOST, EDGEQUAKE_EMBEDDING_PROVIDER, etc.)
        let embedding_provider =
            super::provider_setup::resolve_embedding_provider(embedding_provider);

        // Get embedding dimension from provider for vector storage
        let embedding_dim = embedding_provider.dimension();

        let kv_storage = Arc::new(MemoryKVStorage::new("default"));
        let vector_storage = Arc::new(MemoryVectorStorage::new("default", embedding_dim));
        let graph_storage = Arc::new(MemoryGraphStorage::new("default"));

        // Log provider and dimension configuration for debugging
        tracing::info!(
            provider = embedding_provider.name(),
            dimension = embedding_dim,
            storage_type = "memory",
            namespace = "default",
            "Vector storage initialized"
        );

        // Create workspace service with default tenant
        let workspace_service: SharedWorkspaceService = Arc::new(InMemoryWorkspaceService::new());

        // Create conversation service
        let conversation_service = memory_conversation_service();

        let pipeline = super::query_bootstrap::build_ingestion_pipeline(
            Arc::clone(&llm_provider),
            Arc::clone(&embedding_provider),
        );

        // Create task infrastructure
        let task_storage = Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new());
        let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));

        let engine_impl = super::query_bootstrap::build_production_query_engine(
            Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            Arc::clone(&embedding_provider),
            Arc::clone(&llm_provider),
            Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
        );

        // Create workspace vector registry for per-workspace dimensions
        let vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> =
            Arc::new(MemoryWorkspaceVectorRegistry::new(
                Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            ));

        // Create auth services
        let auth = AuthRuntime::from_env();
        let (resource_guard, graph_materialize, pdf_vision) =
            super::resource_runtime::build_resource_runtime();

        Self {
            storage: StorageRuntime {
                kv_storage: Arc::clone(&kv_storage)
                    as Arc<dyn edgequake_storage::traits::KVStorage>,
                vector_storage: Arc::clone(&vector_storage)
                    as Arc<dyn edgequake_storage::traits::VectorStorage>,
                vector_registry,
                graph_storage: Arc::clone(&graph_storage)
                    as Arc<dyn edgequake_storage::traits::GraphStorage>,
                auth_memory: Arc::new(crate::services::auth_memory_store::AuthMemoryStore::new()),
                #[cfg(feature = "postgres")]
                pdf_storage: memory_pdf_storage(),
                mode: StorageMode::Memory,
            },
            query: QueryRuntime {
                llm_provider: Arc::clone(&llm_provider),
                vision_llm_provider: None,
                embedding_provider: Arc::clone(&embedding_provider),
                engine_impl,
                pipeline,
                models_config: super::bundled_models::bundled_models_config(),
            },
            auth,
            tasks: TaskRuntime::new(task_storage, task_queue),
            workspace_service,
            conversation_service,
            config: AppConfig::default(),
            cache_manager: CacheManager::with_defaults(),
            rate_limiter: RateLimiter::new(TokenBucketConfig::default()),
            #[cfg(feature = "postgres")]
            pg_pool: None,
            start_time: std::time::Instant::now(),
            path_validation_config: crate::path_validation::PathValidationConfig {
                allow_any_path: true,
                ..Default::default()
            },
            audit_logger: None,
            resource_guard,
            graph_materialize,
            pdf_vision,
            #[cfg(feature = "postgres")]
            migration_bootstrap: None,
            #[cfg(feature = "postgres")]
            postgres_capabilities: None,
            security: ApiSecurityConfig::default(),
        }
    }

    /// Create a minimal state for testing.
    pub fn test_state() -> Self {
        use edgequake_llm::MockProvider;

        Self::build_test_state(Arc::new(MockProvider::new()))
    }

    /// Build test state from a pre-configured mock (DRY — worker E2E seeds extraction JSON).
    pub fn build_test_state(mock_provider: Arc<edgequake_llm::MockProvider>) -> Self {
        let kv_storage = Arc::new(MemoryKVStorage::new("test"));
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 1536)); // Match MockProvider dimension
        let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
        let pipeline = super::query_bootstrap::build_ingestion_pipeline(
            Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        );

        // Create workspace service
        let workspace_service: SharedWorkspaceService = Arc::new(InMemoryWorkspaceService::new());

        // Create conversation service
        let conversation_service = memory_conversation_service();

        // Create task infrastructure
        let task_storage = Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new());
        let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));

        // Create SOTA query engine with mock keywords for testing
        let engine_impl = Arc::new(
            QueryEngine::with_mock_keywords(
                QueryEngineConfig::default(),
                Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
                Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
                Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
                Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            )
            .with_kv_storage(
                Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>
            )
            .with_embedding_cache()
            .with_result_cache(),
        );

        // Create workspace vector registry for per-workspace dimensions
        let vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> =
            Arc::new(MemoryWorkspaceVectorRegistry::new(
                Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            ));

        // Test harness: simulate EDGEQUAKE_DEV_MODE (open API without auth middleware).
        let auth = AuthRuntime::new(AuthConfig {
            auth_enabled: false,
            dev_mode: true,
            ..AuthConfig::default()
        });
        let (resource_guard, graph_materialize, pdf_vision) =
            super::resource_runtime::build_resource_runtime();

        Self {
            storage: StorageRuntime {
                kv_storage: Arc::clone(&kv_storage)
                    as Arc<dyn edgequake_storage::traits::KVStorage>,
                vector_storage: Arc::clone(&vector_storage)
                    as Arc<dyn edgequake_storage::traits::VectorStorage>,
                vector_registry,
                graph_storage: Arc::clone(&graph_storage)
                    as Arc<dyn edgequake_storage::traits::GraphStorage>,
                auth_memory: Arc::new(crate::services::auth_memory_store::AuthMemoryStore::new()),
                #[cfg(feature = "postgres")]
                pdf_storage: memory_pdf_storage(),
                mode: StorageMode::Memory,
            },
            query: QueryRuntime {
                llm_provider: Arc::clone(&mock_provider)
                    as Arc<dyn edgequake_llm::traits::LLMProvider>,
                vision_llm_provider: None,
                embedding_provider: Arc::clone(&mock_provider)
                    as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
                engine_impl,
                pipeline,
                models_config: Arc::new(ModelsConfig::builtin_defaults()),
            },
            auth,
            tasks: TaskRuntime::new(task_storage, task_queue),
            workspace_service,
            conversation_service,
            config: AppConfig::default(),
            cache_manager: CacheManager::with_defaults(),
            rate_limiter: RateLimiter::new(TokenBucketConfig::strict(100, 60)),
            #[cfg(feature = "postgres")]
            pg_pool: None,
            start_time: std::time::Instant::now(),
            path_validation_config: crate::path_validation::PathValidationConfig {
                allow_any_path: true,
                ..Default::default()
            },
            audit_logger: None,
            resource_guard,
            graph_materialize,
            pdf_vision,
            #[cfg(feature = "postgres")]
            migration_bootstrap: None,
            #[cfg(feature = "postgres")]
            postgres_capabilities: None,
            security: ApiSecurityConfig::default(),
        }
    }

    /// Test state with PostgreSQL pool — auth/session use PG SSOT (SPEC-027 phase 41).
    #[cfg(feature = "postgres")]
    pub fn test_state_with_pg_pool(pool: sqlx::PgPool) -> Self {
        let mut state = Self::test_state();
        state.pg_pool = Some(pool);
        state
    }
}
