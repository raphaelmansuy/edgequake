# OODA Loop Iteration #06 - Orient Phase

**Date:** 2026-01-11  
**Mission:** Architecture Understanding & Implementation Strategy  
**Phase:** Orient (System Analysis & Design Planning)

---

## Executive Summary

**Deep Dive Findings:**

- ✅ Query engine uses global embedding provider from AppState (line 256 [`engine.rs`](../../edgequake/crates/edgequake-query/src/engine.rs#L256))
- ✅ Vector storage has `clear()` method for DB rebuild (line 125 [`vector.rs`](../../edgequake/crates/edgequake-storage/src/traits/vector.rs#L125))
- ❌ No workspace-level provider configuration mechanism
- ❌ Query engine cannot dynamically switch embedding providers per workspace

**Key Insight:** Current architecture assumes single global embedding provider. Workspace-level embedding requires significant refactoring.

---

## 1. Query Engine Architecture Analysis

### 1.1 Current Flow

**File:** [`engine.rs`](../../edgequake/crates/edgequake-query/src/engine.rs#L250-L305)

```rust
pub struct QueryEngine {
    config: QueryEngineConfig,
    vector_storage: Arc<dyn VectorStorage>,
    graph_storage: Arc<dyn GraphStorage>,
    embedding_provider: Arc<dyn EmbeddingProvider>, // ← GLOBAL provider
    llm_provider: Arc<dyn LLMProvider>,
    // ...
}

pub async fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
    // Step 1: Generate query embedding using GLOBAL provider
    let query_embedding = self.embedding_provider.embed_one(&request.query).await?;

    // Step 2: Search vectors (assumes same dimension as stored embeddings)
    let context = self.retrieve_context(
        &request.query,
        &query_embedding,
        mode,
        request.tenant_id(),
        request.workspace_id(), // ← workspace_id passed but not used for provider selection
    ).await?;
}
```

**Problem:** Embedding provider is fixed at QueryEngine construction time, stored in AppState.

### 1.2 AppState Architecture

**File:** [`state.rs`](../../edgequake/crates/edgequake-api/src/state.rs#L1-L150)

```rust
pub struct AppState {
    // Global providers (same for all workspaces)
    pub llm_provider: Arc<dyn LLMProvider>,
    pub embedding_provider: Arc<dyn EmbeddingProvider>,

    // Services
    pub query_engine: Arc<QueryEngine>, // ← Uses global embedding_provider
    pub pipeline: Arc<Pipeline>,
    pub workspace_service: SharedWorkspaceService,
    // ...
}
```

**Implication:** Changing embedding provider requires restarting server to rebuild AppState.

### 1.3 Required Architecture Changes

**Strategy A: Workspace-Aware Query Engine** (Recommended)

```rust
// New approach: Query engine factory pattern
pub struct WorkspaceQueryEngine {
    config: QueryEngineConfig,
    storage_factory: Arc<dyn StorageFactory>, // Creates storage per workspace
    provider_factory: Arc<dyn ProviderFactory>, // Creates providers per workspace
}

impl WorkspaceQueryEngine {
    pub async fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        // 1. Lookup workspace configuration
        let workspace_id = request.workspace_id().ok_or(QueryError::MissingWorkspace)?;
        let workspace_config = self.workspace_service.get_config(workspace_id).await?;

        // 2. Create embedding provider for this workspace
        let embedding_provider = self.provider_factory
            .create_embedding(workspace_config.embedding_model)?;

        // 3. Generate query embedding with workspace-specific provider
        let query_embedding = embedding_provider.embed_one(&request.query).await?;

        // 4. Ensure dimension matches workspace vector storage
        assert_eq!(query_embedding.len(), workspace_config.embedding_dimension);

        // ... continue query
    }
}
```

**Strategy B: Provider Registry** (Alternative)

```rust
// Maintain map of embedding providers, switch dynamically
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn EmbeddingProvider>>, // key: model name
}

// Query engine looks up provider per request
let provider_key = workspace_config.embedding_model;
let embedding_provider = self.provider_registry.get(&provider_key)?;
```

---

## 2. Vector Storage Rebuild Capability

### 2.1 Existing Trait Methods

**File:** [`vector.rs`](../../edgequake/crates/edgequake-storage/src/traits/vector.rs#L51-L125)

```rust
#[async_trait]
pub trait VectorStorage: Send + Sync {
    // ...

    /// Clear all vectors.
    async fn clear(&self) -> Result<()>; // ← Exists! Can use for rebuild

    /// Get count of stored vectors.
    async fn count(&self) -> Result<usize>;

    /// Check if storage is empty.
    async fn is_empty(&self) -> Result<bool>;
}
```

**Good News:** `clear()` method already exists. No trait changes needed.

### 2.2 Rebuild Workflow Design

```rust
// New endpoint: POST /api/v1/workspaces/:id/rebuild-embeddings
pub async fn rebuild_workspace_embeddings(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<RebuildRequest>,
) -> Result<Json<RebuildResponse>, ApiError> {
    // 1. Validate new embedding model
    let new_provider = ProviderFactory::create_embedding(&request.new_embedding_model)?;

    // 2. Lock workspace for writes (prevent concurrent queries)
    state.workspace_service.lock(workspace_id).await?;

    // 3. Clear existing vector storage
    state.vector_storage.clear().await?;

    // 4. Retrieve all documents from workspace
    let documents = state.kv_storage.get_workspace_documents(workspace_id).await?;

    // 5. Re-embed all documents with new provider
    for doc in documents {
        let embeddings = new_provider.embed(&doc.chunks).await?;
        state.vector_storage.upsert(&embeddings).await?;
    }

    // 6. Update workspace configuration
    state.workspace_service.update_embedding_model(workspace_id, request.new_embedding_model).await?;

    // 7. Unlock workspace
    state.workspace_service.unlock(workspace_id).await?;

    Ok(Json(RebuildResponse { success: true, vectors_rebuilt: documents.len() }))
}
```

### 2.3 Postgres vs Memory Storage

**Postgres** ([`PostgresAGEGraphStorage`](../../edgequake/crates/edgequake-storage/src/adapters/postgres_age.rs)):

- `clear()` executes `DELETE FROM vectors WHERE workspace_id = $1`
- Transactional, can rollback on failure
- Supports concurrent readers during rebuild (with locking)

**Memory** ([`MemoryVectorStorage`](../../edgequake/crates/edgequake-storage/src/adapters/memory/vector.rs)):

- `clear()` executes `self.vectors.write().unwrap().clear()`
- Non-transactional, data lost if server crashes mid-rebuild
- Requires exclusive lock for safety

---

## 3. Workspace Schema Extension

### 3.1 Current Schema

**File:** [`workspaces_types.rs`](../../edgequake/crates/edgequake-api/src/handlers/workspaces_types.rs#L42-L106)

```rust
// Current (MISSING embedding fields)
pub struct CreateWorkspaceApiRequest {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub max_documents: Option<usize>,
}

pub struct WorkspaceResponse {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub max_documents: Option<usize>,
    pub created_at: String,
    pub updated_at: String,
}
```

### 3.2 Required Fields

```rust
// New (WITH embedding configuration)
pub struct CreateWorkspaceApiRequest {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub max_documents: Option<usize>,

    // NEW FIELDS
    /// Embedding model name (e.g., "text-embedding-3-small", "embeddinggemma:latest")
    /// If None, uses server default from EDGEQUAKE_DEFAULT_EMBEDDING_MODEL
    pub embedding_model: Option<String>,

    /// Embedding provider (e.g., "openai", "ollama")
    /// If None, auto-detected from embedding_model name
    pub embedding_provider: Option<String>,
}

pub struct WorkspaceResponse {
    // ... existing fields ...

    // NEW FIELDS
    pub embedding_model: String,          // e.g., "text-embedding-3-small"
    pub embedding_provider: String,       // e.g., "openai"
    pub embedding_dimension: usize,       // e.g., 1536
    pub vector_count: usize,              // Number of embeddings stored
}
```

### 3.3 Database Migration

**Postgres Schema Change:**

```sql
-- Add columns to workspaces table
ALTER TABLE workspaces
ADD COLUMN embedding_model VARCHAR(255) DEFAULT 'text-embedding-3-small',
ADD COLUMN embedding_provider VARCHAR(50) DEFAULT 'openai',
ADD COLUMN embedding_dimension INTEGER DEFAULT 1536;

-- Backfill existing workspaces with server defaults
UPDATE workspaces
SET embedding_model = COALESCE(
    (SELECT value FROM server_config WHERE key = 'default_embedding_model'),
    'text-embedding-3-small'
);

-- Add index for lookups
CREATE INDEX idx_workspaces_embedding ON workspaces(embedding_model);
```

**Memory Storage:** No migration needed (in-memory HashMap, just add fields).

---

## 4. LM Studio Provider Implementation

### 4.1 Current Issue

**File:** [`factory.rs#L200-L225`](../../edgequake/crates/edgequake-llm/src/factory.rs#L200-L225)

```rust
// Current: LM Studio uses OpenAI compatibility wrapper
fn create_lmstudio() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
    let provider = Arc::new(
        OpenAIProvider::compatible("lmstudio-key", base_url)
            .with_model(&model)
            .with_embedding_model(&embedding_model)
            .with_embedding_dimension(embedding_dim)
    );

    // PROBLEM: Cannot access LM Studio-specific APIs:
    // - GET /v1/models (list available models)
    // - Health check endpoint
    // - Custom error codes
}
```

### 4.2 Dedicated LM Studio Provider Design

**New File:** [`lmstudio.rs`](../../edgequake/crates/edgequake-llm/src/providers/lmstudio.rs)

```rust
/// LM Studio provider with native API support.
#[derive(Debug, Clone)]
pub struct LMStudioProvider {
    client: Client,
    host: String,
    model: String,
    embedding_model: String,
    max_context_length: usize,
    embedding_dimension: usize,
}

impl LMStudioProvider {
    /// Create from environment variables.
    pub fn from_env() -> Result<Self> {
        let host = std::env::var("LMSTUDIO_HOST")
            .unwrap_or_else(|_| "http://localhost:1234".to_string());

        // LM Studio-specific: Query server for available models
        let model = Self::detect_default_model(&host).await?;

        // ...
    }

    /// List available models from LM Studio server.
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/v1/models", self.host);
        let response: ModelsResponse = self.client.get(&url).send().await?.json().await?;
        Ok(response.data.into_iter().map(|m| m.id).collect())
    }

    /// Health check (ping LM Studio server).
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let url = format!("{}/health", self.host);
        // LM Studio may not have /health, use /v1/models as proxy
        let status = self.client.get(&url).send().await?.status();
        Ok(if status.is_success() { HealthStatus::Healthy } else { HealthStatus::Unhealthy })
    }
}
```

**Benefits:**

- Native model discovery
- Health checks
- Better error messages (LM Studio-specific codes)
- Can detect embedding dimensions automatically

---

## 5. WebUI Provider Selector Design

### 5.1 Current Query Page

**File:** [`query/page.tsx`](<../../edgequake_webui/src/app/(dashboard)/query/page.tsx>)

**Current UI:** Only has query mode dropdown (local/global/hybrid/naive). No provider selector.

### 5.2 Proposed UI

```tsx
// New component: Provider & Model Selector
<div className="provider-selector">
  <Select value={provider} onValueChange={setProvider}>
    <SelectTrigger>
      <SelectValue placeholder="Select Provider" />
    </SelectTrigger>
    <SelectContent>
      <SelectGroup>
        <SelectLabel>OpenAI</SelectLabel>
        <SelectItem value="openai:gpt-4o-mini">GPT-4o Mini</SelectItem>
        <SelectItem value="openai:gpt-4-turbo">GPT-4 Turbo</SelectItem>
      </SelectGroup>
      <SelectGroup>
        <SelectLabel>Ollama (Local)</SelectLabel>
        <SelectItem value="ollama:gemma3:12b">Gemma 3 12B</SelectItem>
        <SelectItem value="ollama:llama3">Llama 3</SelectItem>
      </SelectGroup>
      <SelectGroup>
        <SelectLabel>LM Studio (Local)</SelectLabel>
        <SelectItem value="lmstudio:gemma2-9b-it">Gemma 2 9B IT</SelectItem>
      </SelectGroup>
    </SelectContent>
  </Select>
</div>
```

**Placement:** Right above query input box, next to query mode dropdown.

### 5.3 API Integration

```typescript
// New API endpoint: GET /api/v1/providers/available
interface AvailableProvider {
  provider: string; // "openai" | "ollama" | "lmstudio"
  models: string[]; // ["gpt-4o-mini", "gpt-4-turbo"]
  status: "connected" | "disconnected";
}

// Query request includes provider override
fetch("/api/v1/query", {
  method: "POST",
  body: JSON.stringify({
    query: "What is EdgeQuake?",
    mode: "hybrid",
    provider_override: "ollama:gemma3:12b", // NEW
  }),
});
```

---

## 6. Implementation Sequence

### Phase 1: Foundation (Iterations 06-15)

1. **Database Schema** (Iteration 07-08)

   - Add embedding fields to workspace table
   - Migration script for existing workspaces
   - Update workspace service to handle new fields

2. **LM Studio Provider** (Iteration 09-12)

   - Create `lmstudio.rs` with native API support
   - Model discovery and health checks
   - Integration tests

3. **Provider Registry** (Iteration 13-15)
   - Create ProviderRegistry for dynamic provider switching
   - Workspace-to-provider mapping service
   - Cache provider instances for performance

### Phase 2: Query Engine Refactor (Iterations 16-25)

4. **Workspace-Aware Query** (Iteration 16-20)

   - Modify QueryEngine to lookup workspace embedding model
   - Create embedding provider per request
   - Dimension validation

5. **Vector DB Rebuild** (Iteration 21-25)
   - API endpoint for rebuild trigger
   - Progress tracking and status updates
   - Edge case handling (concurrent queries, failures)

### Phase 3: WebUI Integration (Iterations 26-35)

6. **Provider Selector UI** (Iteration 26-30)

   - Available providers API endpoint
   - Provider dropdown component
   - Query page integration

7. **Workspace Creation UI** (Iteration 31-35)
   - Embedding model selector
   - Default model from server config
   - Dimension mismatch warnings

### Phase 4: Testing & Documentation (Iterations 36-50)

8. **Comprehensive Testing** (Iteration 36-45)

   - Postgres vs Memory storage tests
   - Cross-provider query tests
   - Non-regression suite

9. **Documentation & Guides** (Iteration 46-48)

   - Setup guides per provider
   - API migration guide
   - Architecture diagrams

10. **Final Validation** (Iteration 49-50)
    - Full E2E test suite
    - Performance benchmarks
    - Security audit

---

## 7. Critical Design Decisions

### Decision 1: Provider Instance Caching

**Problem:** Creating new embedding provider per query is expensive (API key validation, connection pool setup).

**Solution:** Provider registry with cached instances.

```rust
pub struct ProviderRegistry {
    cache: RwLock<HashMap<String, Arc<dyn EmbeddingProvider>>>,
}

impl ProviderRegistry {
    pub async fn get_or_create(&self, model: &str) -> Result<Arc<dyn EmbeddingProvider>> {
        // Try read lock first
        if let Some(provider) = self.cache.read().unwrap().get(model) {
            return Ok(provider.clone());
        }

        // Upgrade to write lock
        let mut cache = self.cache.write().unwrap();

        // Double-check (another thread may have created it)
        if let Some(provider) = cache.get(model) {
            return Ok(provider.clone());
        }

        // Create new provider
        let provider = Arc::new(self.factory.create(model)?);
        cache.insert(model.to_string(), provider.clone());
        Ok(provider)
    }
}
```

### Decision 2: Workspace Lock During Rebuild

**Problem:** Queries during vector rebuild may get inconsistent results (some old embeddings, some new).

**Solution:** Optimistic locking with retry.

```rust
pub struct WorkspaceLock {
    rebuilding: RwLock<HashSet<Uuid>>, // workspace IDs currently rebuilding
}

// Query handler checks lock
if workspace_lock.is_rebuilding(workspace_id).await? {
    return Err(QueryError::WorkspaceRebuilding {
        workspace_id,
        retry_after_seconds: 60,
    });
}

// Rebuild acquires exclusive lock
workspace_lock.acquire_rebuild_lock(workspace_id).await?;
defer! { workspace_lock.release_rebuild_lock(workspace_id).await }
```

### Decision 3: Default Embedding Model

**Problem:** New workspaces need default embedding model, but server may have multiple providers available.

**Solution:** Server-level configuration with provider priority.

```bash
# Environment variables
EDGEQUAKE_DEFAULT_EMBEDDING_MODEL=text-embedding-3-small
EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER=openai

# Fallback chain:
# 1. Explicitly configured default
# 2. Auto-detected from OPENAI_API_KEY
# 3. Ollama if OLLAMA_HOST set
# 4. Mock provider (error state - require explicit config)
```

---

## 8. Risk Mitigation

### Risk 1: Breaking API Changes

**Mitigation:**

- Keep old API endpoints working (v1)
- Add new endpoints with version (v2)
- Deprecation warnings for 2 releases

### Risk 2: Data Migration Failures

**Mitigation:**

- Test migration script on copy of production data
- Provide rollback SQL script
- Add database backup step to deployment process

### Risk 3: Query Performance Degradation

**Mitigation:**

- Benchmark before/after refactoring
- Provider instance caching
- Database index on workspace.embedding_model

---

## 9. Metrics & Success Criteria

### Code Metrics

- [ ] < 5% increase in query latency (P95)
- [ ] < 10% increase in memory usage (AppState)
- [ ] 100% test coverage for new provider code
- [ ] Zero clippy warnings

### Feature Metrics

- [ ] Provider switching works without server restart
- [ ] Vector rebuild completes in < 5 minutes for 1000 documents
- [ ] Dimension mismatch detected before query execution
- [ ] Concurrent queries blocked during rebuild

---

## 10. Next Steps (Decide Phase)

**Iteration 07 Actions:**

1. Create detailed implementation plan for workspace schema changes
2. Design database migration strategy
3. Plan rollback procedures
4. Identify integration test scenarios

**Blockers to Resolve:**

- [ ] Confirm LM Studio actual model names (spec may be incorrect)
- [ ] Decide on provider registry vs factory pattern
- [ ] Determine workspace lock mechanism (DB-based or in-memory)

---

**Commit Message for Iteration 06 Orient:**

```
docs(ooda-06): Architecture analysis for workspace-level embeddings

Analyzed query engine, storage, and state management:
- QueryEngine uses global embedding provider (refactor required)
- VectorStorage has clear() for rebuild (no trait changes needed)
- Workspace schema missing embedding fields (DB migration required)
- LM Studio needs dedicated provider (not OpenAI wrapper)

Design decisions:
- Provider registry with caching (performance)
- Optimistic locking during rebuild (consistency)
- Server-level default embedding configuration

Implementation phases (50 OODA loops):
- Phase 1 (06-15): Foundation (schema, LM Studio, registry)
- Phase 2 (16-25): Query engine refactor + rebuild logic
- Phase 3 (26-35): WebUI provider selector + workspace UI
- Phase 4 (36-50): Testing, docs, validation

Files analyzed:
- edgequake-query/src/engine.rs (QueryEngine architecture)
- edgequake-storage/src/traits/vector.rs (clear() method exists)
- edgequake-api/src/state.rs (AppState structure)

Next: Decide phase (iteration 07) - Workspace schema design
```
