//! PostgreSQL storage constructors for `AppState`.
//!
//! Provides the `new_postgres()` factory that wires up persistent PostgreSQL-backed
//! adapters including pgvector, Apache AGE, and conversation/workspace services.

use std::sync::Arc;

use super::config::{AppConfig, SharedConversationService, SharedWorkspaceService, StorageMode};
use super::{ApiSecurityConfig, AppState, AuthRuntime, QueryRuntime, StorageRuntime, TaskRuntime};
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
    /// - `EDGEQUAKE_LLM_PROVIDER=ollama|lmstudio|openai|…` - explicit selection
    /// - `OLLAMA_HOST` present → Ollama provider
    /// - `OPENAI_API_KEY` present → OpenAI provider
    ///
    /// Mock is forbidden as the server default.
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
        super::provider_setup::normalize_local_provider_hosts_for_docker();

        // Create providers via factory (auto-detects from environment)
        let (llm_provider, embedding_provider) =
            ProviderFactory::from_env().expect("Failed to create LLM provider from environment");

        // Application runtime must never serve Mock as the process-wide default
        // unless an explicit test escape hatch is set (EDGEQUAKE_ALLOW_MOCK_PROVIDER=1).
        if crate::provider_visibility::is_mock_provider(llm_provider.name())
            && !crate::provider_visibility::mock_provider_allowed()
        {
            return Err("Mock LLM provider is forbidden as the server default. \
                 Set EDGEQUAKE_LLM_PROVIDER to a real provider (ollama, openai, mistral, …) \
                 and ensure the corresponding credentials/host are available."
                .into());
        }
        if crate::provider_visibility::is_mock_provider(embedding_provider.name())
            && !crate::provider_visibility::mock_provider_allowed()
        {
            return Err(
                "Mock embedding provider is forbidden as the server default. \
                 Set EDGEQUAKE_EMBEDDING_PROVIDER to a real provider (ollama, openai, …)."
                    .into(),
            );
        }

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

        // SPEC-090 F-090-28: role-split pools (query/ingest/queue/admin).
        // Optional DATABASE_READ_URL feeds the query pool (F-090-31).
        // SPEC-112: floor queue pool to resolved worker count so claim_next
        // cannot stampede a 4-conn pool when WORKER_THREADS is larger.
        let worker_floor = edgequake_pipeline::resolve_worker_pool_limits().num_workers as u32;
        let pool_bundle = edgequake_storage::PgPoolBundle::connect_with_queue_floor(
            &database_url,
            Some(worker_floor.max(1)),
        )
        .await?;
        let pool = pool_bundle.ingest.clone();
        let admin_pool = pool_bundle.admin.clone();
        let queue_pool = pool_bundle.queue.clone();
        let query_pool = pool_bundle.query.clone();
        let _db_pool_size = pool_bundle.total_max_connections();
        tracing::info!(
            worker_floor,
            queue_max = pool_bundle.queue_max,
            "SPEC-112: task queue pool sized for claim_next workers"
        );

        // SPEC-112 LAW-112-3: shared-DB slot budget (warn default; fail if env set).
        let pool_budget = match edgequake_storage::check_pool_budget(
            &admin_pool,
            pool_bundle.total_max_connections(),
        )
        .await
        {
            Ok(report) => {
                tracing::info!(
                    need = report.need,
                    limit = report.limit,
                    instances = report.instances,
                    total_pool_max = report.total_pool_max,
                    pg_max = report.pg_max,
                    ok = report.ok,
                    "SPEC-112: pool budget check"
                );
                edgequake_storage::enforce_pool_budget(&report)?;
                Some(report)
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "SPEC-112: pool budget probe failed — continuing without gate"
                );
                None
            }
        };

        let pg_config = edgequake_storage::adapters::postgres::PostgresConfig::new(
            host, port, database, user, password,
        )
        .with_namespace("default")
        .with_max_connections(pool_bundle.ingest_max);

        // Ensure required extensions are available (these should be created in Docker init.sql,
        // but we check and log if they're missing)
        tracing::info!("Checking required PostgreSQL extensions...");

        // Check if essential extensions exist (don't create them - that requires superuser)
        let extensions_result = sqlx::query_scalar::<_, String>(
            "SELECT extname FROM pg_extension WHERE extname IN ('vector', 'uuid-ossp')",
        )
        .fetch_all(&admin_pool)
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

        // SPEC-006 / SPEC-090: migrations + reconcile on **admin** pool only.
        // SPEC-091 LD-15: serving boot is fail-closed verify-only — schema apply
        // is `edgequake migrate` only; pending/newer schema ⇒ exit-78 refusal.
        let migration_bootstrap =
            super::migration_bootstrap::bootstrap_for_serving(&admin_pool).await?;

        // SPEC-091: automatic migration engine (07-migration-engine.md). Verify
        // mode registers jobs + reports estimates; EDGEQUAKE_MIGRATION_MODE=
        // automatic runs the leased, adaptive W1 backfill on the admin pool.
        {
            let prefix = pg_config.table_prefix();
            let kv_table = edgequake_storage::adapters::postgres::qualified_kv_table_name(&prefix);
            let vectors_table = format!("public.eq_{}_vectors", prefix);
            edgequake_storage::migration_engine::spawn_for_serving(
                &admin_pool,
                kv_table,
                vectors_table,
            );
        }

        // SPEC-091 QW1 (LAW-Q3, LD-11): cluster-wide provider budget. Seed the
        // ledger from the resolved budget (one number, one resolver), install
        // it on the local-inference gate, and spawn the stale-slot reaper
        // (EC-22 backstop). budget=0 ⇒ cloud-only: no DB round trips.
        {
            use edgequake_tasks::ProviderBudget as _;
            let provider_budget = edgequake_tasks::provider_budget_from_env();
            if provider_budget > 0 {
                let budget_store = std::sync::Arc::new(
                    edgequake_tasks::PostgresProviderBudget::new(admin_pool.clone()),
                );
                for provider_key in ["ollama", "lmstudio"] {
                    if let Err(e) = budget_store
                        .seed_budget(provider_key, provider_budget, "env")
                        .await
                    {
                        tracing::warn!(
                            provider = provider_key,
                            error = %e,
                            "SPEC-091 QW1: provider budget seed failed (gate falls back to process semaphore)"
                        );
                    }
                }
                let shared: edgequake_tasks::SharedProviderBudget = budget_store.clone();
                crate::local_inference_gate::install_provider_budget(shared);
                tracing::info!(
                    budget = provider_budget,
                    "SPEC-091 QW1: cluster-wide provider budget installed"
                );
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                        match budget_store.reap_expired().await {
                            Ok(n) if n > 0 => {
                                tracing::warn!(
                                    reaped = n,
                                    "SPEC-091 QW1: reaped expired provider slots (EC-22)"
                                );
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!(error = %e, "provider slot reaper failed")
                            }
                        }
                    }
                });
            }
        }

        // SPEC-091 Wave B2: typed quarantine sink — failed compensation cleanups
        // land in public.compensation_quarantine (drained below) instead of KV.
        edgequake_storage::set_quarantine_sink(std::sync::Arc::new(
            edgequake_storage::PgQuarantineSink::new(admin_pool.clone()),
        ));

        // SPEC-091 Wave B3+B4/B5: one pool serves every relational KV-family
        // cutover (wsdoc membership, checkpoints, artifacts) via the shared
        // sidecar registry.
        crate::services::workspace_document_index::register_membership_pool(admin_pool.clone());

        // SPEC-090 F-090-32: HNSW shape manifest drift (log + metric).
        if let Err(e) = edgequake_storage::check_hnsw_index_manifest(&admin_pool).await {
            tracing::warn!(error = %e, "SPEC-090: HNSW index manifest check failed");
        }

        let age_extversion = migration_bootstrap.migration_043.extversion_after.clone();
        let postgres_capabilities =
            edgequake_storage::adapters::postgres::PostgresCapabilities::detect(
                &admin_pool,
                age_extversion,
            )
            .await;
        tracing::info!(
            postgres_major = postgres_capabilities.postgres_major,
            document_id_generator = postgres_capabilities.document_id_generator.as_str(),
            vector_storage_mode = postgres_capabilities.vector_storage_mode.as_str(),
            age_rls_effective = postgres_capabilities.age_rls_effective,
            age_copy_loader = postgres_capabilities.age_copy_loader_effective,
            "SPEC-042-E PostgreSQL capabilities detected"
        );

        // Auto-configure vector dimension from embedding provider
        let embedding_dim = embedding_provider.dimension();
        tracing::info!(
            "Using vector dimension {} from {} provider",
            embedding_dim,
            std::env::var("EDGEQUAKE_LLM_PROVIDER").unwrap_or_else(|_| "auto-detected".to_string())
        );

        // SPEC-090: ingest pool for writes; query pool for QueryEngine reads.
        use edgequake_storage::adapters::postgres::PostgresPool;
        use edgequake_storage::{DimensionEnsureOutcome, DimensionReconcilePolicy};
        let ingest_pool = PostgresPool::from_existing(pool.clone(), pg_config.clone());
        let query_pg = {
            let mut c = pg_config.clone();
            c.max_connections = pool_bundle.query_max;
            PostgresPool::from_existing(query_pool.clone(), c)
        };
        let kv_storage = Arc::new(PostgresKVStorage::with_pool(
            ingest_pool.clone(),
            pg_config.clone(),
        ));
        let graph_storage = Arc::new(PostgresAGEGraphStorage::with_pool(
            ingest_pool.clone(),
            pg_config.clone(),
        ));
        let kv_query = Arc::new(PostgresKVStorage::with_pool(
            query_pg.clone(),
            pg_config.clone(),
        ));
        let graph_query = Arc::new(PostgresAGEGraphStorage::with_pool(
            query_pg.clone(),
            pg_config.clone(),
        ));

        // First principles (SPEC-058 + per-workspace tables):
        // - Empty default table may recreate to match provider dim (schema heal).
        // - Non-empty mismatch keeps stored schema so Acc/other WS stay reachable;
        //   rebind default vector storage to stored dim (PreferExisting).
        // - New workspaces still use provider `embedding_dim` via registry.
        let provisional = PgVectorStorage::with_pool_and_dimension(
            ingest_pool.clone(),
            pg_config.clone(),
            embedding_dim,
        );
        let outcome = provisional
            .reconcile_dimension(embedding_dim, DimensionReconcilePolicy::PreferExisting)
            .await?;
        let (vector_storage, recreated) = match outcome {
            DimensionEnsureOutcome::Matched => (Arc::new(provisional), false),
            DimensionEnsureOutcome::Recreated => (Arc::new(provisional), true),
            DimensionEnsureOutcome::KeptExisting { stored, required } => {
                tracing::warn!(
                    stored_dimension = stored,
                    provider_dimension = required,
                    provider = embedding_provider.name(),
                    "Default vector table kept at stored dim (SPEC-058 PreferExisting). \
                     Default namespace queries need a matching embedding provider or \
                     EDGEQUAKE_ALLOW_VECTOR_TABLE_REBUILD=1 + re-embed. \
                     Per-workspace tables are unaffected."
                );
                (
                    Arc::new(PgVectorStorage::with_pool_and_dimension(
                        ingest_pool.clone(),
                        pg_config.clone(),
                        stored,
                    )),
                    false,
                )
            }
        };
        let vector_query = Arc::new(PgVectorStorage::with_pool_and_dimension(
            query_pg.clone(),
            pg_config.clone(),
            vector_storage.dimension(),
        ));
        if recreated {
            tracing::warn!(
                dimension = embedding_dim,
                provider = embedding_provider.name(),
                "Default vector table recreated due to dimension change (empty or ALLOW_REBUILD). \
                 Documents in the default namespace need re-embed if rows were wiped."
            );
        }
        tracing::info!(
            provider_dimension = embedding_dim,
            default_vector_dimension = vector_storage.dimension(),
            recreated,
            "Default vector storage dimension reconciled"
        );

        // Initialize storage backends to establish connections
        kv_storage.initialize().await?;
        vector_storage.initialize().await?;

        // SPEC-091 W3/IW2 (fail-fast): when the typed backend is authoritative,
        // `chunk_embeddings` (108) and fleet tables (130) must exist — refuse
        // to boot rather than degrade to 42P01 / empty ANN on first read/write.
        if edgequake_storage::vector_backend_reads_typed(
            edgequake_storage::vector_backend_from_env(),
        ) {
            let required: &[&str] = &[
                "chunk_embeddings",
                "entity_embeddings",
                "relationship_embeddings",
                "report_embeddings",
            ];
            let mut missing = Vec::new();
            for table in required {
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
                     WHERE table_schema = 'public' AND table_name = $1)",
                )
                .bind(table)
                .fetch_one(&admin_pool)
                .await
                .unwrap_or(false);
                if !exists {
                    missing.push(*table);
                }
            }
            if !missing.is_empty() {
                return Err(format!(
                    "EDGEQUAKE_VECTOR_BACKEND=typed_embeddings/chunk_embeddings but \
                     missing typed vector table(s): {} — run `edgequake migrate` \
                     (migrations 108 + 130) first.",
                    missing.join(", ")
                )
                .into());
            }
        }
        // SPEC-090 F-090-19: fail-closed on graph init unless EDGEQUAKE_ALLOW_NO_GRAPH=1.
        if let Err(e) = graph_storage.initialize().await {
            let allow = std::env::var("EDGEQUAKE_ALLOW_NO_GRAPH")
                .ok()
                .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
                .unwrap_or(false);
            if allow {
                tracing::warn!(
                    "Graph storage (Apache AGE) not available: {} \
                     — continuing with EDGEQUAKE_ALLOW_NO_GRAPH=1",
                    e
                );
            } else {
                return Err(format!(
                    "Graph storage (Apache AGE) initialize failed: {e}. \
                     Set EDGEQUAKE_ALLOW_NO_GRAPH=1 to start without graph storage."
                )
                .into());
            }
        } else {
            edgequake_storage::spawn_community_backfill_if_needed(
                Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>
            );
        }

        tracing::info!("PostgreSQL storage backends initialized successfully");

        // SPEC-091 IW3 (LD-14): refuse stale rollback flags against dropped stores.
        // SPEC-091 Doc 23: seed KV relation Absent cache from the same census (LAW-KVH2).
        {
            let posture = edgequake_storage::detect_cutover_posture(&admin_pool)
                .await
                .map_err(|e| format!("SPEC-091 cutover posture probe failed: {e}"))?;
            edgequake_storage::validate_cutover_flags(&posture)?;
            kv_storage.seed_relation_from_dropped(posture.kv_store_dropped);
            kv_query.seed_relation_from_dropped(posture.kv_store_dropped);
        }

        // SPEC-091 IW3 (GAP-091-18): compensation-quarantine drain with real applier.
        crate::services::compensation_drain_applier::spawn_compensation_drain_applier(
            admin_pool.clone(),
            Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
            Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
        );

        // SPEC-091 RM0: outbox drain (default on) — mark processed / TTL; compensate dispatch.
        crate::services::outbox_drain_applier::spawn_outbox_drain_applier(
            admin_pool.clone(),
            Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
        );

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

        // SPEC-101: Skip silent Default seed on secure fresh installs (wizard owns creation).
        // Escape hatches: EDGEQUAKE_PROVISION_DEFAULTS / EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD /
        // auth disabled / dev_mode (LAW-101-7).
        let auth_enabled = std::env::var("EDGEQUAKE_AUTH_ENABLED")
            .map(|v| {
                !matches!(
                    v.to_ascii_lowercase().as_str(),
                    "0" | "false" | "no" | "off"
                )
            })
            .unwrap_or(true);
        let dev_mode = std::env::var("EDGEQUAKE_DEV_MODE")
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false);
        if crate::handlers::setup::should_provision_defaults_at_boot(auth_enabled, dev_mode) {
            workspace_service_impl.ensure_defaults().await?;
            tracing::info!("Default tenant and workspace ensured in PostgreSQL");
        } else {
            tracing::info!(
                "SPEC-101: skipping silent default tenant/workspace provision (first-run wizard)"
            );
        }

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
            edgequake_tasks::postgres::PostgresTaskStorage::new(queue_pool.clone()),
        );
        let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));
        tracing::info!("✓ Task storage: PostgreSQL queue pool (SPEC-090 F-090-28)");

        let engine_impl = super::query_bootstrap::build_production_query_engine(
            Arc::clone(&vector_query) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph_query) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            Arc::clone(&embedding_provider),
            Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            Arc::clone(&kv_query) as Arc<dyn edgequake_storage::traits::KVStorage>,
        );

        // Create workspace vector registry for per-workspace dimensions (ingest pool)
        let vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> =
            Arc::new(PgWorkspaceVectorRegistry::new(
                pg_config,
                ingest_pool.clone(),
                Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
                embedding_dim,
            ));

        if migration_bootstrap.migration_080.halfvec_conversion_applied {
            vector_registry.clear_cache().await;
            tracing::warn!(
                target: "edgequake.migration",
                step = "migration_080_cache_cleared",
                operator_action = "verify_embeddings_after_halfvec_conversion",
                "M080 halfvec conversion applied — cleared workspace vector registry cache (SPEC-045 SRE-M05)"
            );
        }

        // Create auth services
        let auth = AuthRuntime::from_env();

        // Create PDF storage (SPEC-007) - uses the connection pool
        let pdf_storage: Arc<dyn edgequake_storage::PdfDocumentStorage> =
            Arc::new(edgequake_storage::PostgresPdfStorage::new(pool.clone()));
        let original_storage: Arc<dyn edgequake_storage::DocumentOriginalStorage> = Arc::new(
            edgequake_storage::PostgresOriginalStorage::new(pool.clone()),
        );
        let mm_asset_storage: Arc<dyn edgequake_storage::DocumentMmAssetStorage> =
            Arc::new(edgequake_storage::PostgresMmAssetStorage::new(pool.clone()));
        let page_layout_storage: Arc<dyn edgequake_storage::DocumentPageLayoutStorage> = Arc::new(
            edgequake_storage::PostgresPageLayoutStorage::new(pool.clone()),
        );

        let storage = StorageRuntime {
            kv_storage: Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector_storage)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            vector_registry,
            graph_storage: Arc::clone(&graph_storage)
                as Arc<dyn edgequake_storage::traits::GraphStorage>,
            auth_memory: Arc::new(crate::services::auth_memory_store::AuthMemoryStore::new()),
            pdf_storage: Some(pdf_storage),
            original_storage: Some(original_storage),
            mm_asset_storage: Some(mm_asset_storage),
            page_layout_storage: Some(page_layout_storage),
            mode: StorageMode::PostgreSQL,
        };
        storage.validate_postgres_adapters()?;

        let audit_logger = AuditLogger::new(pool.clone());
        let (resource_guard, graph_materialize, pdf_vision, read_path_db) =
            super::resource_runtime::build_resource_runtime();

        let configured_pool_size = pool_bundle.total_max_connections() as usize;
        crate::read_path::warn_if_local_pool_oversized(configured_pool_size, llm_provider.name());

        let mut app_state = Self {
            storage,
            query: QueryRuntime {
                llm_provider: Arc::clone(&llm_provider)
                    as Arc<dyn edgequake_llm::traits::LLMProvider>,
                vision_llm_provider: super::provider_setup::resolve_vision_llm_provider(),
                embedding_provider: Arc::clone(&embedding_provider),
                engine_impl,
                pipeline,
                models_config: super::bundled_models::bundled_models_config(),
                model_catalog: Arc::new(crate::model_catalog::ModelCatalog::new()),
            },
            auth,
            tasks: TaskRuntime::new(task_storage, task_queue),
            workspace_service,
            conversation_service,
            config: AppConfig::default(),
            cache_manager: CacheManager::with_defaults(),
            rate_limiter: RateLimiter::new(TokenBucketConfig::default()),
            pg_pool: Some(pool.clone()),
            pool_bundle: Some(pool_bundle),
            pool_budget,
            start_time: std::time::Instant::now(),
            path_validation_config: Self::load_path_validation_config(),
            audit_logger: Some(audit_logger),
            resource_guard,
            graph_materialize,
            pdf_vision,
            parse_jobs: crate::handlers::parse::ParseJobStore::from_env(),
            read_path_db,
            migration_bootstrap: Some(migration_bootstrap),
            postgres_capabilities: Some(postgres_capabilities),
            security: ApiSecurityConfig::from_env(),
            allow_set_provider: None,
            server_config: crate::server_config_store::ServerConfigStore::new(),
        };
        app_state.allow_set_provider = crate::services::spec146_authz::build_allow_set_provider(
            app_state.pg_pool.as_ref(),
            app_state.security.doc_abac,
        );

        // SPEC-043: load server_config LLM defaults into process-wide overrides
        if let Err(e) = app_state.server_config.load_from_pool(&admin_pool).await {
            tracing::warn!(error = %e, "Failed to load server_config LLM defaults at startup");
        } else {
            tracing::info!("Loaded server_config LLM defaults and app attribution (SPEC-043)");
        }

        // SPEC-021 P4-02: Startup storage invariant check + auto-repair (SAFE tier)
        // SPEC-021 P3-01: Log the entity sync mode for observability
        {
            use crate::services::pending_doc_task_reconcile::{
                reconcile_pending_documents_by_ids, reconcile_pending_documents_missing_tasks,
                startup_reconcile_max_from_env,
            };
            use crate::storage_inspector::{
                inv07_sample_ids, InspectorConfig, Inv07HealHook, StorageInspector,
            };
            let inspector = StorageInspector::new(
                Arc::new(admin_pool.clone()),
                InspectorConfig::for_namespace("default"),
            );
            let report = inspector.inspect().await;
            if report.has_critical {
                tracing::error!(
                    schema_issues = report.schema_issues.len(),
                    invariant_violations = report.invariant_violations.len(),
                    "CRITICAL: Storage invariant violations detected at startup (SPEC-021)"
                );
            } else if report.has_warning {
                tracing::warn!(
                    schema_issues = report.schema_issues.len(),
                    invariant_violations = report.invariant_violations.len(),
                    duration_ms = report.duration_ms,
                    "Storage health warnings at startup (SPEC-021)"
                );
            } else {
                tracing::info!(
                    duration_ms = report.duration_ms,
                    "Storage health OK (SPEC-021)"
                );
            }
            let repaired = inspector.auto_repair_safe(&report).await;
            if !repaired.is_empty() {
                tracing::info!(
                    count = repaired.len(),
                    "Storage auto-repairs applied at startup"
                );
            }

            // INV-07: budgeted SPEC-054 heal of sample ids at startup (inspect ≠ apply_repair).
            let inv07_ids = inv07_sample_ids(&report);
            if !inv07_ids.is_empty() {
                let budget = startup_reconcile_max_from_env();
                tracing::info!(
                    count = inv07_ids.len(),
                    "INV-07 at startup: budgeted SPEC-054 reconcile for sample ids"
                );
                if let Err(e) =
                    reconcile_pending_documents_by_ids(&app_state, &inv07_ids, budget, "inv07_startup")
                        .await
                {
                    tracing::warn!(error = %e, "INV-07 startup sample reconcile failed");
                }
                if let Err(e) = reconcile_pending_documents_missing_tasks(
                    &app_state,
                    budget,
                    "inv07_startup_backlog",
                )
                .await
                {
                    tracing::warn!(error = %e, "INV-07 startup backlog reconcile failed");
                }
            }

            // SPEC-021 P-D1: hourly monitor + INV-07 sample heal hook.
            let heal_state = app_state.clone();
            let inv07_heal: Inv07HealHook = Arc::new(move |ids: Vec<String>| {
                let state = heal_state.clone();
                Box::pin(async move {
                    let budget = startup_reconcile_max_from_env();
                    if let Err(e) =
                        reconcile_pending_documents_by_ids(&state, &ids, budget, "inv07_hourly")
                            .await
                    {
                        tracing::warn!(error = %e, "INV-07 hourly sample reconcile failed");
                    }
                    if let Err(e) = reconcile_pending_documents_missing_tasks(
                        &state,
                        budget,
                        "inv07_hourly_backlog",
                    )
                    .await
                    {
                        tracing::warn!(error = %e, "INV-07 hourly backlog reconcile failed");
                    }
                })
            });
            std::sync::Arc::new(inspector).spawn_hourly_monitor(Some(inv07_heal));
        }

        // Rate-limit cleanup is started from create_router (SPEC-083 S-11).
        Ok(app_state)
    }
}
