//! PostgreSQL storage constructors for `AppState`.
//!
//! Provides the `new_postgres()` factory that wires up persistent PostgreSQL-backed
//! adapters including pgvector, Apache AGE, and conversation/workspace services.

use std::sync::Arc;

use super::config::{AppConfig, SharedConversationService, SharedWorkspaceService, StorageMode};
use super::{
    create_bm25_reranker, AppState, AuthRuntime, QueryRuntime, StorageRuntime, TaskRuntime,
};
use crate::cache_manager::CacheManager;
use edgequake_audit::AuditLogger;
use edgequake_core::env::apply_model_env_aliases;
use edgequake_core::{ConversationServiceImpl, WorkspaceServiceImpl};
use edgequake_rate_limiter::{RateLimitConfig as TokenBucketConfig, RateLimiter};
use edgequake_storage::{
    traits::{GraphStorage, KVStorage, VectorStorage},
    PgVectorStorage, PgWorkspaceVectorRegistry, PostgresAGEGraphStorage, PostgresKVStorage,
};
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
    pub async fn new_postgres(
        database_url: impl Into<String>,
        llm_api_key: impl Into<String>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use edgequake_llm::ProviderFactory;

        apply_model_env_aliases();

        let database_url = database_url.into();
        let llm_api_key = llm_api_key.into();

        // Set OPENAI_API_KEY for backward compatibility (factory will use it if OpenAI selected)
        if !llm_api_key.is_empty() {
            std::env::set_var("OPENAI_API_KEY", &llm_api_key);
        }

        // FIX #166: Recognize EDGEQUAKE_CHAT_* as aliases for the standard LLM env vars.
        super::provider_setup::apply_chat_env_aliases();

        // Create providers via factory (auto-detects from environment)
        let (llm_provider, embedding_provider) =
            ProviderFactory::from_env().expect("Failed to create LLM provider from environment");

        // Allow a dedicated embedding provider / host to override the default
        // (OLLAMA_EMBEDDING_HOST, EDGEQUAKE_EMBEDDING_PROVIDER, etc.)
        let embedding_provider =
            super::provider_setup::resolve_embedding_provider(embedding_provider);

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
        // WHY 32 connections (env-configurable via DATABASE_POOL_SIZE):
        // The frontend polls ~8 concurrent endpoints every 2s. Pipeline workers
        // hold connections for the full processing duration (embedding = minutes).
        // 10 connections are exhausted instantly under any real load, causing
        // "pool timed out" 500s → polling feedback loop. QW5: raised 25→32 to
        // match PostgresConfig::default max_connections and absorb the new
        // bounded-concurrency batch ingestion (EDGEQUAKE_INGEST_CONCURRENCY).
        let db_pool_size: u32 = std::env::var("DATABASE_POOL_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(32);
        let pg_config = edgequake_storage::adapters::postgres::PostgresConfig::new(
            host, port, database, user, password,
        )
        .with_namespace("default")
        .with_max_connections(db_pool_size);

        // Create PostgreSQL connection pool for conversation service.
        //
        // WHY after_connect sets search_path=public:
        // Migration 001 creates the 'edgequake' schema. After that, PostgreSQL's
        // default search_path "$user",public resolves "$user"="edgequake" to that
        // schema first. Without this, SQLx finds no _sqlx_migrations in edgequake
        // (it's in public), creates a fresh empty one, thinks all migrations are
        // unapplied, then tries to re-insert version=1 into public._sqlx_migrations
        // which already exists → "duplicate key value violates unique constraint
        // _sqlx_migrations_pkey" → panic on every restart.
        // Using after_connect guarantees ALL pool connections always use public,
        // so _sqlx_migrations is consistently read/written in the correct schema.
        //
        // acquire_timeout(5s): fail fast instead of queuing 30s — callers get
        // a quick 500 and back off, rather than stacking up 30s waiters.
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(db_pool_size)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .after_connect(|conn, _| {
                Box::pin(async move {
                    sqlx::query("SET search_path TO public")
                        .execute(conn)
                        .await
                        .map(|_| ())
                })
            })
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
                    tracing::warn!(
                        error.source = "postgres_init",
                        error.action = "extension_check",
                        extension = "pgvector",
                        "pgvector extension not found — vector search may not work"
                    );
                }
                if exts.contains(&"uuid-ossp".to_string()) {
                    tracing::info!("✓ uuid-ossp extension available");
                } else {
                    tracing::warn!(
                        error.source = "postgres_init",
                        error.action = "extension_check",
                        extension = "uuid-ossp",
                        "uuid-ossp extension not found"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    error.source = "postgres_init",
                    error.action = "extension_check",
                    error.message = %e,
                    "Could not check PostgreSQL extensions"
                );
            }
        }

        // SPEC-006: migration bootstrap with progression logs + migration 038 verify/repair
        let migration_bootstrap =
            super::migration_bootstrap::run_postgres_migrations(&pool).await?;

        // Auto-configure vector dimension from embedding provider
        let embedding_dim = embedding_provider.dimension();
        tracing::info!(
            "Using vector dimension {} from {} provider",
            embedding_dim,
            std::env::var("EDGEQUAKE_LLM_PROVIDER").unwrap_or_else(|_| "auto-detected".to_string())
        );

        // Create PostgreSQL-backed storages (SPEC-011: single shared pool)
        use edgequake_storage::adapters::postgres::PostgresPool;
        let storage_pool = PostgresPool::from_existing(pool.clone(), pg_config.clone());
        let kv_storage = Arc::new(PostgresKVStorage::with_pool(
            storage_pool.clone(),
            pg_config.clone(),
        ));
        let vector_storage = Arc::new(PgVectorStorage::with_pool_and_dimension(
            storage_pool.clone(),
            pg_config.clone(),
            embedding_dim,
        ));
        let graph_storage = Arc::new(PostgresAGEGraphStorage::with_pool(
            storage_pool.clone(),
            pg_config.clone(),
        ));

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
        // WHY: Apache AGE (graph extension) may not be available in all PostgreSQL deployments
        // (e.g., pgvector-only images used in CI). Graph storage failure is non-fatal;
        // graph-dependent features (entity extraction, Cypher queries) will degrade gracefully
        // by returning errors, while the server continues to serve all other endpoints.
        if let Err(e) = graph_storage.initialize().await {
            tracing::warn!(
                "⚠ Graph storage (Apache AGE) not available: {} \
                - graph features will be degraded. \
                Install Apache AGE extension for full functionality.",
                e
            );
        }

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

        let pipeline = super::query_bootstrap::build_ingestion_pipeline(
            Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            Arc::clone(&embedding_provider),
        );

        // Create task infrastructure (OODA-06: Use PostgreSQL for task persistence)
        // WHY: Tasks must persist across backend restarts so cancel/retry work correctly.
        // Previous bug: MemoryTaskStorage was used, causing tasks to be lost on restart.
        let task_storage: edgequake_tasks::SharedTaskStorage = Arc::new(
            edgequake_tasks::postgres::PostgresTaskStorage::new(pool.clone()),
        );
        let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));
        tracing::info!("✓ Task storage: PostgreSQL (persistent across restarts)");

        let reranker = create_bm25_reranker();
        let (query_engine, sota_engine) = super::query_bootstrap::build_production_query_engines(
            Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            Arc::clone(&embedding_provider),
            Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            reranker,
        );

        // Create workspace vector registry for per-workspace dimensions
        let vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> =
            Arc::new(PgWorkspaceVectorRegistry::new(
                pg_config,
                storage_pool.clone(),
                Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
                embedding_dim,
            ));

        // Create auth services
        let auth = AuthRuntime::from_env();

        // Create PDF storage (SPEC-007) - uses the connection pool
        let pdf_storage: Arc<dyn edgequake_storage::PdfDocumentStorage> =
            Arc::new(edgequake_storage::PostgresPdfStorage::new(pool.clone()));

        let storage = StorageRuntime {
            kv_storage: Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector_storage)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            vector_registry,
            graph_storage: Arc::clone(&graph_storage)
                as Arc<dyn edgequake_storage::traits::GraphStorage>,
            pdf_storage: Some(pdf_storage),
            mode: StorageMode::PostgreSQL,
        };
        storage.validate_postgres_adapters()?;

        let audit_logger = AuditLogger::new(pool.clone());
        let (resource_guard, graph_materialize) = super::resource_runtime::build_resource_runtime();

        Ok(Self {
            storage,
            query: QueryRuntime {
                llm_provider: Arc::clone(&llm_provider)
                    as Arc<dyn edgequake_llm::traits::LLMProvider>,
                vision_llm_provider: super::provider_setup::resolve_vision_llm_provider(),
                embedding_provider: Arc::clone(&embedding_provider),
                query_engine,
                sota_engine,
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
            pg_pool: Some(pool),
            start_time: std::time::Instant::now(),
            path_validation_config: Self::load_path_validation_config(),
            audit_logger: Some(audit_logger),
            resource_guard,
            graph_materialize,
            migration_bootstrap: Some(migration_bootstrap),
        })
    }
}
