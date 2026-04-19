//! In-memory storage constructors for `AppState`.
//!
//! Provides factory methods that wire up memory-backed adapters for development,
//! testing, and lightweight deployments.

use std::sync::Arc;

use edgequake_auth::{AuthConfig, JwtService, PasswordService, RbacService};
use edgequake_core::env::apply_model_env_aliases;
use edgequake_core::{InMemoryConversationService, InMemoryWorkspaceService};
use edgequake_llm::ModelsConfig;
use edgequake_pipeline::Pipeline;
use edgequake_query::{QueryEngine, QueryEngineConfig, SOTAQueryConfig, SOTAQueryEngine};
use edgequake_rate_limiter::{RateLimitConfig as TokenBucketConfig, RateLimiter};
use edgequake_storage::adapters::memory::{
    MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage, MemoryWorkspaceVectorRegistry,
};
use edgequake_tasks::PipelineState;

use super::config::{AppConfig, SharedConversationService, SharedWorkspaceService, StorageMode};
use super::{create_bm25_reranker, AppState};
use crate::cache_manager::CacheManager;
use crate::handlers::ProgressBroadcaster;

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
        query_engine: Arc<QueryEngine>,
        sota_engine: Arc<SOTAQueryEngine>,
        pipeline: Arc<Pipeline>,
        task_storage: edgequake_tasks::SharedTaskStorage,
        task_queue: edgequake_tasks::SharedTaskQueue,
        workspace_service: SharedWorkspaceService,
    ) -> Self {
        let auth_config = AuthConfig::from_env();
        let jwt_service = Arc::new(JwtService::new(auth_config.clone()));
        let password_service = Arc::new(PasswordService::new(auth_config.clone()));
        let rbac_service = Arc::new(RbacService::new());
        let conversation_service: SharedConversationService =
            Arc::new(InMemoryConversationService::new());

        Self {
            kv_storage,
            vector_storage,
            vector_registry,
            graph_storage,
            llm_provider,
            embedding_provider,
            query_engine,
            sota_engine,
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
            models_config: Arc::new(
                ModelsConfig::load().unwrap_or_else(|_| ModelsConfig::builtin_defaults()),
            ),
            #[cfg(feature = "postgres")]
            pg_pool: None,
            #[cfg(feature = "postgres")]
            pdf_storage: None,
            start_time: std::time::Instant::now(),
            // SECURITY (OODA-248): Default to secure config (no paths allowed).
            // Production deployments should configure allowed_paths.
            path_validation_config: crate::path_validation::PathValidationConfig::default(),
            cancellation_registry: edgequake_tasks::CancellationRegistry::new(),
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
        let conversation_service: SharedConversationService =
            Arc::new(InMemoryConversationService::new());

        // Create pipeline with LLM and embedding providers configured
        use edgequake_pipeline::LLMExtractor;
        let extractor = Arc::new(LLMExtractor::new(Arc::clone(&llm_provider)));
        let pipeline = Arc::new(
            Pipeline::default_pipeline()
                .with_extractor(extractor)
                .with_embedding_provider(Arc::clone(&embedding_provider)),
        );

        // Create task infrastructure
        let task_storage = Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new());
        let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));

        // Create legacy query engine (for backward compatibility)
        let query_engine = Arc::new(QueryEngine::new(
            QueryEngineConfig::default(),
            Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            Arc::clone(&embedding_provider),
            Arc::clone(&llm_provider),
        ));

        // Create SOTA query engine with LightRAG-style enhancements
        let reranker = create_bm25_reranker();
        let sota_engine = Arc::new(
            SOTAQueryEngine::new(
                SOTAQueryConfig::default(),
                Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
                Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
                Arc::clone(&embedding_provider),
                Arc::clone(&llm_provider),
            )
            .with_reranker(reranker),
        );

        // Create workspace vector registry for per-workspace dimensions
        let vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> =
            Arc::new(MemoryWorkspaceVectorRegistry::new(
                Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            ));

        // Create auth services
        let auth_config = AuthConfig::from_env();
        let jwt_service = Arc::new(JwtService::new(auth_config.clone()));
        let password_service = Arc::new(PasswordService::new(auth_config.clone()));
        let rbac_service = Arc::new(RbacService::new());

        Self {
            kv_storage: Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector_storage)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            vector_registry,
            graph_storage: Arc::clone(&graph_storage)
                as Arc<dyn edgequake_storage::traits::GraphStorage>,
            llm_provider: Arc::clone(&llm_provider),
            embedding_provider: Arc::clone(&embedding_provider),
            query_engine,
            sota_engine,
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
            models_config: Arc::new(
                ModelsConfig::load().unwrap_or_else(|_| ModelsConfig::builtin_defaults()),
            ),
            #[cfg(feature = "postgres")]
            pg_pool: None,
            // PDF storage not available in memory mode
            #[cfg(feature = "postgres")]
            pdf_storage: None,
            start_time: std::time::Instant::now(),
            // SECURITY (OODA-248): Memory mode uses permissive config for dev/testing.
            // Production should use PostgreSQL mode with explicit allowed_paths.
            path_validation_config: crate::path_validation::PathValidationConfig {
                allow_any_path: true, // Permissive for memory/dev mode
                ..Default::default()
            },
            cancellation_registry: edgequake_tasks::CancellationRegistry::new(),
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

        // Create legacy query engine (for backward compatibility)
        let query_config = QueryEngineConfig::default();
        let query_engine = Arc::new(QueryEngine::new(
            query_config,
            Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        ));

        // Create SOTA query engine with mock keywords for testing
        let sota_engine = Arc::new(SOTAQueryEngine::with_mock_keywords(
            SOTAQueryConfig::default(),
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

        // Create workspace vector registry for per-workspace dimensions
        let vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> =
            Arc::new(MemoryWorkspaceVectorRegistry::new(
                Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            ));

        Self {
            kv_storage: Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector_storage)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            vector_registry,
            graph_storage: Arc::clone(&graph_storage)
                as Arc<dyn edgequake_storage::traits::GraphStorage>,
            llm_provider: Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            embedding_provider: Arc::clone(&mock_provider)
                as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            query_engine,
            sota_engine,
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
            models_config: Arc::new(ModelsConfig::builtin_defaults()), // Use builtins for testing
            #[cfg(feature = "postgres")]
            pg_pool: None,
            // PDF storage not available in test mode
            #[cfg(feature = "postgres")]
            pdf_storage: None,
            start_time: std::time::Instant::now(),
            // SECURITY (OODA-248): Test state is permissive for testing
            path_validation_config: crate::path_validation::PathValidationConfig {
                allow_any_path: true,
                ..Default::default()
            },
            cancellation_registry: edgequake_tasks::CancellationRegistry::new(),
        }
    }
}
