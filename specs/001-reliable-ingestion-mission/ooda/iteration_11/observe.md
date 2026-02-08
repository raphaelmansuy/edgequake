# OODA-11: Observe - Health API Enhancement

## Mission Criterion
> "Ensure health API make it easy to know all parts of the applied configuration (llm provider, embedding provider, models used, database connection status, pdf storage status, etc.)"

## Current Health API Response

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "storage_mode": "postgresql",
  "workspace_id": "default",
  "components": {
    "kv_storage": true,
    "vector_storage": true,
    "graph_storage": true,
    "llm_provider": true
  },
  "llm_provider_name": "ollama",
  "schema": {
    "latest_version": 23,
    "migrations_applied": 22,
    "last_applied_at": "2026-02-08T08:02:10.203594+00:00"
  }
}
```

## Gap Analysis

| Field | Present | Source | Notes |
|-------|---------|--------|-------|
| LLM provider name | ✅ | `state.llm_provider.name()` | Shows "ollama" or "openai" |
| LLM model | ❌ | `state.llm_provider.model()` | **MISSING** - Critical for debugging |
| Embedding provider name | ❌ | `state.embedding_provider.name()` | **MISSING** |
| Embedding model | ❌ | `state.embedding_provider.model()` | **MISSING** |
| Embedding dimension | ❌ | `state.embedding_provider.dimension()` | **MISSING** - Important for vector ops |
| PDF storage status | ❌ | `state.pdf_storage.is_some()` | **MISSING** |
| Database URL (sanitized) | ❌ | N/A | Optional, low priority |

## Code Locations

### HealthResponse struct
**File**: `edgequake/crates/edgequake-api/src/handlers/health_types.rs:14`

```rust
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub storage_mode: String,
    pub workspace_id: String,
    pub components: ComponentHealth,
    pub llm_provider_name: Option<String>,
    pub schema: Option<SchemaHealth>,
}
```

### Health handler implementation
**File**: `edgequake/crates/edgequake-api/src/handlers/health.rs:67`

```rust
pub async fn health_check(State(state): State<AppState>) -> ApiResult<Json<HealthResponse>> {
    // ... builds response using state fields
}
```

### AppState storage of providers
**File**: `edgequake/crates/edgequake-api/src/state.rs:175-181`

```rust
pub pdf_storage: Option<Arc<dyn edgequake_storage::PdfDocumentStorage>>,
pub llm_provider: Arc<dyn LLMProvider>,
pub embedding_provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
```

## Trait Methods Available

### LLMProvider trait (`edgequake-llm/src/traits.rs:137`)
- `fn name(&self) -> &str` ✅
- `fn model(&self) -> &str` ✅
- `fn max_context_length(&self) -> usize` ✅

### EmbeddingProvider trait (`edgequake-llm/src/traits.rs:329`)
- `fn name(&self) -> &str` ✅
- `fn model(&self) -> &str` ✅
- `fn dimension(&self) -> usize` ✅
- `fn max_tokens(&self) -> usize` ✅

## Observed Patterns

1. Both traits expose `name()` and `model()` - consistent interface
2. `EmbeddingProvider` also has `dimension()` - critical for vector storage compatibility
3. PDF storage is `Option<Arc<...>>` - can be None for memory mode
4. Health handler already accesses `state.llm_provider.name()` - extending is straightforward
