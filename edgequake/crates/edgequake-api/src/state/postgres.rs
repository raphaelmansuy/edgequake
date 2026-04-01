//! PostgreSQL storage constructors for `AppState`.
//!
//! Provides the `new_postgres()` factory that wires up persistent PostgreSQL-backed
//! adapters including pgvector, Apache AGE, and conversation/workspace services.

use std::sync::Arc;

use edgequake_auth::{AuthConfig, JwtService, PasswordService, RbacService};
use edgequake_core::{ConversationServiceImpl, WorkspaceServiceImpl};
use edgequake_llm::ModelsConfig;
use edgequake_pipeline::Pipeline;
use edgequake_query::{QueryEngine, QueryEngineConfig, SOTAQueryConfig, SOTAQueryEngine};
use edgequake_rate_limiter::{RateLimitConfig as TokenBucketConfig, RateLimiter};
use edgequake_storage::{
    traits::{GraphStorage, KVStorage, VectorStorage},
    PgVectorStorage, PgWorkspaceVectorRegistry, PostgresAGEGraphStorage, PostgresKVStorage,
};
use edgequake_tasks::PipelineState;

use super::config::{AppConfig, SharedConversationService, SharedWorkspaceService, StorageMode};
use super::{create_bm25_reranker, AppState};
use crate::cache_manager::CacheManager;
use crate::handlers::ProgressBroadcaster;

fn is_sqlx_database_duplicate_key(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(db_error) => {
            db_error.code().as_deref() == Some("23505")
                && db_error.constraint().as_deref() == Some("_sqlx_migrations_pkey")
        }
        _ => false,
    }
}

fn is_sqlx_migration_duplicate_key(error: &sqlx::migrate::MigrateError) -> bool {
    let rendered = error.to_string();
    let debug_rendered = format!("{error:?}");

    rendered.contains("_sqlx_migrations_pkey")
        || rendered.contains("duplicate key value violates unique constraint")
        || debug_rendered.contains("_sqlx_migrations_pkey")
        || debug_rendered.contains("duplicate key value violates unique constraint")
}

async fn repair_sqlx_migrations(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    tracing::warn!(
        "Detected stale SQLx migration history; clearing SQLx bookkeeping and retrying once"
    );

    let removed = sqlx::query("DELETE FROM public._sqlx_migrations")
        .execute(pool)
        .await?
        .rows_affected();

    tracing::warn!(
        removed = removed,
        "Cleared SQLx migration bookkeeping rows before retry"
    );

    Ok(())
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
    /// LLM/embedding providers are created by the shared startup provider
    /// selection helper, which prefers explicit EdgeQuake model/provider env
    /// vars and falls back to provider auto-detection.
    ///
    /// The `llm_api_key` parameter is kept for backward compatibility and will set `OPENAI_API_KEY`
    /// when provided. For Ollama/LM Studio, you can pass an empty string and use environment
    /// variables.
    pub async fn new_postgres(
        database_url: impl Into<String>,
        llm_api_key: impl Into<String>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let database_url = database_url.into();
        let llm_api_key = llm_api_key.into();

        // Set OPENAI_API_KEY for backward compatibility (factory will use it if OpenAI selected)
        if !llm_api_key.is_empty() {
            std::env::set_var("OPENAI_API_KEY", &llm_api_key);
        }

        super::sync_openai_compatible_api_key();

        // Create providers via the shared startup selection helper.
        let (llm_provider, embedding_provider) = super::create_startup_providers()
            .expect("Failed to create LLM provider from environment");

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

        // Run migrations from the workspace root migrations directory.
        // If SQLx's bookkeeping table gets out of sync, reset it once and retry
        // because these migrations are intentionally idempotent.
        tracing::info!("Running database migrations...");
        if let Err(initial_error) = sqlx::migrate!("../../migrations").run(&pool).await {
            if is_sqlx_migration_duplicate_key(&initial_error) {
                repair_sqlx_migrations(&pool).await?;
                sqlx::migrate!("../../migrations")
                    .run(&pool)
                    .await
                    .map_err(|retry_error| {
                        tracing::error!(
                            error = %retry_error,
                            "SQLx migration retry failed after bookkeeping reset"
                        );
                        retry_error
                    })?;
            } else {
                return Err(initial_error.into());
            }
        }
        tracing::info!("✓ Database migrations completed successfully");

        // Auto-configure vector dimension from embedding provider
        let embedding_dim = embedding_provider.dimension();
        tracing::info!(
            "Using vector dimension {} from {} provider",
            embedding_dim,
            embedding_provider.name()
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
        let task_storage: edgequake_tasks::SharedTaskStorage = Arc::new(
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
            cancellation_registry: edgequake_tasks::CancellationRegistry::new(),
        })
    }
}
