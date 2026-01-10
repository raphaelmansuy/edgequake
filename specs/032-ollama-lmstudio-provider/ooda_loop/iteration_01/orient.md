# OODA Loop Iteration #1: Orient

**Date:** 2026-01-10  
**Phase:** Analysis & Architecture Planning

## 📋 Previous Observations Summary

From [observe.md](./observe.md):

- ✅ Ollama provider EXISTS but with wrong defaults
- ❌ No LM Studio provider
- ❌ No provider factory/selection system
- ❌ No vector DB migration mechanism
- ✅ Vector storage abstraction with dimension configuration

## 🏗️ Architecture Analysis

### Current Provider Architecture

**Trait Hierarchy:**

```
LLMProvider (trait)          EmbeddingProvider (trait)
    ├── OpenAIProvider           ├── OpenAIProvider
    ├── AzureOpenAIProvider      ├── AzureOpenAIProvider
    ├── OllamaProvider           ├── OllamaProvider
    ├── GeminiProvider           ├── GeminiProvider
    ├── JinaProvider (embedding only)
    └── MockProvider             └── MockProvider
```

**Key Finding:** `OpenAIProvider::compatible()` method enables OpenAI-compatible APIs!

```rust
// edgequake/crates/edgequake-llm/src/providers/openai.rs:54
pub fn compatible(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
    let config = OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(base_url);
    Self::with_config(config)
}
```

**Implication:** LM Studio can use `OpenAIProvider::compatible()` since it's OpenAI-compatible!

### Vector Storage Architecture

**File:** [edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs](../../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L52-L78)

```rust
pub struct PgVectorStorage {
    dimension: usize,  // ✅ Configurable!
    // ...
}

impl PgVectorStorage {
    pub fn new(config: PostgresConfig) -> Self {
        let dimension = 1536; // Default OpenAI embedding dimension
        // ...
    }

    pub fn with_dimension(config: PostgresConfig, dimension: usize) -> Self {
        let mut storage = Self::new(config);
        storage.dimension = dimension;  // ✅ Can override!
        storage
    }
}
```

**Critical Finding:** Dimension is hardcoded in table creation!

```rust
// Line 85-107
async fn create_table(&self) -> Result<()> {
    let sql = format!(
        r#"CREATE TABLE IF NOT EXISTS {} (
            embedding vector({}) NOT NULL,  // ⚠️  FIXED at table creation!
        )#",
        self.table_name, self.dimension
    );
}
```

**Problem:** Changing embedding model requires:

1. Drop existing table
2. Recreate with new dimension
3. Re-embed all documents

### API Layer Provider Selection

**File:** [edgequake/crates/edgequake-api/src/state.rs](../../../../edgequake/crates/edgequake-api/src/state.rs#L315)

```rust
pub fn new_memory(llm_api_key: impl Into<String>) -> Self {
    let llm_provider = Arc::new(OpenAIProvider::new(llm_api_key));
    // ❌ Hardcoded to OpenAI!
}
```

**No environment-based selection!** API always creates OpenAI provider.

## 🔬 Model Research

### Gemma 3 Models (Required by Spec)

**Spec Requirements:**

- Ollama LLM: `gemma3:12b`
- Ollama Embedding: `embeddinggemma:latest`

**Research Needed:**

1. ❓ What is the embedding dimension of `embeddinggemma:latest`?
2. ❓ Is `gemma3:12b` available in Ollama? (might be `gemma2:12b`)
3. ❓ Context length for gemma models?

### LM Studio Defaults (Required by Spec)

**Spec Requirements:**

- LLM: `gemma-3n-e4b-it-mlxmodel`
- Embedding: `text-embedding-ada-002` (768 dimensions - OpenAI)

**Issues:**

- ❌ `gemma-3n-e4b-it-mlxmodel` looks like MLX-optimized (Mac-specific)
- ⚠️ LM Studio serves OpenAI-compatible API, so any model can work
- ✅ `text-embedding-ada-002` is standard (but OpenAI, not local)

## 🎯 Design Decisions

### Decision #1: Provider Factory Pattern

**Create:** `edgequake/crates/edgequake-llm/src/factory.rs`

```rust
pub struct ProviderFactory;

impl ProviderFactory {
    /// Auto-select provider based on environment
    pub fn from_env() -> Result<(
        Arc<dyn LLMProvider>,
        Arc<dyn EmbeddingProvider>
    )> {
        // Check EDGEQUAKE_LLM_PROVIDER or auto-detect
        // Priority: OLLAMA_HOST > OPENAI_API_KEY > Mock
    }

    /// Create specific provider
    pub fn create_ollama(config: OllamaConfig) -> Result<...>
    pub fn create_lmstudio(config: LMStudioConfig) -> Result<...>
    pub fn create_openai(api_key: String) -> Result<...>
}
```

**Rationale:**

- Centralized provider creation
- Environment-based auto-selection
- Easy testing with mock providers

### Decision #2: LM Studio via Alias, Not New Provider

**Do NOT create `LMStudioProvider` struct!**

Instead:

- Use `OpenAIProvider::compatible()`
- Add convenience method: `OllamaProvider::lmstudio()` or factory method
- Document LM Studio as OpenAI-compatible mode

**Rationale:**

- DRY principle (don't duplicate OpenAI logic)
- LM Studio IS OpenAI-compatible
- Less maintenance burden

### Decision #3: Vector Migration Utility

**Create:** `edgequake/crates/edgequake-storage/src/migration.rs`

```rust
pub struct VectorMigration {
    old_storage: Arc<dyn VectorStorage>,
    new_storage: Arc<dyn VectorStorage>,
}

impl VectorMigration {
    /// Detect dimension mismatch
    pub fn needs_migration(&self) -> bool;

    /// Drop and recreate vector tables
    pub async fn recreate_vector_db(&self) -> Result<()>;

    /// Re-embed all documents (requires document storage access)
    pub async fn reembed_documents(...) -> Result<()>;
}
```

**Rationale:**

- Explicit migration process
- Prevents accidental data loss
- Can be triggered via CLI or API

### Decision #4: Configuration Environment Variables

**New Variables:**

```bash
# Provider Selection
EDGEQUAKE_LLM_PROVIDER=ollama|openai|lmstudio|mock

# Ollama Configuration
OLLAMA_HOST=http://localhost:11434
OLLAMA_MODEL=gemma3:12b           # ⚠️  Verify this exists
OLLAMA_EMBEDDING_MODEL=embeddinggemma:latest
OLLAMA_EMBEDDING_DIM=768          # ⚠️  Need to confirm

# LM Studio Configuration
LMSTUDIO_HOST=http://localhost:1234
LMSTUDIO_MODEL=gemma2-9b-it
LMSTUDIO_EMBEDDING_MODEL=text-embedding-ada-002
LMSTUDIO_EMBEDDING_DIM=1536

# OpenAI (existing)
OPENAI_API_KEY=sk-...
OPENAI_BASE_URL=https://api.openai.com/v1
```

## 🗺️ Implementation Roadmap

### Phase 1: Fix Ollama Defaults (Easy Win)

1. Update `DEFAULT_OLLAMA_MODEL` to `gemma3:12b`
2. Update `DEFAULT_OLLAMA_EMBEDDING_MODEL` to `embeddinggemma:latest`
3. Research and set correct embedding dimension

**Files:**

- `edgequake/crates/edgequake-llm/src/providers/ollama.rs`

### Phase 2: Provider Factory (Core Infrastructure)

1. Create `edgequake/crates/edgequake-llm/src/factory.rs`
2. Implement `from_env()` method with provider selection logic
3. Add LM Studio convenience methods
4. Export in `lib.rs`

**Files:**

- `edgequake/crates/edgequake-llm/src/factory.rs` (new)
- `edgequake/crates/edgequake-llm/src/lib.rs` (modify)

### Phase 3: API Integration (Use Factory)

1. Replace hardcoded `OpenAIProvider::new()` in `state.rs`
2. Use `ProviderFactory::from_env()`
3. Pass dimension to vector storage based on embedding model

**Files:**

- `edgequake/crates/edgequake-api/src/state.rs`

### Phase 4: Vector Migration Utility (Safety)

1. Create migration module
2. Add dimension detection
3. Add recreate/reembed methods
4. Add CLI command (optional)

**Files:**

- `edgequake/crates/edgequake-storage/src/migration.rs` (new)
- `edgequake/crates/edgequake-storage/src/lib.rs` (export)

### Phase 5: Testing & Documentation

1. E2E tests with Ollama (if available locally)
2. E2E tests with Mock provider (always)
3. Update configuration docs
4. Add provider switching guide

**Files:**

- `edgequake/crates/edgequake-llm/tests/e2e_provider_factory.rs` (new)
- `docs/0007-configuration-reference.md` (update)
- `docs/0005-llm-integration.md` (update)

## 🚧 Roadblocks Identified

### Roadblock #1: Model Name Verification

**Issue:** Spec requires `gemma3:12b` but Ollama might use `gemma2:12b`

**Solution:** Research Ollama model registry or allow both names

### Roadblock #2: Embedding Dimensions Unknown

**Issue:** Don't know dimension of `embeddinggemma:latest`

**Solution:**

1. Check Ollama documentation
2. Query Ollama API: `curl http://localhost:11434/api/show -d '{"name": "embeddinggemma:latest"}'`
3. Default to 768 (common for embedding models)

### Roadblock #3: LM Studio Model Name

**Issue:** `gemma-3n-e4b-it-mlxmodel` is very specific (MLX format)

**Solution:** Make it configurable, document that any LM Studio model works

## 📊 Complexity Estimates

| Task                | LOC     | Files | Difficulty  | Risk   |
| ------------------- | ------- | ----- | ----------- | ------ |
| Fix Ollama defaults | 10      | 1     | ⭐ Easy     | Low    |
| Provider factory    | 200     | 2     | ⭐⭐ Medium | Low    |
| API integration     | 50      | 1     | ⭐ Easy     | Medium |
| Migration utility   | 150     | 2     | ⭐⭐⭐ Hard | High   |
| Testing             | 300     | 3     | ⭐⭐ Medium | Low    |
| **Total**           | **710** | **9** | ⭐⭐ Medium | Medium |

## 🎯 Success Criteria

1. ✅ Can switch providers via `EDGEQUAKE_LLM_PROVIDER` environment variable
2. ✅ Ollama provider uses spec-compliant defaults
3. ✅ LM Studio works via OpenAI-compatible mode
4. ✅ Vector dimension auto-configured based on embedding model
5. ✅ Migration utility prevents dimension mismatch errors
6. ✅ All tests pass (Mock, OpenAI, Ollama if available)
7. ✅ Documentation updated

## 🔜 Next: Decide Phase

Ready to formulate detailed implementation plan!
