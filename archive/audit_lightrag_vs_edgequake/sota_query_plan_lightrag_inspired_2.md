# SOTA Query Plan: LightRAG-Inspired Implementation v2

> **Date:** 2025-12-31  
> **Status:** 🚀 ACTIVE IMPLEMENTATION  
> **Goal:** Achieve 100%+ feature parity with LightRAG's query engine

---

## Executive Summary

This document provides a **precision implementation plan** for achieving SOTA GraphRAG-style retrieval in EdgeQuake. Based on the deep code audit (16-deep-query-code-audit.md) and roadmap (17-sota-implementation-roadmap.md), we will implement:

1. **LLM-based Keyword Extraction** with caching
2. **Separate Entity/Relationship/Chunk Vector DBs**
3. **Source ID Tracking** for chunk-entity provenance
4. **Enhanced Query Engine** with proper mode implementations
5. **Dynamic Token Budgeting** with priority-based allocation
6. **Query Caching** with invalidation
7. **Reranking Implementation** (Cohere + local cross-encoder)
8. **Comprehensive Tests** to prove parity

---

## Critical Gap Analysis

| Feature            | LightRAG                      | EdgeQuake Current  | EdgeQuake Target     |
| ------------------ | ----------------------------- | ------------------ | -------------------- |
| Keyword Extraction | LLM + cache                   | Word-split mock    | LLM + Redis/PG cache |
| Entity VDB         | Dedicated `entities_vdb`      | Unified            | Dedicated table      |
| Relationship VDB   | Dedicated `relationships_vdb` | Unified            | Dedicated table      |
| Chunk VDB          | Dedicated `chunks_vdb`        | Unified            | Dedicated table      |
| Source ID Linking  | Full `source_id` tracking     | Placeholder        | Full provenance      |
| Reranking          | Cohere/OpenAI/Custom          | API stub only      | Cohere + local       |
| Query Caching      | Hash-based LLM cache          | None               | Multi-level cache    |
| Token Truncation   | Dynamic per-type              | Fixed proportional | Priority-based       |

---

## Phase 1: LLM Keyword Extraction (Critical)

### 1.1 Implementation Files

```
edgequake-query/src/
├── keywords/
│   ├── mod.rs              # Module exports
│   ├── extractor.rs        # KeywordExtractor trait + impl
│   ├── llm_extractor.rs    # LLM-based extraction
│   ├── cache.rs            # Keyword caching
│   └── intent.rs           # Query intent classification
```

### 1.2 Data Structures

```rust
/// Query intent for adaptive retrieval
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum QueryIntent {
    Factual,      // "What is X?"
    Relational,   // "How does X relate to Y?"
    Exploratory,  // "Tell me about X"
    Comparative,  // "Compare X and Y"
    Procedural,   // "How to do X?"
}

/// Extracted keywords with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedKeywords {
    pub high_level: Vec<String>,
    pub low_level: Vec<String>,
    pub query_intent: QueryIntent,
    pub cache_key: String,
    pub extracted_at: chrono::DateTime<chrono::Utc>,
}
```

### 1.3 LLM Prompt (Matching LightRAG)

```
Extract high-level and low-level keywords from the following query.

High-level keywords: Abstract concepts, themes, topics (used for Global mode search)
Low-level keywords: Specific entities, technical terms, proper nouns (used for Local mode search)

Query: "{query}"

Respond with JSON:
{
  "high_level_keywords": ["concept1", "concept2"],
  "low_level_keywords": ["entity1", "entity2"],
  "query_intent": "factual|relational|exploratory|comparative|procedural"
}
```

### 1.4 Caching Strategy

- **Cache Key**: SHA256(query + mode + tenant_id)
- **Storage**: PostgreSQL `eq_keyword_cache` table
- **TTL**: 24 hours (configurable)
- **Invalidation**: On document update affecting entities

### 1.5 Tests

```rust
#[tokio::test]
async fn test_llm_keyword_extraction_basic() {
    // Test with mock LLM provider
}

#[tokio::test]
async fn test_keyword_cache_hit() {
    // Test cache hit scenario
}

#[tokio::test]
async fn test_keyword_cache_miss() {
    // Test cache miss + compute + store
}

#[tokio::test]
async fn test_query_intent_classification() {
    // Test all intent types
}
```

---

## Phase 2: Separate Vector DBs

### 2.1 Schema Changes

```sql
-- New tables for semantic separation
CREATE TABLE IF NOT EXISTS eq_{prefix}_entity_vectors (
    id TEXT PRIMARY KEY,
    entity_name TEXT NOT NULL,
    entity_type TEXT,
    embedding vector(1536) NOT NULL,
    metadata JSONB DEFAULT '{}',
    tenant_id TEXT,
    workspace_id TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS eq_{prefix}_relationship_vectors (
    id TEXT PRIMARY KEY,
    source_entity TEXT NOT NULL,
    target_entity TEXT NOT NULL,
    relation_type TEXT,
    embedding vector(1536) NOT NULL,
    metadata JSONB DEFAULT '{}',
    tenant_id TEXT,
    workspace_id TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS eq_{prefix}_chunk_vectors (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    chunk_index INTEGER,
    embedding vector(1536) NOT NULL,
    content TEXT,
    metadata JSONB DEFAULT '{}',
    tenant_id TEXT,
    workspace_id TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### 2.2 Index Strategy

```sql
-- Entity vectors: HNSW for fast nearest neighbor
CREATE INDEX idx_entity_vectors_hnsw
ON eq_{prefix}_entity_vectors
USING hnsw (embedding vector_cosine_ops);

-- Relationship vectors: HNSW
CREATE INDEX idx_rel_vectors_hnsw
ON eq_{prefix}_relationship_vectors
USING hnsw (embedding vector_cosine_ops);

-- Chunk vectors: HNSW
CREATE INDEX idx_chunk_vectors_hnsw
ON eq_{prefix}_chunk_vectors
USING hnsw (embedding vector_cosine_ops);

-- Tenant/workspace filtering
CREATE INDEX idx_entity_vectors_tenant
ON eq_{prefix}_entity_vectors (tenant_id, workspace_id);
```

### 2.3 Implementation

```rust
/// Query-specific vector stores (LightRAG pattern)
pub struct QueryVectorStores {
    pub entities: Arc<dyn VectorStorage>,
    pub relationships: Arc<dyn VectorStorage>,
    pub chunks: Arc<dyn VectorStorage>,
}

impl QueryVectorStores {
    pub async fn search_by_mode(
        &self,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        mode: QueryMode,
        top_k: usize,
    ) -> Result<ModeSearchResults> {
        match mode {
            QueryMode::Local => {
                // Search entities with LOW-level keywords embedding
                let entities = self.entities
                    .query(&embeddings.low_level, top_k, None)
                    .await?;
                Ok(ModeSearchResults::local(entities))
            }
            QueryMode::Global => {
                // Search relationships with HIGH-level keywords embedding
                let relationships = self.relationships
                    .query(&embeddings.high_level, top_k, None)
                    .await?;
                Ok(ModeSearchResults::global(relationships))
            }
            QueryMode::Hybrid => {
                // Run local + global in parallel
                let (local, global) = tokio::join!(
                    self.search_by_mode(keywords, embeddings, QueryMode::Local, top_k / 2),
                    self.search_by_mode(keywords, embeddings, QueryMode::Global, top_k / 2),
                );
                Ok(ModeSearchResults::hybrid(local?, global?))
            }
            QueryMode::Mix => {
                // Hybrid + direct chunk search
                let (hybrid, chunks) = tokio::join!(
                    self.search_by_mode(keywords, embeddings, QueryMode::Hybrid, top_k / 2),
                    self.chunks.query(&embeddings.query, top_k / 2, None),
                );
                Ok(ModeSearchResults::mix(hybrid?, chunks?))
            }
            QueryMode::Naive => {
                // Direct chunk search only
                let chunks = self.chunks
                    .query(&embeddings.query, top_k, None)
                    .await?;
                Ok(ModeSearchResults::naive(chunks))
            }
        }
    }
}
```

### 2.4 Tests

```rust
#[tokio::test]
async fn test_entity_vector_search() {}

#[tokio::test]
async fn test_relationship_vector_search() {}

#[tokio::test]
async fn test_chunk_vector_search() {}

#[tokio::test]
async fn test_hybrid_mode_parallel() {}

#[tokio::test]
async fn test_mix_mode_with_chunks() {}
```

---

## Phase 3: Source ID Tracking

### 3.1 Schema Changes

```sql
-- Add source_ids to entity nodes (AGE properties)
-- stored as JSONB array in properties column

-- Example property structure:
{
  "node_id": "SARAH_CHEN",
  "entity_type": "PERSON",
  "description": "Lead researcher...",
  "source_ids": [
    {
      "chunk_id": "doc1_chunk_3",
      "document_id": "doc1",
      "file_path": "research.pdf",
      "char_offset": 1500,
      "char_length": 200
    }
  ]
}
```

### 3.2 Pipeline Changes

```rust
/// Source reference for provenance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReference {
    pub chunk_id: String,
    pub document_id: String,
    pub file_path: String,
    pub char_offset: usize,
    pub char_length: usize,
}

/// Extended extracted entity with provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntityWithProvenance {
    pub name: String,
    pub entity_type: String,
    pub description: String,
    pub source_refs: Vec<SourceReference>,
}
```

### 3.3 Chunk Retrieval from Entities (LightRAG Pattern)

```rust
pub struct ChunkGraphLinker {
    graph_storage: Arc<dyn GraphStorage>,
    kv_storage: Arc<dyn KVStorage>,
}

impl ChunkGraphLinker {
    /// Find chunks linked to retrieved entities via source_ids
    pub async fn chunks_from_entities(
        &self,
        entities: &[RetrievedEntity],
        method: ChunkSelectionMethod,
        max_chunks: usize,
    ) -> Result<Vec<LinkedChunk>> {
        // Step 1: Collect all source_ids from entities
        let mut chunk_frequency: HashMap<String, usize> = HashMap::new();

        for entity in entities {
            if let Some(source_ids) = &entity.source_ids {
                for source in source_ids {
                    *chunk_frequency.entry(source.chunk_id.clone()).or_insert(0) += 1;
                }
            }
        }

        // Step 2: Select chunks by method (WEIGHT or VECTOR)
        let selected = match method {
            ChunkSelectionMethod::Weight => {
                // Sort by entity overlap frequency
                let mut sorted: Vec<_> = chunk_frequency.into_iter().collect();
                sorted.sort_by(|a, b| b.1.cmp(&a.1));
                sorted.into_iter().take(max_chunks).map(|(id, _)| id).collect()
            }
            ChunkSelectionMethod::Vector => {
                // Re-rank by vector similarity
                // Implementation needs query embedding
                unimplemented!()
            }
        };

        // Step 3: Batch retrieve chunk content
        let chunks = self.kv_storage.get_batch(&selected).await?;
        Ok(chunks)
    }
}
```

---

## Phase 4: Query Engine Refactor

### 4.1 New Query Pipeline

```rust
pub struct SOTAQueryEngine {
    config: QueryEngineConfig,

    // Storage layers
    vector_stores: QueryVectorStores,
    graph_storage: Arc<dyn GraphStorage>,
    kv_storage: Arc<dyn KVStorage>,

    // Providers
    embedding_provider: Arc<dyn EmbeddingProvider>,
    llm_provider: Arc<dyn LLMProvider>,

    // Query components
    keyword_extractor: Arc<dyn KeywordExtractor>,
    chunk_linker: ChunkGraphLinker,
    reranker: Option<Arc<dyn Reranker>>,
    cache: QueryCache,

    // Tokenizer
    tokenizer: Arc<dyn Tokenizer>,
}

impl SOTAQueryEngine {
    pub async fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        let mode = request.mode.unwrap_or(self.config.default_mode);

        // 1. Check cache
        let cache_key = self.cache.compute_key(&request);
        if let Some(cached) = self.cache.get(&cache_key).await? {
            return Ok(cached);
        }

        // 2. Extract keywords (with caching)
        let keywords = self.keyword_extractor.extract(&request.query).await?;

        // 3. Compute query embeddings (query + high-level + low-level)
        let embeddings = self.compute_embeddings(&request.query, &keywords).await?;

        // 4. Mode-specific retrieval
        let raw_results = self.vector_stores
            .search_by_mode(&keywords, &embeddings, mode, self.config.max_results)
            .await?;

        // 5. Fetch entities from graph (batch operation)
        let entity_ids: Vec<String> = raw_results.entity_ids();
        let (nodes, degrees, edges) = tokio::join!(
            self.graph_storage.get_nodes_batch(&entity_ids),
            self.graph_storage.node_degrees_batch(&entity_ids),
            self.graph_storage.get_edges_for_nodes_batch(&entity_ids),
        );

        // 6. Retrieve linked chunks via source_ids
        let linked_chunks = self.chunk_linker
            .chunks_from_entities(&entities, ChunkSelectionMethod::Weight, self.config.max_chunks)
            .await?;

        // 7. Optional reranking
        let context = if let Some(reranker) = &self.reranker {
            self.rerank_context(reranker, &request.query, raw_context).await?
        } else {
            raw_context
        };

        // 8. Token budget allocation
        let truncated = self.apply_token_budget(context).await?;

        // 9. Generate response (if not context_only)
        let response = if request.context_only {
            QueryResponse::context_only(truncated, mode)
        } else {
            let answer = self.generate_answer(&request.query, &truncated).await?;
            QueryResponse::with_answer(answer, truncated, mode)
        };

        // 10. Cache result
        self.cache.set(&cache_key, &response).await?;

        Ok(response)
    }
}
```

### 4.2 Proper Mode Implementations

```rust
impl SOTAQueryEngine {
    /// Local mode: Entity-centric search with low-level keywords
    async fn query_local(&self, keywords: &ExtractedKeywords, embeddings: &QueryEmbeddings) -> Result<QueryContext> {
        // 1. Search entity VDB with LOW-level keywords embedding
        let entity_results = self.vector_stores.entities
            .query(&embeddings.low_level, self.config.max_entities, None)
            .await?;

        // 2. Get entity data with degrees (batch)
        let entity_ids: Vec<String> = entity_results.iter().map(|r| r.id.clone()).collect();
        let (nodes, degrees) = tokio::join!(
            self.graph_storage.get_nodes_batch(&entity_ids),
            self.graph_storage.node_degrees_batch(&entity_ids),
        );

        // 3. Get edges connecting these entities (batch)
        let edges = self.graph_storage.get_edges_for_nodes_batch(&entity_ids).await?;

        // 4. Get chunks linked to entities via source_ids
        let chunks = self.chunk_linker.chunks_from_entities(&entities).await?;

        Ok(QueryContext::from_local(entities, edges, chunks))
    }

    /// Global mode: Relationship-centric search with high-level keywords
    async fn query_global(&self, keywords: &ExtractedKeywords, embeddings: &QueryEmbeddings) -> Result<QueryContext> {
        // 1. Search relationship VDB with HIGH-level keywords embedding
        let rel_results = self.vector_stores.relationships
            .query(&embeddings.high_level, self.config.max_relationships, None)
            .await?;

        // 2. Extract entity IDs from relationships
        let mut entity_ids: HashSet<String> = HashSet::new();
        for rel in &rel_results {
            if let Some(src) = rel.metadata.get("source").and_then(|v| v.as_str()) {
                entity_ids.insert(src.to_string());
            }
            if let Some(tgt) = rel.metadata.get("target").and_then(|v| v.as_str()) {
                entity_ids.insert(tgt.to_string());
            }
        }

        // 3. Get entity data (batch)
        let nodes = self.graph_storage.get_nodes_batch(&entity_ids.into_iter().collect()).await?;

        // 4. Get chunks linked to relationships via source_ids
        let chunks = self.chunk_linker.chunks_from_relationships(&rel_results).await?;

        Ok(QueryContext::from_global(relationships, entities, chunks))
    }

    /// Hybrid mode: Round-robin merge of local + global
    async fn query_hybrid(&self, keywords: &ExtractedKeywords, embeddings: &QueryEmbeddings) -> Result<QueryContext> {
        let (local, global) = tokio::join!(
            self.query_local(keywords, embeddings),
            self.query_global(keywords, embeddings),
        );

        // Round-robin interleave with deduplication
        QueryContext::merge_round_robin(local?, global?)
    }

    /// Mix mode: Hybrid + direct chunk vector search
    async fn query_mix(&self, keywords: &ExtractedKeywords, embeddings: &QueryEmbeddings) -> Result<QueryContext> {
        let (hybrid, direct_chunks) = tokio::join!(
            self.query_hybrid(keywords, embeddings),
            self.vector_stores.chunks.query(&embeddings.query, self.config.max_chunks / 2, None),
        );

        QueryContext::merge_with_direct_chunks(hybrid?, direct_chunks?)
    }
}
```

---

## Phase 5: Dynamic Token Budgeting

### 5.1 Priority-Based Allocation

```rust
pub struct TokenBudget {
    pub total_limit: usize,
    pub system_prompt_reserve: usize,
    pub response_reserve: usize,
}

impl TokenBudget {
    pub fn allocate(&self, context: &QueryContext, tokenizer: &dyn Tokenizer) -> TokenAllocations {
        let available = self.total_limit - self.system_prompt_reserve - self.response_reserve;

        // Count actual tokens
        let entity_tokens: usize = context.entities.iter()
            .map(|e| tokenizer.count_tokens(&e.to_string()))
            .sum();
        let rel_tokens: usize = context.relationships.iter()
            .map(|r| tokenizer.count_tokens(&r.to_string()))
            .sum();
        let chunk_tokens: usize = context.chunks.iter()
            .map(|c| tokenizer.count_tokens(&c.content))
            .sum();

        let total_needed = entity_tokens + rel_tokens + chunk_tokens;

        if total_needed <= available {
            // Everything fits
            return TokenAllocations::full(entity_tokens, rel_tokens, chunk_tokens);
        }

        // Priority-based allocation:
        // 1. Entities: 50% (core semantics)
        // 2. Chunks: 35% (grounding evidence)
        // 3. Relationships: 15% (connections)

        let entity_budget = (available as f32 * 0.50) as usize;
        let chunk_budget = (available as f32 * 0.35) as usize;
        let rel_budget = available - entity_budget - chunk_budget;

        TokenAllocations {
            entity_budget: entity_budget.min(entity_tokens),
            chunk_budget: chunk_budget.min(chunk_tokens),
            relationship_budget: rel_budget.min(rel_tokens),
        }
    }
}
```

---

## Phase 6: Query Caching

### 6.1 Multi-Level Cache

```rust
pub struct QueryCache {
    /// L1: Keyword extraction cache
    keyword_cache: Arc<dyn Cache<ExtractedKeywords>>,

    /// L2: Context cache (after retrieval)
    context_cache: Arc<dyn Cache<QueryContext>>,

    /// L3: Response cache (final answer)
    response_cache: Arc<dyn Cache<QueryResponse>>,

    /// Invalidation tracker
    invalidation: InvalidationTracker,
}

impl QueryCache {
    pub fn compute_key(&self, request: &QueryRequest) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&request.query);
        hasher.update(&request.mode.unwrap_or_default().to_string());
        if let Some(tid) = &request.tenant_id() {
            hasher.update(tid);
        }
        hex::encode(hasher.finalize())
    }

    pub async fn invalidate_for_document(&self, document_id: &str) {
        let affected = self.invalidation.get_keys_for_document(document_id).await;
        for key in affected {
            self.context_cache.delete(&key).await.ok();
            self.response_cache.delete(&key).await.ok();
        }
    }
}
```

---

## Phase 7: Reranking Implementation

### 7.1 Reranker Trait

```rust
#[async_trait]
pub trait Reranker: Send + Sync {
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<RankableDocument>,
        top_k: usize,
    ) -> Result<Vec<RerankedDocument>>;
}

pub struct RankableDocument {
    pub id: String,
    pub content: String,
    pub original_score: f32,
}

pub struct RerankedDocument {
    pub document: RankableDocument,
    pub rerank_score: f32,
    pub original_rank: usize,
}
```

### 7.2 Cohere Implementation

```rust
pub struct CohereReranker {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

#[async_trait]
impl Reranker for CohereReranker {
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<RankableDocument>,
        top_k: usize,
    ) -> Result<Vec<RerankedDocument>> {
        let response = self.client
            .post("https://api.cohere.ai/v1/rerank")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "query": query,
                "documents": documents.iter().map(|d| &d.content).collect::<Vec<_>>(),
                "top_n": top_k,
            }))
            .send()
            .await?;

        // Parse and reconstruct rankings
        // ...
    }
}
```

---

## Phase 8: Integration Tests

### 8.1 Test Suite Structure

```
edgequake-query/tests/
├── keyword_extraction_test.rs
├── vector_stores_test.rs
├── source_id_tracking_test.rs
├── query_engine_test.rs
├── token_budget_test.rs
├── query_cache_test.rs
├── reranking_test.rs
└── e2e_parity_test.rs        # Compare with LightRAG output
```

### 8.2 Parity Verification Tests

```rust
/// Compare EdgeQuake output with LightRAG baseline
#[tokio::test]
async fn test_parity_local_mode() {
    let engine = create_sota_query_engine().await;

    // Test queries that have known LightRAG results
    let queries = vec![
        "What is Sarah Chen's role?",
        "How does machine learning improve healthcare?",
        "Explain the relationship between OpenAI and Microsoft",
    ];

    for query in queries {
        let result = engine.query(QueryRequest::new(query).with_mode(QueryMode::Local)).await?;

        // Verify:
        // 1. Keywords extracted correctly
        // 2. Entities retrieved by low-level keywords
        // 3. Edges fetched in batch
        // 4. Chunks linked via source_ids
        assert!(!result.context.entities.is_empty(), "Should have entities");
        assert!(result.context.entities.len() <= engine.config().max_entities);
    }
}

#[tokio::test]
async fn test_parity_global_mode() {
    // Similar for global mode
}

#[tokio::test]
async fn test_parity_hybrid_mode() {
    // Test round-robin merging
}

#[tokio::test]
async fn test_batch_query_performance() {
    let engine = create_sota_query_engine().await;
    let start = std::time::Instant::now();

    // 100 entity retrieval should be <100ms
    let result = engine.query(
        QueryRequest::new("complex query").with_mode(QueryMode::Local)
    ).await?;

    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "Query should complete in <500ms, took {}ms", elapsed.as_millis());
}
```

---

## Implementation Order

| Priority | Phase                          | Estimated Effort | Dependencies |
| -------- | ------------------------------ | ---------------- | ------------ |
| P0       | Phase 1: Keyword Extraction    | 1 day            | None         |
| P0       | Phase 4: Query Engine Refactor | 2 days           | Phase 1      |
| P1       | Phase 2: Separate Vector DBs   | 2 days           | Phase 4      |
| P1       | Phase 3: Source ID Tracking    | 1 day            | Phase 4      |
| P2       | Phase 5: Token Budgeting       | 0.5 day          | Phase 4      |
| P2       | Phase 6: Query Caching         | 1 day            | Phase 4      |
| P3       | Phase 7: Reranking             | 1 day            | Phase 4      |
| P0       | Phase 8: Tests                 | Throughout       | All          |

---

## Success Metrics

### Feature Parity (Minimum)

- [ ] LLM keyword extraction with caching
- [ ] Separate entity/relationship/chunk vector searches
- [ ] Low-level keywords → Entity VDB (Local mode)
- [ ] High-level keywords → Relationship VDB (Global mode)
- [ ] Source ID tracking for chunk provenance
- [ ] Batch graph operations (existing ✅)
- [ ] Query caching with TTL

### Performance

- Query latency P50 < 500ms
- Query latency P99 < 2000ms
- Batch 100 entities < 100ms
- Cache hit rate > 50%

### Quality

- Keyword extraction accuracy > 80%
- Retrieval recall matches LightRAG on test set
- Answer quality A/B parity with LightRAG

---

## Files to Create/Modify

### New Files

| File                                                              | Purpose                   |
| ----------------------------------------------------------------- | ------------------------- |
| `edgequake-query/src/keywords/mod.rs`                             | Keyword extraction module |
| `edgequake-query/src/keywords/llm_extractor.rs`                   | LLM-based extractor       |
| `edgequake-query/src/keywords/cache.rs`                           | Keyword cache             |
| `edgequake-query/src/keywords/intent.rs`                          | Query intent classifier   |
| `edgequake-query/src/vector_stores.rs`                            | Multi-VDB query layer     |
| `edgequake-query/src/chunk_linker.rs`                             | Source ID chunk linking   |
| `edgequake-query/src/rerank/mod.rs`                               | Reranking module          |
| `edgequake-query/src/rerank/cohere.rs`                            | Cohere reranker           |
| `edgequake-query/src/cache.rs`                                    | Query cache               |
| `edgequake-storage/src/adapters/postgres/entity_vectors.rs`       | Entity VDB                |
| `edgequake-storage/src/adapters/postgres/relationship_vectors.rs` | Relationship VDB          |

### Modified Files

| File                                     | Changes                   |
| ---------------------------------------- | ------------------------- |
| `edgequake-query/src/engine.rs`          | Integrate new components  |
| `edgequake-query/src/context.rs`         | Add source_ids field      |
| `edgequake-query/src/lib.rs`             | Export new modules        |
| `edgequake-storage/src/traits/vector.rs` | Add type-specific methods |
| `edgequake-pipeline/src/extraction.rs`   | Track source_ids          |

---

_This plan is based on the deep code audit from 2025-12-31. Implementation should proceed in priority order with tests written alongside each phase._
