# OODA Loop Iteration #3 - Decide Phase

**Timestamp:** 2025-01-10  
**Status:** ✅ Complete  
**Duration:** 10 minutes

## Decision: Phase 5 E2E Testing Implementation

### Scope for Iteration #3

After analyzing the gaps and risks, I've decided to implement:

✅ **Phase 5A: Provider Auto-Detection Tests** (HIGH PRIORITY)  
✅ **Phase 5B: In-Memory + Ollama Integration** (HIGH PRIORITY)  
✅ **Phase 5C: Dimension Validation Logic** (CRITICAL)  
⏸️ **PostgreSQL Testing** - Defer to Iteration #4 (requires Docker setup)

### Rationale

1. **Provider auto-detection tests** catch environment-based selection bugs
2. **In-Memory tests** validate core functionality without infrastructure complexity
3. **Dimension validation** prevents data corruption (critical safety feature)
4. **PostgreSQL deferred** due to complexity (Docker, migrations, cleanup)

### Implementation Plan

## Phase 5A: E2E Provider Auto-Detection Tests

**File:** `edgequake/crates/edgequake-llm/tests/e2e_provider_factory.rs` (NEW)

**Estimated Time:** 30 minutes

### Test 1: Ollama Auto-Detection
```rust
#[tokio::test]
async fn test_provider_auto_detection_ollama() {
    // Clean environment
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OPENAI_API_KEY");
    
    // Set Ollama host
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    
    // Create providers
    let (llm, embedding) = ProviderFactory::from_env().unwrap();
    
    // Verify Ollama selected
    assert_eq!(llm.name(), "ollama");
    assert_eq!(embedding.name(), "ollama");
    assert_eq!(embedding.dimension(), 768); // embeddinggemma dimension
    
    // Cleanup
    std::env::remove_var("OLLAMA_HOST");
}
```

### Test 2: OpenAI Auto-Detection
```rust
#[tokio::test]
async fn test_provider_auto_detection_openai() {
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OLLAMA_HOST");
    std::env::set_var("OPENAI_API_KEY", "sk-test-key");
    
    let (llm, embedding) = ProviderFactory::from_env().unwrap();
    
    assert_eq!(llm.name(), "openai");
    assert_eq!(embedding.dimension(), 1536);
    
    std::env::remove_var("OPENAI_API_KEY");
}
```

### Test 3: Mock Fallback
```rust
#[tokio::test]
async fn test_provider_auto_detection_mock_fallback() {
    // Clear all provider env vars
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("OPENAI_API_KEY");
    
    let (llm, embedding) = ProviderFactory::from_env().unwrap();
    
    assert_eq!(llm.name(), "mock");
    assert_eq!(embedding.dimension(), 1536);
}
```

### Test 4: Explicit Provider Override
```rust
#[tokio::test]
async fn test_explicit_provider_override() {
    // Set multiple provider env vars
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    std::env::set_var("OPENAI_API_KEY", "sk-test");
    
    // Explicit override should win
    std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
    
    let (llm, _) = ProviderFactory::from_env().unwrap();
    assert_eq!(llm.name(), "mock");
    
    // Cleanup
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("OPENAI_API_KEY");
}
```

**Success Criteria:**
- 4 tests pass
- Tests are isolated (cleanup env vars)
- Clear test names describe behavior

## Phase 5B: In-Memory + Ollama Integration Tests

**File:** `edgequake/crates/edgequake-api/tests/e2e_provider_integration.rs` (NEW)

**Estimated Time:** 45 minutes

### Test 1: AppState with Ollama
```rust
#[tokio::test]
async fn test_appstate_with_ollama() {
    // Check if Ollama is available
    if !is_ollama_available().await {
        eprintln!("Skipping: Ollama not running at localhost:11434");
        return;
    }
    
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    std::env::remove_var("OPENAI_API_KEY");
    
    let state = AppState::new_memory(None::<String>);
    
    // Verify correct provider selected
    assert_eq!(state.llm_provider.name(), "ollama");
    assert_eq!(state.embedding_provider.dimension(), 768);
    
    // Verify storage configured with correct dimension
    assert_eq!(state.vector_storage.dimension(), 768);
    
    std::env::remove_var("OLLAMA_HOST");
}

async fn is_ollama_available() -> bool {
    reqwest::get("http://localhost:11434/api/version")
        .await
        .is_ok()
}
```

### Test 2: Real Embedding Storage
```rust
#[tokio::test]
async fn test_real_embedding_storage() {
    if !is_ollama_available().await {
        return;
    }
    
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    let state = AppState::new_memory(None::<String>);
    
    // Generate real embedding with Ollama
    let text = "artificial intelligence and machine learning";
    let embedding = state.embedding_provider
        .embed(text)
        .await
        .expect("Failed to generate embedding");
    
    // Verify dimension
    assert_eq!(embedding.len(), 768);
    
    // Store in vector storage
    state.vector_storage
        .store("test-doc-1", &embedding, text)
        .await
        .expect("Failed to store embedding");
    
    // Retrieve and verify
    let retrieved = state.vector_storage
        .get("test-doc-1")
        .await
        .expect("Failed to retrieve")
        .expect("Document not found");
    
    assert_eq!(retrieved.len(), 768);
    
    std::env::remove_var("OLLAMA_HOST");
}
```

### Test 3: Similarity Search with Ollama
```rust
#[tokio::test]
async fn test_similarity_search_with_ollama() {
    if !is_ollama_available().await {
        return;
    }
    
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    let state = AppState::new_memory(None::<String>);
    
    // Store multiple documents
    let docs = vec![
        ("doc1", "artificial intelligence and neural networks"),
        ("doc2", "quantum computing and physics"),
        ("doc3", "machine learning algorithms and models"),
    ];
    
    for (id, text) in docs {
        let embedding = state.embedding_provider.embed(text).await.unwrap();
        state.vector_storage.store(id, &embedding, text).await.unwrap();
    }
    
    // Query with similar text
    let query = "deep learning and AI";
    let query_embedding = state.embedding_provider.embed(query).await.unwrap();
    
    let results = state.vector_storage
        .search(&query_embedding, 2)
        .await
        .expect("Search failed");
    
    // Verify relevant results returned
    assert_eq!(results.len(), 2);
    assert!(results[0].id == "doc1" || results[0].id == "doc3");
    
    std::env::remove_var("OLLAMA_HOST");
}
```

**Success Criteria:**
- 3 tests pass when Ollama available
- Tests skip gracefully if Ollama unavailable
- Real embeddings generated and stored

## Phase 5C: Dimension Validation Logic

**Estimated Time:** 1 hour

### Step 1: Add `dimension()` Method to VectorStorage Trait

**File:** `edgequake/crates/edgequake-storage/src/traits.rs`

**Change:**
```rust
#[async_trait]
pub trait VectorStorage: Send + Sync {
    // ... existing methods ...
    
    /// Get the vector dimension this storage was configured with.
    ///
    /// Returns the dimension size used when creating this storage instance.
    fn dimension(&self) -> usize;
}
```

### Step 2: Implement for MemoryVectorStorage

**File:** `edgequake/crates/edgequake-storage/src/adapters/memory.rs`

**Change:**
```rust
impl VectorStorage for MemoryVectorStorage {
    // ... existing methods ...
    
    fn dimension(&self) -> usize {
        self.dimension
    }
}
```

### Step 3: Add Validation in AppState

**File:** `edgequake/crates/edgequake-api/src/state.rs`

**Change in `new_memory()`:**
```rust
pub fn new_memory(llm_api_key: Option<impl Into<String>>) -> Self {
    use edgequake_llm::ProviderFactory;

    if let Some(key) = llm_api_key {
        std::env::set_var("OPENAI_API_KEY", key.into());
    }

    let (llm_provider, embedding_provider) =
        ProviderFactory::from_env().expect("Failed to create LLM provider");

    let embedding_dim = embedding_provider.dimension();
    
    // Log dimension for observability
    tracing::info!(
        "Creating AppState with {}-dimensional embeddings from {} provider",
        embedding_dim,
        std::env::var("EDGEQUAKE_LLM_PROVIDER")
            .unwrap_or_else(|_| "auto-detected".to_string())
    );

    let vector_storage = Arc::new(MemoryVectorStorage::new("default", embedding_dim));
    
    // ... rest of initialization ...
}
```

### Step 4: Add Warning for Dimension Mismatch (Future Enhancement)

**Note:** Full dimension validation requires:
1. Storing dimension metadata in storage
2. Checking on startup
3. Providing migration utility

**Decision:** Log warning for now, implement full validation in Iteration #4

```rust
// Future enhancement location in new_memory():
if let Some(existing_dim) = detect_existing_dimension(&kv_storage).await {
    if existing_dim != embedding_dim {
        tracing::warn!(
            "Dimension mismatch detected: storage has {}-dim vectors, provider generates {}-dim. \
            You may need to recreate the database. See docs/0005-llm-integration.md#provider-switching",
            existing_dim,
            embedding_dim
        );
    }
}
```

### Test for Dimension Detection

**File:** `edgequake/crates/edgequake-api/tests/e2e_dimension_validation.rs` (NEW)

```rust
#[tokio::test]
async fn test_dimension_detection() {
    // Create state with Mock (1536-dim)
    std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
    let state_1536 = AppState::new_memory(None::<String>);
    assert_eq!(state_1536.vector_storage.dimension(), 1536);
    
    // Create state with Ollama (768-dim)
    if is_ollama_available().await {
        std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        
        let state_768 = AppState::new_memory(None::<String>);
        assert_eq!(state_768.vector_storage.dimension(), 768);
    }
}
```

**Success Criteria:**
- `dimension()` method added to trait
- Implemented for all storage backends
- Dimension logged on startup
- 1 test passing

## Time Budget

| Task | Estimated | Buffer | Total |
|------|-----------|--------|-------|
| Phase 5A: Auto-detection tests | 30min | +10min | 40min |
| Phase 5B: In-Memory + Ollama | 45min | +15min | 60min |
| Phase 5C: Dimension validation | 60min | +20min | 80min |
| **Total** | **135min** | **45min** | **180min (3h)** |

## Risk Mitigation

### Risk 1: Ollama Unavailable
**Mitigation:**
- All Ollama tests check availability first
- Skip with clear message if unavailable
- No CI failures due to missing Ollama

### Risk 2: Test Flakiness
**Mitigation:**
- Use deterministic test data
- Clean environment before each test
- Retry logic for network calls (future)

### Risk 3: Breaking Existing Tests
**Mitigation:**
- Run full test suite after each change
- Keep changes minimal and atomic
- Separate test files (no modification of existing)

## Success Criteria

### Must Have (Iteration #3)
- [ ] 4 provider auto-detection tests passing
- [ ] 3 In-Memory + Ollama tests passing (if Ollama available)
- [ ] `dimension()` method added to VectorStorage trait
- [ ] Dimension logged on AppState creation
- [ ] 1 dimension detection test passing
- [ ] All existing tests still passing (44 tests)

### Nice to Have
- [ ] PostgreSQL + Ollama test (defer to #4)
- [ ] Dimension mismatch warning on startup
- [ ] Performance benchmarks

### Success Metrics
- **Test Coverage:** 50+ tests (44 existing + 8 new)
- **E2E Coverage:** 8 integration tests
- **Pass Rate:** 100%
- **Execution Time:** <30 seconds (excluding Ollama calls)

## Next Actions (Ordered)

1. **Create test file:** `edgequake/crates/edgequake-llm/tests/e2e_provider_factory.rs`
2. **Implement Phase 5A tests** (4 tests)
3. **Run tests:** `cargo test --package edgequake-llm`
4. **Create test file:** `edgequake/crates/edgequake-api/tests/e2e_provider_integration.rs`
5. **Implement helper:** `is_ollama_available()` function
6. **Implement Phase 5B tests** (3 tests)
7. **Run tests:** `cargo test --package edgequake-api`
8. **Add `dimension()` to trait:** `edgequake-storage/src/traits.rs`
9. **Implement for Memory:** `edgequake-storage/src/adapters/memory.rs`
10. **Add logging:** `edgequake-api/src/state.rs`
11. **Create test file:** `edgequake/crates/edgequake-api/tests/e2e_dimension_validation.rs`
12. **Run full suite:** `cargo test --workspace`
13. **Commit changes:** Atomic commits per phase
14. **Document findings:** Act phase writeup

**Proceed to:** Act phase → Execute implementation
