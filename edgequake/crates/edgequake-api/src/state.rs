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

use std::sync::Arc;

use crate::cache_manager::CacheManager;
use crate::handlers::ProgressBroadcaster;
use edgequake_auth::{AuthConfig, JwtService, PasswordService, RbacService};
use edgequake_core::{
    ConversationService, InMemoryConversationService, InMemoryWorkspaceService, WorkspaceService,
};
use edgequake_llm::ModelsConfig;
use edgequake_pipeline::Pipeline;
use edgequake_query::{QueryEngine, QueryEngineConfig, SOTAQueryConfig, SOTAQueryEngine};
use edgequake_rate_limiter::{RateLimitConfig as TokenBucketConfig, RateLimiter};
use edgequake_storage::adapters::memory::{
    MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage, MemoryWorkspaceVectorRegistry,
};
use edgequake_tasks::{PipelineState, SharedTaskQueue, SharedTaskStorage};
use serde::{Deserialize, Serialize};

/// Create the configured BM25 reranker.
///
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
use edgequake_core::ConversationServiceImpl;
#[cfg(feature = "postgres")]
use edgequake_core::WorkspaceServiceImpl;
#[cfg(feature = "postgres")]
use edgequake_storage::{
    GraphStorage, KVStorage, PgVectorStorage, PgWorkspaceVectorRegistry, PostgresAGEGraphStorage,
    PostgresKVStorage, VectorStorage,
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
}

/// Application configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Workspace/tenant ID.
    pub workspace_id: String,

    /// Maximum document size in bytes.
    /// SPEC-028: Updated to 50MB to support larger documents.
    pub max_document_size: usize,

    /// Maximum query length.
    pub max_query_length: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            workspace_id: "default".to_string(),
            // SPEC-028: 50MB document upload limit (was 10MB)
            // WHY: Support larger documents like research papers and reports
            max_document_size: 50 * 1024 * 1024, // 50 MB
            max_query_length: 10000,
        }
    }
}

impl AppState {
    /// Load path validation configuration from environment.
    ///
    /// SECURITY (OODA-248): Configures allowed directories for filesystem access.
    ///
    /// # Environment Variables
    ///
    /// - `ALLOWED_SCAN_PATHS`: Colon-separated list of allowed directories
    ///   Example: `/data/uploads:/home/user/documents`
    /// - `ALLOW_ANY_SCAN_PATH`: Set to "true" to allow any path (NOT RECOMMENDED)
    #[cfg(feature = "postgres")]
    fn load_path_validation_config() -> crate::path_validation::PathValidationConfig {
        use std::path::PathBuf;

        let allowed_paths: Vec<PathBuf> = std::env::var("ALLOWED_SCAN_PATHS")
            .unwrap_or_default()
            .split(':')
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect();

        let allow_any_path = std::env::var("ALLOW_ANY_SCAN_PATH")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        if allow_any_path {
            tracing::warn!(
                "⚠️ ALLOW_ANY_SCAN_PATH=true - Directory scanning is unrestricted! \
                 This is a security risk in production."
            );
        } else if allowed_paths.is_empty() {
            tracing::info!(
                "Path validation: No ALLOWED_SCAN_PATHS configured. \
                 scan_directory endpoint will reject all paths."
            );
        } else {
            tracing::info!(
                paths = ?allowed_paths,
                "Path validation: scan_directory restricted to allowed paths"
            );
        }

        crate::path_validation::PathValidationConfig {
            allowed_paths,
            allow_any_path,
            follow_symlinks: false, // Security: don't follow symlinks
            max_depth: 50,
        }
    }

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

        // If API key provided, set it in environment for factory to use
        if let Some(key) = llm_api_key {
            std::env::set_var("OPENAI_API_KEY", key.into());
        }

        // Use ProviderFactory for auto-detection
        let (llm_provider, embedding_provider) =
            ProviderFactory::from_env().expect("Failed to create LLM provider from environment");

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
        let auth_config = AuthConfig::default();
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
        }
    }

    /// Create a new application state with PostgreSQL storage.
    ///
    /// # Provider Selection
    ///
    /// LLM provider is automatically selected based on environment:
    /// - `EDGEQUAKE_LLM_PROVIDER=ollama|lmstudio|mock` - explicit selection
    /// - `OLLAMA_HOST` present → Ollama provider
    /// - `OPENAI_API_KEY` present → OpenAI provider
    /// - Default → Mock provider
    ///
    /// The `llm_api_key` parameter is kept for backward compatibility and will set `OPENAI_API_KEY`
    /// when provided. For Ollama/LM Studio, you can pass an empty string and use environment variables.
    #[cfg(feature = "postgres")]
    pub async fn new_postgres(
        database_url: impl Into<String>,
        llm_api_key: impl Into<String>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use edgequake_llm::ProviderFactory;

        let database_url = database_url.into();
        let llm_api_key = llm_api_key.into();

        // Set OPENAI_API_KEY for backward compatibility (factory will use it if OpenAI selected)
        if !llm_api_key.is_empty() {
            std::env::set_var("OPENAI_API_KEY", &llm_api_key);
        }

        // Create providers via factory (auto-detects from environment)
        let (llm_provider, embedding_provider) =
            ProviderFactory::from_env().expect("Failed to create LLM provider from environment");

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

        // CRITICAL: Set search_path to public BEFORE running migrations
        // This ensures _sqlx_migrations table is created in public schema, not user's default schema
        sqlx::query("SET search_path TO public")
            .execute(&pool)
            .await?;

        // Run migrations from the workspace root migrations directory
        // SQLx migrations will create all required tables automatically
        tracing::info!("Running database migrations...");
        sqlx::migrate!("../../migrations").run(&pool).await?;
        tracing::info!("✓ Database migrations completed successfully");

        // Auto-configure vector dimension from embedding provider
        let embedding_dim = embedding_provider.dimension();
        tracing::info!(
            "Using vector dimension {} from {} provider",
            embedding_dim,
            std::env::var("EDGEQUAKE_LLM_PROVIDER").unwrap_or_else(|_| "auto-detected".to_string())
        );

        // Create PostgreSQL-backed storages
        let kv_storage = Arc::new(PostgresKVStorage::new(pg_config.clone()));
        let vector_storage = Arc::new(PgVectorStorage::with_dimension(
            pg_config.clone(),
            embedding_dim,
        ));
        let graph_storage = Arc::new(PostgresAGEGraphStorage::new(pg_config.clone()));

        // OODA-228: Ensure default vector storage has correct dimension BEFORE initialize
        // WHY: If embedding provider changed (e.g., OpenAI 1536 → Ollama 768),
        // the existing table has the wrong dimension. We must recreate it.
        // This is the same logic used for workspace storage.
        let recreated = vector_storage.ensure_dimension(embedding_dim).await?;
        if recreated {
            tracing::warn!(
                dimension = embedding_dim,
                provider = embedding_provider.name(),
                "⚠️ Default vector table recreated due to dimension change (OODA-228). \
                 All existing vectors were cleared. Documents need to be re-embedded."
            );
        }

        // Initialize storage backends to establish connections
        kv_storage.initialize().await?;
        vector_storage.initialize().await?;
        graph_storage.initialize().await?;

        tracing::info!("PostgreSQL storage backends initialized successfully");

        // Log provider and dimension configuration for debugging
        tracing::info!(
            provider = embedding_provider.name(),
            dimension = embedding_provider.dimension(),
            storage_type = "postgres",
            namespace = "default",
            recreated = recreated,
            "Vector storage validated successfully"
        );

        // Create workspace service for full persistence
        let workspace_service_impl = WorkspaceServiceImpl::new(pool.clone());

        // Ensure default tenant and workspace exist (critical for non-authenticated mode)
        workspace_service_impl.ensure_defaults().await?;
        tracing::info!("Default tenant and workspace ensured in PostgreSQL");

        let workspace_service: SharedWorkspaceService = Arc::new(workspace_service_impl);

        // Create conversation service
        let conversation_service: SharedConversationService =
            Arc::new(ConversationServiceImpl::new(pool.clone()));

        // Create pipeline with LLM and embedding providers configured
        use edgequake_pipeline::LLMExtractor;
        let extractor = Arc::new(LLMExtractor::new(
            Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>
        ));
        let pipeline = Arc::new(
            Pipeline::default_pipeline()
                .with_extractor(extractor)
                .with_embedding_provider(Arc::clone(&embedding_provider)),
        );

        // Create task infrastructure (OODA-06: Use PostgreSQL for task persistence)
        // WHY: Tasks must persist across backend restarts so cancel/retry work correctly.
        // Previous bug: MemoryTaskStorage was used, causing tasks to be lost on restart.
        let task_storage: SharedTaskStorage = Arc::new(
            edgequake_tasks::postgres::PostgresTaskStorage::new(pool.clone()),
        );
        let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));
        tracing::info!("✓ Task storage: PostgreSQL (persistent across restarts)");

        // Create legacy query engine (for backward compatibility)
        let query_engine = Arc::new(QueryEngine::new(
            QueryEngineConfig::default(),
            Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            Arc::clone(&embedding_provider),
            Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        ));

        // Create SOTA query engine with LightRAG-style enhancements
        let reranker = create_bm25_reranker();
        let sota_engine = Arc::new(
            SOTAQueryEngine::new(
                SOTAQueryConfig::default(),
                Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
                Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
                Arc::clone(&embedding_provider),
                Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            )
            .with_reranker(reranker),
        );

        // Create workspace vector registry for per-workspace dimensions
        let vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> =
            Arc::new(PgWorkspaceVectorRegistry::new(
                pg_config,
                Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
                embedding_dim,
            ));

        // Create auth services
        let auth_config = AuthConfig::default();
        let jwt_service = Arc::new(JwtService::new(auth_config.clone()));
        let password_service = Arc::new(PasswordService::new(auth_config.clone()));
        let rbac_service = Arc::new(RbacService::new());

        // Create PDF storage (SPEC-007) - uses the connection pool
        let pdf_storage: Arc<dyn edgequake_storage::PdfDocumentStorage> =
            Arc::new(edgequake_storage::PostgresPdfStorage::new(pool.clone()));

        Ok(Self {
            kv_storage: Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector_storage)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            vector_registry,
            graph_storage: Arc::clone(&graph_storage)
                as Arc<dyn edgequake_storage::traits::GraphStorage>,
            llm_provider: Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
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
            storage_mode: StorageMode::PostgreSQL,
            models_config: Arc::new(
                ModelsConfig::load().unwrap_or_else(|_| ModelsConfig::builtin_defaults()),
            ),
            pg_pool: Some(pool),
            pdf_storage: Some(pdf_storage),
            start_time: std::time::Instant::now(),
            // SECURITY (OODA-248): PostgreSQL mode defaults to secure config.
            // Administrators should configure ALLOWED_SCAN_PATHS environment variable.
            path_validation_config: Self::load_path_validation_config(),
        })
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
        use edgequake_llm::ProviderFactory;
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
                let llm_provider =
                    ProviderFactory::create_safe_llm_provider(&ws.llm_provider, &ws.llm_model);

                // Try to create workspace-specific embedding provider with safety limits
                let embedding_provider = ProviderFactory::create_safe_embedding_provider(
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
