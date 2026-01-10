# OODA Loop Iteration #4 - Observe Phase

**Date:** 2025-01-26  
**Mission:** Full execution of SPEC-032 Ollama/LM Studio provider support (50 OODA loops)  
**Focus:** Dimension Validation & Storage Safety  
**Previous Status:** Iteration #3 complete (10 E2E tests passing)

---

## Mission Context

We're implementing explicit Ollama and LM Studio provider support for EdgeQuake RAG framework. This is Iteration #4 of minimum 50 OODA loops.

**Primary Objectives (from SPEC-032):**
1. ✅ Ollama provider with gemma3:12b + embeddinggemma:latest defaults
2. ✅ LM Studio provider via OpenAI-compatible API
3. ✅ Easy switching between providers (OpenAI, Ollama, LM Studio)
4. ⏳ Vector database recreation mechanism for dimension changes
5. ⏳ Test PostgreSQL AND In-Memory storage backends
6. ⏳ Test WebUI integration for API compatibility
7. ✅ Non-regression (North Star)

**Current Progress:** 3/50 iterations (6%)

---

## Current State Assessment

### What Works ✅

**Provider Infrastructure:**
- ✅ OllamaProvider with correct defaults (gemma3:12b, embeddinggemma:latest)
- ✅ ProviderFactory with environment-based auto-detection
- ✅ Priority chain: EDGEQUAKE_LLM_PROVIDER > OLLAMA_HOST > OPENAI_API_KEY > Mock
- ✅ AppState integration with ProviderFactory

**Test Coverage:**
- ✅ 7 ProviderFactory E2E tests (auto-detection, priority chain, dimension detection)
- ✅ 3 AppState integration tests (configuration validation)
- ✅ Total: 10 new E2E tests (100% passing)
- ✅ 54 total tests passing workspace-wide

**Documentation:**
- ✅ 430+ lines of user-facing documentation (configuration + integration guides)
- ✅ 1,437 lines of OODA loop documentation (Iterations #1-#3)

### What's Missing ⚠️

**Critical Safety Gap: No Dimension Validation**

**Problem Statement:**
When a user switches from OpenAI (1536-dim) to Ollama (768-dim), or vice versa, the existing vector storage contains embeddings of the wrong dimension. This causes:

1. **Silent Failures** - No error when storing mismatched dimensions
2. **Incorrect Search Results** - Similarity search compares incompatible vectors
3. **Data Corruption Risk** - Mixed-dimension vectors in same storage
4. **Poor User Experience** - No guidance on how to migrate

**Current Behavior (UNSAFE):**
```rust
// User starts with OpenAI (1536-dim)
std::env::set_var("OPENAI_API_KEY", "sk-...");
let state = AppState::new_memory(None::<String>);
// Stores vectors with 1536 dimensions

// Later, user switches to Ollama (768-dim)
std::env::remove_var("OPENAI_API_KEY");
std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
let state = AppState::new_memory(None::<String>);
// Now stores vectors with 768 dimensions
// ❌ No warning about dimension mismatch
// ❌ Old 1536-dim vectors still in storage
// ❌ Similarity search produces garbage results
```

**Expected Behavior (SAFE):**
```rust
// Scenario 1: Clean storage (no existing vectors)
let state = AppState::new_memory(None::<String>);
// ✅ Logs: "Initialized vector storage with 768 dimensions"

// Scenario 2: Storage has existing vectors with different dimension
let state = AppState::new_memory(None::<String>);
// ✅ Detects: Storage has 1536-dim vectors, provider expects 768
// ✅ Error: "Dimension mismatch: storage=1536, provider=768. Run migration tool."
// ✅ Provides: Migration command or auto-migration option
```

---

## Gap Analysis

### Gap #1: VectorStorage Trait Missing dimension() Method

**Current Trait:**
```rust
#[async_trait]
pub trait VectorStorage: Send + Sync {
    async fn store_vector(&self, id: String, vector: Vec<f32>) -> Result<()>;
    async fn get_vector(&self, id: &str) -> Result<Option<Vec<f32>>>;
    async fn similarity_search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>>;
    // ❌ No way to query storage dimension
}
```

**Required Addition:**
```rust
#[async_trait]
pub trait VectorStorage: Send + Sync {
    // ... existing methods ...
    
    /// Get the embedding dimension configured for this storage.
    /// Returns None if storage is empty (no dimension configured yet).
    fn dimension(&self) -> Option<usize>;
}
```

**Why This Design:**
- `fn` not `async fn` - Dimension is metadata, not I/O operation
- `Option<usize>` - Storage may be empty (no vectors stored yet)
- Call is cheap - Just returns a field value

---

### Gap #2: In-Memory Storage Doesn't Track Dimension

**Current Implementation:**
```rust
pub struct MemoryVectorStorage {
    vectors: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    namespace: String,
    // ❌ No dimension field
}

impl MemoryVectorStorage {
    pub fn new(namespace: &str, _dimension: usize) -> Self {
        // dimension parameter is ignored!
        Self {
            vectors: Arc::new(RwLock::new(HashMap::new())),
            namespace: namespace.to_string(),
        }
    }
}
```

**Required Changes:**
1. Add `dimension: usize` field to struct
2. Store dimension in constructor
3. Implement `dimension()` trait method

---

### Gap #3: PostgreSQL Storage Doesn't Expose Dimension

**Assumption:** PgVectorStorage likely has dimension in schema but doesn't expose it via trait.

**Required Investigation:**
1. Check if `pgvector` table schema stores dimension
2. Add SQL query to retrieve dimension from metadata
3. Implement `dimension()` trait method

---

### Gap #4: AppState Doesn't Validate Dimension Match

**Current AppState::new_memory():**
```rust
pub fn new_memory(llm_api_key: Option<impl Into<String>>) -> Self {
    let (llm_provider, embedding_provider) = ProviderFactory::from_env()
        .expect("Failed to create LLM provider");
    
    let embedding_dim = embedding_provider.dimension();
    let vector_storage = Arc::new(MemoryVectorStorage::new("default", embedding_dim));
    
    // ❌ No validation that storage dimension matches provider
    // ❌ No warning if dimension changes
    // ❌ No migration guidance
    
    // ... create other components ...
}
```

**Required Validation Logic:**
```rust
pub fn new_memory(llm_api_key: Option<impl Into<String>>) -> Self {
    let (llm_provider, embedding_provider) = ProviderFactory::from_env()
        .expect("Failed to create LLM provider");
    
    let provider_dim = embedding_provider.dimension();
    let vector_storage = Arc::new(MemoryVectorStorage::new("default", provider_dim));
    
    // ✅ Validation: Check if storage already has different dimension
    if let Some(storage_dim) = vector_storage.dimension() {
        if storage_dim != provider_dim {
            panic!(
                "Dimension mismatch: storage has {}-dim vectors, \
                 provider expects {}-dim. Rebuild vector storage or \
                 switch back to previous provider.",
                storage_dim, provider_dim
            );
        }
    }
    
    // ✅ Logging: Record dimension for debugging
    tracing::info!(
        provider = embedding_provider.name(),
        dimension = provider_dim,
        "Vector storage initialized"
    );
    
    // ... create other components ...
}
```

---

## Technical Reconnaissance

### File Locations

**Storage Trait:**
- `edgequake/crates/edgequake-storage/src/traits.rs`
- Line ~20-50 (estimated)
- Need to add `dimension()` method

**In-Memory Storage:**
- `edgequake/crates/edgequake-storage/src/adapters/memory.rs`
- MemoryVectorStorage struct
- Need to add `dimension: usize` field and implement trait method

**PostgreSQL Storage:**
- `edgequake/crates/edgequake-storage/src/adapters/postgres.rs` (if exists)
- OR `edgequake/crates/edgequake-storage/src/postgres/` directory
- Need to query dimension from schema

**AppState:**
- `edgequake/crates/edgequake-api/src/state.rs`
- Lines 323-360 (new_memory function)
- Lines 520-560 (new_postgres function, estimated)
- Need to add validation logic

---

## Risk Assessment

### Risk #1: Dimension Mismatch → Data Corruption

**Severity:** CRITICAL  
**Likelihood:** HIGH (users will switch providers)  
**Impact:** Incorrect search results, user frustration, data loss

**Current Mitigation:** NONE  
**Required Mitigation:** Dimension validation + clear error messages

---

### Risk #2: Breaking Change to VectorStorage Trait

**Severity:** MEDIUM  
**Likelihood:** CERTAIN (we're modifying public trait)  
**Impact:** Downstream code must implement new trait method

**Mitigation Strategy:**
1. Add default implementation if possible (not possible for trait methods in Rust)
2. OR: Provide blanket implementation for common types
3. Document breaking change in commit message
4. Update all implementations in same commit (atomic change)

**Implementations to Update:**
- MemoryVectorStorage ✅ (in this iteration)
- PgVectorStorage ✅ (in this iteration)
- MockVectorStorage ⚠️ (if exists)

---

### Risk #3: Empty Storage Has No Dimension

**Scenario:** User creates fresh AppState, no vectors stored yet. What dimension should storage.dimension() return?

**Options:**
1. **Return `None`** - Storage is empty, no dimension configured
2. **Return `Some(provider_dim)`** - Use provider's dimension
3. **Store dimension in constructor** - Dimension is metadata, not data

**Chosen Design:** Option 3 (Store dimension in constructor)

**Rationale:**
- Dimension is determined at storage creation time (from provider)
- Storage should always know its dimension (even if empty)
- Simplifies validation logic (no Option unwrapping)

**Revised Trait Method:**
```rust
fn dimension(&self) -> usize;  // NOT Option<usize>
```

---

## Test Strategy

### Test #1: MemoryVectorStorage Dimension Tracking

**File:** `edgequake/crates/edgequake-storage/tests/test_memory_dimension.rs`

**Test Cases:**
1. `test_memory_storage_dimension_from_constructor`
   - Create storage with dimension=768
   - Assert `storage.dimension() == 768`

2. `test_memory_storage_dimension_persistence`
   - Create storage with dimension=1536
   - Store 10 vectors
   - Assert `storage.dimension() == 1536` (unchanged)

---

### Test #2: Dimension Validation in AppState

**File:** `edgequake/crates/edgequake-api/tests/e2e_dimension_validation.rs`

**Test Cases:**
1. `test_appstate_dimension_match_success`
   - Set OLLAMA_HOST (768-dim)
   - Create AppState
   - Assert no panic
   - Assert dimension logged

2. `test_appstate_dimension_mismatch_panic`
   - Mock storage with 1536-dim vectors
   - Set OLLAMA_HOST (768-dim)
   - Create AppState
   - Assert panic with clear error message

3. `test_appstate_fresh_storage_no_panic`
   - Empty storage
   - Set OLLAMA_HOST
   - Create AppState
   - Assert no panic (fresh storage, no mismatch)

---

### Test #3: PgVectorStorage Dimension Query

**File:** `edgequake/crates/edgequake-storage/tests/test_postgres_dimension.rs`

**Test Cases:**
1. `test_postgres_dimension_from_schema`
   - Create pgvector table with dimension=768
   - Query dimension via trait method
   - Assert correct dimension returned

2. `test_postgres_empty_table_dimension`
   - Create empty pgvector table
   - Query dimension
   - Assert dimension matches schema (not data count)

---

## Success Criteria

✅ **SC-1:** VectorStorage trait has `dimension()` method  
✅ **SC-2:** MemoryVectorStorage implements `dimension()` correctly  
✅ **SC-3:** PgVectorStorage implements `dimension()` correctly  
✅ **SC-4:** AppState::new_memory validates dimension match  
✅ **SC-5:** AppState::new_postgres validates dimension match  
✅ **SC-6:** Clear error message on dimension mismatch  
✅ **SC-7:** Dimension logged on successful initialization  
✅ **SC-8:** All tests passing (workspace-wide)  
✅ **SC-9:** No regressions (existing tests still pass)  
✅ **SC-10:** Code committed with atomic changes

---

## Estimated Work Breakdown

### Phase 6A: Trait Definition Update (20 minutes)
- Read `edgequake-storage/src/traits.rs`
- Add `dimension()` method to VectorStorage trait
- Document method purpose and return semantics

### Phase 6B: MemoryVectorStorage Implementation (30 minutes)
- Update struct to include `dimension: usize` field
- Modify constructor to store dimension
- Implement `dimension()` trait method
- Write 2 unit tests

### Phase 6C: PgVectorStorage Implementation (40 minutes)
- Locate PgVectorStorage implementation
- Add dimension query logic (SQL or schema inspection)
- Implement `dimension()` trait method
- Write 2 unit tests (requires test DB setup)

### Phase 6D: AppState Validation Logic (30 minutes)
- Update `AppState::new_memory()` with validation
- Update `AppState::new_postgres()` with validation
- Add `tracing::info!` logging
- Write 3 E2E tests

### Phase 6E: Integration Testing (20 minutes)
- Run full workspace test suite
- Verify no regressions
- Fix any compilation errors
- Validate dimension mismatch detection

**Total Estimated Time:** 140 minutes (2 hours 20 minutes)

---

## Dependencies & Blockers

### Dependencies
- ✅ ProviderFactory implemented (Iteration #2)
- ✅ AppState uses ProviderFactory (Iteration #3)
- ✅ Test infrastructure in place (serial_test, etc.)

### Potential Blockers
1. **PgVectorStorage may not exist yet** - Need to check if PostgreSQL adapter is implemented
   - Fallback: Implement only for MemoryVectorStorage, add TODO for PostgreSQL
2. **Dimension stored in pgvector schema?** - Need to verify table structure
   - Fallback: Add dimension to metadata table if not in schema

### Assumptions
- MemoryVectorStorage exists and can be modified
- Storage trait is in `edgequake-storage` crate
- AppState has access to storage dimension after creation

---

## Observation Conclusion

**Critical Gap Identified:** No dimension validation between provider and storage. This is a **critical safety issue** that can cause silent data corruption.

**Recommended Action:** Proceed to Orient phase to design dimension validation architecture.

**Confidence Level:** HIGH - Problem is well-understood, solution is clear, time estimates are conservative.

**Next Phase:** Orient - Design dimension validation architecture and implementation strategy.

---

**OODA Progress:** 4/50 iterations (8%)  
**Phase Progress:** Iteration #4 - Observe ✅ COMPLETE

---

**End of Observe Phase**
