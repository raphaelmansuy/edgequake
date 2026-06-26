# 02 — Storage Runtime Wiring

> **Spec**: 021-storage-study  
> **File**: 04-api-storage-usage/02-storage-runtime.md  
> **Date**: 2026-06-25  
> **Source**: `edgequake-api/src/state/`,  
> `edgequake-api/src/state/storage_runtime.rs`,  
> `edgequake-api/src/state/postgres.rs`

---

## AppState Structure

`AppState` is the central DI container for the entire API. It is wrapped in
`Arc<AppState>` and shared by all Axum handlers via Axum state injection.

```
AppState {
  storage: StorageRuntime {
    kv_storage:      Arc<dyn KVStorage>          // eq_*_kv
    vector_storage:  Arc<dyn VectorStorage>      // eq_*_vectors (global default)
    vector_registry: Arc<dyn WorkspaceVectorRegistry> // per-workspace routing
    graph_storage:   Arc<dyn GraphStorage>       // AGE graph
    pdf_storage:     Option<Arc<dyn PdfDocumentStorage>>  // pdf_documents
    mode:            StorageMode (Memory | PostgreSQL)
  },

  query: QueryRuntime {
    query_engine:     Arc<dyn QueryEngine>        // SOTA query
    pipeline:         Arc<Pipeline>               // pipeline config
    llm_provider:     Arc<dyn LLMProvider>        // OpenAI / Ollama
    embedding_provider: Arc<dyn EmbeddingProvider>
    reranker:         Arc<dyn Reranker>           // BM25
  },

  auth: AuthRuntime {
    auth_state:  Option<AuthState>               // JWT + password auth
    rate_limiter: RateLimiter                    // tenant rate limiting
  },

  tasks: TaskRuntime {
    task_storage:   Arc<dyn TaskStorage>         // edgequake_tasks table
    task_processor: Arc<DocumentTaskProcessor>
    progress_tx:    Sender<ProgressUpdate>       // WebSocket progress
    cancel_tokens:  DashMap<Uuid, CancellationToken>
  },

  workspace_service:    SharedWorkspaceService,   // tenants + workspaces
  conversation_service: SharedConversationService, // conversations + messages

  cache_manager:    CacheManager,               // LRU cache (conversations, messages)
  rate_limiter:     RateLimiter,
  pg_pool:          Option<PgPool>,             // shared sqlx pool
  audit_logger:     Option<AuditLogger>,
  resource_guard:   ResourceGuard,              // SPEC-006 admission control
  graph_materialize: Arc<GraphMaterializationSemaphore>,
  start_time:       Instant,
  config:           AppConfig,
}
```

---

## PostgreSQL Adapter Initialization Sequence

```
AppState::new_postgres(database_url, llm_api_key)
    |
    +--> Parse DATABASE_URL
    +--> Create sqlx::PgPool (max_connections=32, acquire_timeout=5s)
    +--> SET search_path TO public (after_connect hook)
    |
    +--> Run SQLx migrations (migrations/ directory)
    |    [001_init_database.sql ... 038_add_source_ids_gin_indexes.sql]
    |
    +--> Create PostgresConfig (host, port, db, user, password, namespace="default")
    +--> Create shared PostgresPool (wraps sqlx::PgPool)
    |
    +--> Construct storage adapters:
    |    |-- PostgresKVStorage::with_pool(pool, config)
    |    |    |-- kv.initialize() -> CREATE TABLE eq_eq_default_kv
    |    |
    |    |-- PgVectorStorage::with_pool_and_dimension(pool, config, dim=1536)
    |    |    |-- vector.initialize() -> CREATE TABLE eq_eq_default_vectors
    |    |
    |    |-- PgWorkspaceVectorRegistry::new(config, pool, default_storage, 1536)
    |    |    [lazy: workspace tables created on first get_or_create() call]
    |    |
    |    |-- PostgresAGEGraphStorage::new(config)
    |    |    |-- graph.initialize() -> create AGE graph 'edgequake'
    |    |
    |    |-- PostgresPdfStorage::new(pool)
    |         [uses shared pg_pool for pdf_documents table]
    |
    +--> Construct services:
    |    |-- WorkspaceServiceImpl::new(pool) -> ensures default tenant/workspace
    |    |-- ConversationServiceImpl::new(pool)
    |
    +--> Resolve LLM+embedding providers (ProviderFactory::from_env())
    +--> Build SOTAQueryEngine with storage + providers
    +--> Build DocumentTaskProcessor
    +--> Build AuthRuntime (JWT secret, password hasher)
    +--> Initialize default tenant/workspace if not exists
    |
    +--> Return AppState
```

---

## Memory Adapter Initialization

```
AppState::new_memory()
    |
    +--> MemoryKVStorage::new("default")
    +--> MemoryVectorStorage::new("default", 1536)
    +--> MemoryGraphStorage::new("default")
    +--> MemoryWorkspaceVectorRegistry::new(default_vector_storage)
    +--> MemoryPdfStorage::new()
    +--> InMemoryWorkspaceService::new()
    +--> InMemoryConversationService::new()
    +--> MockLLMProvider / MockEmbeddingProvider
    +--> Return AppState (no database required)
```

---

## Workspace Pipeline Factory

`WorkspacePipelineFactory` provides per-workspace pipeline + vector storage:

```rust
pub struct WorkspacePipelineFactory {
    vector_registry: Arc<dyn WorkspaceVectorRegistry>,
    pg_pool: Option<Arc<PgPool>>,
    // ...
}

impl WorkspacePipelineFactory {
    pub async fn get_or_create_workspace_pipeline(
        &self,
        workspace_id: Uuid,
        embedding_dim: usize,
    ) -> Result<(Arc<Pipeline>, Arc<dyn VectorStorage>)>
```

Used by query handlers to route to the correct workspace vector table.

Source: `edgequake-api/src/workspace_pipeline_factory.rs`

---

## Shared Connection Pool Architecture

All PostgreSQL adapters share **one** `sqlx::PgPool` to avoid connection exhaustion:

```
sqlx::PgPool (max_connections=32)
    |
    +-- PostgresKVStorage       (via PostgresPool wrapper)
    +-- PgVectorStorage         (via PostgresPool wrapper)
    +-- PgWorkspaceVectorRegistry -> creates PgVectorStorage instances (same pool)
    +-- PostgresAGEGraphStorage (via PostgresPool wrapper)
    +-- PostgresPdfStorage      (direct PgPool)
    +-- WorkspaceServiceImpl    (direct PgPool)
    +-- ConversationServiceImpl (direct PgPool)
    +-- TaskStorage             (direct PgPool)
    +-- AuditLogger             (direct PgPool)
```

`PostgresPool` is a thin wrapper around `sqlx::PgPool` that:
- Lazily initializes the pool on first `get()` call
- Caches the underlying pool in an `Arc<OnceLock<PgPool>>`

Source: `edgequake-storage/src/adapters/postgres/connection.rs`
