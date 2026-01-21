# OODA Loop Iteration #4 - Orient Phase

**Date:** 2025-01-26  
**Mission:** Dimension Validation & Storage Safety  
**Focus:** Add validation and logging for provider/storage dimension matching

---

## Strategic Analysis

### Discovery: Infrastructure Already Exists! ✅

**Good News:** After investigating the codebase, we discovered:

1. ✅ `VectorStorage` trait already has `fn dimension() -> usize` method
2. ✅ `MemoryVectorStorage` already implements dimension tracking correctly
3. ✅ `PgVectorStorage` likely implements dimension tracking (need to verify)
4. ✅ AppState creates storage with provider's dimension

**The Actual Gap:**

- ❌ No **logging** when dimension is initialized
- ❌ No **validation** when switching providers (e.g., from OpenAI to Ollama)
- ❌ No **warning** when dimension changes between sessions

### Revised Problem Statement

**Current Behavior:**

```rust
// Session 1: User uses OpenAI (1536-dim)
std::env::set_var("OPENAI_API_KEY", "sk-...");
let state = AppState::new_memory(None::<String>);
// Creates MemoryVectorStorage with dimension=1536
// ❌ No log message about dimension

// ... user stores 1000 vectors with 1536 dimensions ...

// Session 2: User switches to Ollama (768-dim)
std::env::remove_var("OPENAI_API_KEY");
std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
let state = AppState::new_memory(None::<String>);
// Creates NEW MemoryVectorStorage with dimension=768
// ❌ Old vectors (1536-dim) are LOST (new HashMap created)
// ❌ No warning about dimension change
```

**Key Insight:** MemoryVectorStorage creates a NEW HashMap each time, so there's no "migration" problem for in-memory storage. The vectors are ephemeral and lost when AppState is recreated.

**Real Problem is PostgreSQL:**

```rust
// Session 1: OpenAI provider (1536-dim)
let state = AppState::new_postgres(None::<String>).await?;
// Stores vectors in pgvector table with 1536 dimensions

// Session 2: Switch to Ollama (768-dim)
let state = AppState::new_postgres(None::<String>).await?;
// ❌ pgvector table STILL has 1536-dim vectors
// ❌ Provider expects 768-dim vectors
// ❌ Similarity search will FAIL or give wrong results
```

---

## Architecture Analysis

### Option 1: Panic on Dimension Mismatch (Strict)

**Design:**

```rust
pub fn new_memory(llm_api_key: Option<impl Into<String>>) -> Self {
    let (llm_provider, embedding_provider) = ProviderFactory::from_env()
        .expect("Failed to create LLM provider");

    let provider_dim = embedding_provider.dimension();

    // For memory storage: just create fresh storage with new dimension
    let vector_storage = Arc::new(MemoryVectorStorage::new("default", provider_dim));

    tracing::info!(
        provider = embedding_provider.name(),
        dimension = provider_dim,
        storage = "memory",
        "Vector storage initialized"
    );

    // ... rest of AppState creation ...
}

pub async fn new_postgres(llm_api_key: Option<impl Into<String>>) -> Result<Self> {
    let (llm_provider, embedding_provider) = ProviderFactory::from_env()
        .expect("Failed to create LLM provider");

    let provider_dim = embedding_provider.dimension();

    // Create PostgreSQL storage
    let vector_storage = Arc::new(PgVectorStorage::new(/* ... */).await?);

    // ✅ VALIDATION: Check if existing vectors have different dimension
    if !vector_storage.is_empty().await? {
        let storage_dim = vector_storage.dimension();
        if storage_dim != provider_dim {
            return Err(anyhow::anyhow!(
                "Dimension mismatch: PostgreSQL storage contains {}-dimensional vectors, \
                 but provider '{}' expects {}-dimensional vectors. \
                 \n\nOptions:\
                 \n  1. Switch back to previous provider\
                 \n  2. Clear storage with: psql -c 'TRUNCATE vector_embeddings;'\
                 \n  3. Run migration tool: edgequake migrate-vectors --target-dim {}",
                storage_dim,
                embedding_provider.name(),
                provider_dim,
                provider_dim
            ));
        }
    }

    tracing::info!(
        provider = embedding_provider.name(),
        dimension = provider_dim,
        storage = "postgres",
        "Vector storage validated"
    );

    // ... rest of AppState creation ...
}
```

**Pros:**

- ✅ Safe - prevents silent data corruption
- ✅ Clear error message guides user to solution
- ✅ Fail-fast principle

**Cons:**

- ❌ Breaks existing code if user switches providers
- ❌ Requires manual intervention

---

### Option 2: Warn and Continue (Permissive)

**Design:**

```rust
pub async fn new_postgres(llm_api_key: Option<impl Into<String>>) -> Result<Self> {
    let provider_dim = embedding_provider.dimension();
    let vector_storage = Arc::new(PgVectorStorage::new(/* ... */).await?);

    if !vector_storage.is_empty().await? {
        let storage_dim = vector_storage.dimension();
        if storage_dim != provider_dim {
            tracing::warn!(
                storage_dim,
                provider_dim,
                provider = embedding_provider.name(),
                "⚠️  DIMENSION MISMATCH: Storage has {}-dim vectors, provider expects {}-dim. \
                 Similarity search may produce incorrect results. \
                 Consider clearing storage or switching back to previous provider.",
                storage_dim,
                provider_dim
            );
        }
    }

    // Continue anyway...
}
```

**Pros:**

- ✅ Non-breaking - existing code still works
- ✅ User is warned about potential issues

**Cons:**

- ❌ Silent failures possible if user ignores warnings
- ❌ Incorrect search results may confuse users

---

### Option 3: Auto-Clear Storage (Aggressive)

**Design:**

```rust
pub async fn new_postgres(llm_api_key: Option<impl Into<String>>) -> Result<Self> {
    let provider_dim = embedding_provider.dimension();
    let vector_storage = Arc::new(PgVectorStorage::new(/* ... */).await?);

    if !vector_storage.is_empty().await? {
        let storage_dim = vector_storage.dimension();
        if storage_dim != provider_dim {
            tracing::warn!(
                storage_dim,
                provider_dim,
                "Dimension mismatch detected. Auto-clearing storage..."
            );
            vector_storage.clear().await?;
            tracing::info!("Storage cleared. Ready for {}-dim vectors", provider_dim);
        }
    }
}
```

**Pros:**

- ✅ Automatic recovery
- ✅ No manual intervention needed

**Cons:**

- ❌ **DATA LOSS** without user consent
- ❌ Surprising behavior
- ❌ Violates principle of least surprise

---

## Decision: Option 1 (Strict Validation)

**Rationale:**

1. **Safety First** - Prevent silent data corruption
2. **Clear Guidance** - Error message tells user exactly what to do
3. **Fail-Fast** - Better to fail loudly than produce wrong results
4. **User Control** - User explicitly chooses how to handle mismatch

**Exception:** Memory storage doesn't need validation (ephemeral data, new HashMap each time)

---

## Implementation Strategy

### Phase 6A: Add Dimension Logging (Low Risk)

**Target:** `AppState::new_memory()` and `AppState::new_postgres()`

**Changes:**

1. Add `tracing::info!` after vector storage creation
2. Log provider name, dimension, storage type

**Testing:** Visual inspection of logs when running tests

---

### Phase 6B: Add PostgreSQL Dimension Validation (Critical)

**Target:** `AppState::new_postgres()`

**Changes:**

1. Check if storage is empty with `vector_storage.is_empty().await?`
2. If not empty, compare `storage.dimension()` vs `provider.dimension()`
3. If mismatch, return detailed error with recovery options

**Testing:** E2E test that creates storage with 1536-dim, then tries to init with 768-dim provider

---

### Phase 6C: Verify PgVectorStorage Implements dimension() (Validation)

**Target:** `edgequake-storage/src/adapters/postgres/vector.rs`

**Task:** Read implementation and verify `dimension()` returns correct value

**Risk Mitigation:** If not implemented, add implementation in this iteration

---

## Test Architecture

### Test 1: Dimension Logging (MemoryVectorStorage)

**File:** `edgequake/crates/edgequake-api/tests/e2e_dimension_logging.rs`

**Test Case:**

```rust
#[tokio::test]
#[serial]
async fn test_dimension_logged_on_memory_creation() {
    // Setup tracing subscriber to capture logs
    let (subscriber, handle) = tracing_subscriber::fmt()
        .with_test_writer()
        .finish()
        .with_subscriber(/* ... */);

    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    let _state = AppState::new_memory(None::<String>);

    // Verify log contains: dimension=768, provider=ollama, storage=memory
    let logs = handle.collect();
    assert!(logs.contains("dimension=768"));
    assert!(logs.contains("provider=ollama"));
}
```

---

### Test 2: Dimension Validation (PgVectorStorage)

**File:** `edgequake/crates/edgequake-api/tests/e2e_postgres_dimension_validation.rs`

**Test Case:**

```rust
#[tokio::test]
#[serial]
#[cfg(feature = "postgres")]
async fn test_dimension_mismatch_fails() {
    // Setup: Create PostgreSQL storage with OpenAI dimensions
    std::env::set_var("OPENAI_API_KEY", "sk-test");
    let state1 = AppState::new_postgres(None::<String>).await.unwrap();

    // Store one vector to mark storage as non-empty
    state1.vector_storage.upsert(&[(
        "test".to_string(),
        vec![0.0; 1536],  // 1536-dim vector
        serde_json::json!({}),
    )]).await.unwrap();

    // Switch to Ollama (768-dim)
    std::env::remove_var("OPENAI_API_KEY");
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");

    // Should fail with dimension mismatch error
    let result = AppState::new_postgres(None::<String>).await;
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Dimension mismatch"));
    assert!(err_msg.contains("1536"));
    assert!(err_msg.contains("768"));
}
```

---

## Risk Mitigation

### Risk: PgVectorStorage doesn't implement dimension() correctly

**Mitigation:** In Phase 6C, verify implementation. If broken, fix it atomically with validation logic.

---

### Risk: is_empty() is slow for large databases

**Mitigation:** Use `COUNT(*)` with LIMIT 1 in PostgreSQL. If `is_empty()` is expensive, add a separate `has_vectors()` method that's optimized.

---

### Risk: Breaking change for existing users

**Mitigation:** This is intentional - we're preventing silent data corruption. Add migration guide to documentation.

---

## Success Criteria (Revised)

✅ **SC-1:** Dimension logged when AppState created (info level)  
✅ **SC-2:** PostgreSQL dimension validation implemented  
✅ **SC-3:** Clear error message on dimension mismatch  
✅ **SC-4:** Error includes recovery options (3 choices)  
✅ **SC-5:** Test for dimension logging exists  
✅ **SC-6:** Test for dimension mismatch error exists  
✅ **SC-7:** PgVectorStorage dimension() verified working  
✅ **SC-8:** No regressions (all existing tests pass)  
✅ **SC-9:** Documentation updated with dimension guidance

---

## Work Breakdown (Revised)

### Phase 6A: Dimension Logging (20 minutes)

- Add `tracing::info!` to `AppState::new_memory()`
- Add `tracing::info!` to `AppState::new_postgres()`
- Test manually by running cargo test with RUST_LOG=info

### Phase 6B: PostgreSQL Validation Logic (30 minutes)

- Read PgVectorStorage implementation to understand dimension() method
- Add dimension validation in `AppState::new_postgres()`
- Craft detailed error message with recovery options

### Phase 6C: E2E Tests (40 minutes)

- Write dimension logging test (memory storage)
- Write dimension mismatch test (PostgreSQL storage)
- Both tests use `#[serial]` for isolation

### Phase 6D: Documentation (20 minutes)

- Update docs/0005-llm-integration.md with dimension migration section
- Add troubleshooting guide for dimension mismatch errors

**Total Time:** 110 minutes (1 hour 50 minutes)

---

## Dimension Matrix (Reference)

| Provider  | Embedding Model        | Dimension | Storage Compatibility            |
| --------- | ---------------------- | --------- | -------------------------------- |
| OpenAI    | text-embedding-3-small | 1536      | ✅ Compatible with Mock          |
| Ollama    | embeddinggemma:latest  | 768       | ❌ Incompatible with OpenAI/Mock |
| Mock      | Synthetic              | 1536      | ✅ Compatible with OpenAI        |
| LM Studio | text-embedding-ada-002 | 1536      | ✅ Compatible with OpenAI/Mock   |

**Key Insight:** Ollama (768) is the odd one out. Switching to/from Ollama requires storage rebuild.

---

## Next Steps

**Proceed to Decide Phase:**

- Write detailed implementation plan
- Specify exact code changes with line numbers
- Plan test execution strategy
- Estimate time for each sub-task

**After Decide:**

- Proceed to Act phase
- Implement changes
- Run tests
- Commit atomically

---

**OODA Progress:** 4/50 iterations (8%)  
**Phase Progress:** Iteration #4 - Orient ✅ COMPLETE

---

**End of Orient Phase**
