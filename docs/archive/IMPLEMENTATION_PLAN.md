# Complete Implementation Plan: Advanced Retrieval Features

**Date:** 2025-12-22  
**Target:** Implement all critical and high-priority features  
**Timeline:** Complete in current session

---

## Phase 1: Keyword Extraction (Priority: CRITICAL)

### 1.1 Create Keyword Extraction Module

**File:** `edgequake-query/src/keywords.rs`

**Components:**

- `KeywordExtractor` trait
- `LLMKeywordExtractor` implementation
- `MockKeywordExtractor` for testing
- `KeywordCache` wrapper

**API Design:**

```rust
pub struct Keywords {
    pub high_level: Vec<String>,  // Concepts, themes
    pub low_level: Vec<String>,   // Entities, specific terms
}

#[async_trait]
pub trait KeywordExtractor: Send + Sync {
    async fn extract(&self, query: &str) -> Result<Keywords>;
}
```

### 1.2 Integration Points

- Add to `QueryEngineConfig`
- Use in all query strategies (Local uses low_level, Global uses high_level)
- Cache extracted keywords

---

## Phase 2: Token-Based Truncation (Priority: CRITICAL)

### 2.1 Create Tokenizer Module

**File:** `edgequake-query/src/tokenizer.rs`

**Components:**

- `Tokenizer` trait
- `TiktokenTokenizer` implementation (using tiktoken-rs)
- `MockTokenizer` for testing

**API Design:**

```rust
#[async_trait]
pub trait Tokenizer: Send + Sync {
    fn encode(&self, text: &str) -> Vec<u32>;
    fn decode(&self, tokens: &[u32]) -> String;
    fn count_tokens(&self, text: &str) -> usize;
}
```

### 2.2 Truncation System

**File:** `edgequake-query/src/truncation.rs`

**Functions:**

- `truncate_entities(entities, max_tokens, tokenizer)`
- `truncate_relationships(rels, max_tokens, tokenizer)`
- `balance_context(entities, rels, chunks, max_total_tokens, tokenizer)`

### 2.3 Configuration

Add to `QueryEngineConfig`:

- `max_entity_tokens: usize` (default: 8000)
- `max_relation_tokens: usize` (default: 8000)
- `max_total_tokens: usize` (default: 16000)

---

## Phase 3: Chunk Retrieval from Entities (Priority: HIGH)

### 3.1 Chunk Retrieval Module

**File:** `edgequake-query/src/chunk_retrieval.rs`

**Components:**

- `ChunkRetrievalMethod` enum (Weight, Vector)
- `retrieve_chunks_from_entities()`
- `retrieve_chunks_from_relationships()`
- `weighted_polling_selection()`
- `vector_similarity_ranking()`

**API Design:**

```rust
pub enum ChunkRetrievalMethod {
    Weight,  // Frequency-based
    Vector,  // Similarity-based
}

pub async fn retrieve_chunks_from_entities(
    entities: &[RetrievedEntity],
    kv_storage: &Arc<dyn KVStorage>,
    method: ChunkRetrievalMethod,
    query_embedding: Option<&[f32]>,
    max_chunks: usize,
) -> Result<Vec<RetrievedChunk>>
```

### 3.2 Integration

- Update `LocalStrategy` to retrieve chunks from entity source_ids
- Update `GlobalStrategy` to retrieve chunks from relationship source_ids
- Add `related_chunk_number` to config

---

## Phase 4: Integration & Testing

### 4.1 Update Query Engine

**File:** `edgequake-query/src/engine.rs`

**Changes:**

- Add keyword extractor
- Add tokenizer
- Apply truncation before LLM call
- Pass chunk retrieval method to strategies

### 4.2 Update All Strategies

**Files:** `edgequake-query/src/strategies.rs`

**Changes for Each Strategy:**

- `LocalStrategy`: Extract low-level keywords, retrieve chunks from entities
- `GlobalStrategy`: Extract high-level keywords, retrieve chunks from relationships
- `HybridStrategy`: Use both keyword types, merge chunks
- `MixStrategy`: Combine all features

### 4.3 Comprehensive Tests

**Files:**

- `edgequake-query/src/keywords/tests.rs`
- `edgequake-query/src/tokenizer/tests.rs`
- `edgequake-query/src/truncation/tests.rs`
- `edgequake-query/src/chunk_retrieval/tests.rs`
- `edgequake-core/tests/e2e_advanced_features.rs`

---

## Phase 5: Documentation & Finalization

### 5.1 Update Documentation

- API documentation for new modules
- Configuration guide updates
- Migration guide for existing users

### 5.2 Validation

- Run all 233+ existing tests
- Run new feature tests
- Validate with real LLM provider (optional)

---

## Implementation Order

1. ✅ Create plan (current)
2. ⏳ Keyword extraction (30-40 min)
3. ⏳ Token truncation (20-30 min)
4. ⏳ Chunk retrieval (30-40 min)
5. ⏳ Integration (20-30 min)
6. ⏳ Testing (20-30 min)
7. ⏳ Validation & commit (10 min)

**Total Estimated Time:** 2.5-3 hours

---

## Success Criteria

- [ ] All 233+ existing tests pass
- [ ] New feature tests pass (100% coverage)
- [ ] E2E tests demonstrate new features
- [ ] Documentation complete
- [ ] Code committed and pushed
- [ ] No breaking changes to existing API

---

## Dependencies to Add

```toml
# Cargo.toml for edgequake-query
tiktoken-rs = "0.5"  # For tokenization
```

---

Let's execute this plan step by step!
