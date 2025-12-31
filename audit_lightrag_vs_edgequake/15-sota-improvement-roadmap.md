# EdgeQuake SOTA Improvement Roadmap

> **Date:** 2024-12-31  
> **Status:** Proposed  
> **Goal:** Achieve feature parity and performance parity with LightRAG

---

## Overview

This document outlines the implementation plan to elevate EdgeQuake's query engine to State-of-the-Art (SOTA) level, matching and exceeding LightRAG's capabilities.

### Current Status

| Metric                      | Current    | Target  | Gap      |
| --------------------------- | ---------- | ------- | -------- |
| Query Latency (50 entities) | ~700ms     | <100ms  | 7x       |
| Batch Query Support         | ❌         | ✅      | Critical |
| Token Management            | ❌         | ✅      | Critical |
| Real Reranking              | ❌         | ✅      | High     |
| Query Caching               | ❌         | ✅      | High     |
| Streaming                   | ⚠️ Partial | ✅ Full | Medium   |

---

## Phase 1: Batch Query Operations (Week 1)

### 1.1 Extend GraphStorage Trait

**File:** `edgequake-storage/src/traits/graph.rs`

```rust
#[async_trait]
pub trait GraphStorage: Send + Sync {
    // Existing methods...

    /// Batch retrieve multiple nodes by ID
    async fn get_nodes_batch(&self, ids: &[&str]) -> Result<HashMap<String, GraphNode>>;

    /// Batch retrieve edges for multiple source nodes
    async fn get_edges_batch(&self, source_ids: &[&str]) -> Result<HashMap<String, Vec<GraphEdge>>>;

    /// Batch calculate degrees for multiple nodes
    async fn node_degrees_batch(&self, ids: &[&str]) -> Result<HashMap<String, i64>>;

    /// Combined batch: get nodes with their edges in one operation
    async fn get_nodes_with_edges_batch(
        &self,
        ids: &[&str]
    ) -> Result<Vec<(GraphNode, Vec<GraphEdge>)>>;
}
```

### 1.2 Implement for PostgresAGEGraphStorage

**SQL Query Pattern:**

```sql
WITH input(v, ord) AS (
  SELECT v, ord FROM unnest($1::text[]) WITH ORDINALITY AS t(v, ord)
),
ids(node_id, ord) AS (
  SELECT (to_json(v)::text)::agtype AS node_id, ord FROM input
)
SELECT i.node_id::text AS node_id, n.properties
FROM <graph>."Node" AS n
JOIN ids i ON ag_catalog.agtype_access_operator(
    VARIADIC ARRAY[n.properties, '"node_id"'::agtype]
) = i.node_id
ORDER BY i.ord;
```

### 1.3 Implement for MemoryGraphStorage

Add batch implementations for in-memory testing.

### 1.4 Update Query Engine

**File:** `edgequake-core/src/query.rs`

```rust
async fn query_local(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
    let entity_results = self.vector_storage
        .query(query_embedding, params.top_k, None)
        .await?;

    let entity_ids: Vec<&str> = entity_results
        .iter()
        .map(|r| r.id.as_str())
        .collect();

    // Single batch query instead of N individual queries
    let (nodes, edges) = tokio::join!(
        self.graph_storage.get_nodes_batch(&entity_ids),
        self.graph_storage.get_edges_batch(&entity_ids),
    );

    // Process results...
}
```

### 1.5 Tests

- Unit tests for batch methods in MemoryGraphStorage
- Integration tests with PostgreSQL
- Performance benchmark: batch vs individual queries

**Expected Improvement:** 5-10x faster retrieval for large entity sets

---

## Phase 2: Token Budget Management (Week 1-2)

### 2.1 Add Token Counting

**Dependency:** Add `tiktoken-rs` to `edgequake-core/Cargo.toml`

```toml
[dependencies]
tiktoken-rs = "0.5"
```

### 2.2 Create TokenBudget Module

**File:** `edgequake-core/src/token_budget.rs`

```rust
use tiktoken_rs::{cl100k_base, CoreBPE};

pub struct TokenBudget {
    tokenizer: CoreBPE,
    max_tokens: usize,
    entity_budget: usize,
    relationship_budget: usize,
    chunk_budget: usize,
}

impl TokenBudget {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            tokenizer: cl100k_base().unwrap(),
            max_tokens,
            entity_budget: max_tokens / 3,
            relationship_budget: max_tokens / 3,
            chunk_budget: max_tokens / 3,
        }
    }

    pub fn count_tokens(&self, text: &str) -> usize {
        self.tokenizer.encode_ordinary(text).len()
    }

    pub fn truncate_entities(&self, entities: Vec<ContextEntity>) -> Vec<ContextEntity> {
        let mut result = Vec::new();
        let mut used_tokens = 0;

        for entity in entities {
            let tokens = self.count_tokens(&format!("{}: {}", entity.name, entity.description));
            if used_tokens + tokens > self.entity_budget {
                break;
            }
            used_tokens += tokens;
            result.push(entity);
        }

        result
    }

    // Similar methods for relationships and chunks...
}
```

### 2.3 Integrate into Query Engine

```rust
async fn query_hybrid(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
    let budget = TokenBudget::new(params.max_context_tokens.unwrap_or(8000));

    // ... retrieve entities and relationships ...

    // Apply token truncation
    let truncated_entities = budget.truncate_entities(merged_entities);
    let truncated_relationships = budget.truncate_relationships(merged_relationships);

    // Build context with remaining budget for response
    let context = self.build_context(&truncated_entities, &truncated_relationships, &budget);
}
```

### 2.4 Configuration

Add to QueryParams:

- `max_entity_tokens: Option<usize>`
- `max_relationship_tokens: Option<usize>`
- `max_chunk_tokens: Option<usize>`
- `max_total_tokens: Option<usize>`

**Expected Improvement:** Prevent context overflow, better LLM performance

---

## Phase 3: Real Reranking Integration (Week 2)

### 3.1 Reranker Trait

**File:** `edgequake-llm/src/traits/reranker.rs`

```rust
#[async_trait]
pub trait Reranker: Send + Sync {
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<RerankDocument>,
        top_k: usize,
    ) -> Result<Vec<RerankResult>>;
}

pub struct RerankDocument {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, Value>,
}

pub struct RerankResult {
    pub id: String,
    pub score: f32,
    pub original_score: f32,
}
```

### 3.2 Cohere Reranker Implementation

**File:** `edgequake-llm/src/providers/cohere_reranker.rs`

```rust
pub struct CohereReranker {
    client: reqwest::Client,
    api_key: String,
    model: String,  // "rerank-english-v3.0"
}

#[async_trait]
impl Reranker for CohereReranker {
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<RerankDocument>,
        top_k: usize,
    ) -> Result<Vec<RerankResult>> {
        let response = self.client
            .post("https://api.cohere.ai/v1/rerank")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "query": query,
                "documents": documents.iter().map(|d| &d.content).collect::<Vec<_>>(),
                "top_n": top_k,
                "model": self.model,
            }))
            .send()
            .await?;

        // Parse and return results...
    }
}
```

### 3.3 Jina Reranker (Alternative)

```rust
pub struct JinaReranker {
    client: reqwest::Client,
    api_key: String,
    model: String,  // "jina-reranker-v2-base-multilingual"
}
```

### 3.4 Integration into Query Pipeline

```rust
async fn query_local(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
    // ... vector search ...

    if params.enable_rerank {
        let documents: Vec<RerankDocument> = chunks.iter()
            .map(|c| RerankDocument {
                id: c.id.clone(),
                content: c.content.clone(),
                metadata: HashMap::new(),
            })
            .collect();

        let reranked = self.reranker
            .rerank(query, documents, params.rerank_top_k.unwrap_or(10))
            .await?;

        // Apply reranked scores and filter
        chunks = self.apply_rerank_scores(chunks, reranked);
    }
}
```

**Expected Improvement:** 10-30% better answer relevance

---

## Phase 4: Query Result Caching (Week 2)

### 4.1 Cache Key Generation

```rust
fn compute_cache_key(
    query: &str,
    mode: QueryMode,
    top_k: usize,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> String {
    use sha2::{Sha256, Digest};

    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    hasher.update(format!("{:?}", mode).as_bytes());
    hasher.update(top_k.to_string().as_bytes());
    if let Some(tid) = tenant_id {
        hasher.update(tid.as_bytes());
    }
    if let Some(wid) = workspace_id {
        hasher.update(wid.as_bytes());
    }

    format!("query:{:x}", hasher.finalize())
}
```

### 4.2 Cache Storage

Use existing KVStorage for caching:

```rust
async fn check_cache(&self, cache_key: &str) -> Option<QueryResult> {
    if let Ok(Some(cached)) = self.kv_storage.get(cache_key).await {
        if let Ok(result) = serde_json::from_value::<CachedQueryResult>(cached) {
            // Check TTL
            if result.created_at + result.ttl > Utc::now() {
                return Some(result.into());
            }
        }
    }
    None
}

async fn save_to_cache(&self, cache_key: &str, result: &QueryResult, ttl_secs: u64) {
    let cached = CachedQueryResult {
        result: result.clone(),
        created_at: Utc::now(),
        ttl: Duration::seconds(ttl_secs as i64),
    };

    let _ = self.kv_storage
        .set(cache_key, serde_json::to_value(&cached).unwrap())
        .await;
}
```

### 4.3 Cache Invalidation

Invalidate cache when:

- Documents are added/deleted
- Entities/relationships are modified
- Graph structure changes

```rust
pub async fn invalidate_query_cache(&self, workspace_id: &str) {
    let pattern = format!("query:*:workspace:{}:*", workspace_id);
    self.kv_storage.delete_pattern(&pattern).await;
}
```

**Expected Improvement:** 100x faster for repeated queries

---

## Phase 5: Parallel Query Execution (Week 3)

### 5.1 Concurrent Retrieval

```rust
async fn query_hybrid(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
    // Run local and global queries in parallel
    let (local_result, global_result) = tokio::join!(
        self.query_local(query, params),
        self.query_global(query, params),
    );

    // Merge results
    self.merge_query_results(local_result?, global_result?)
}
```

### 5.2 Parallel Embedding

```rust
async fn embed_keywords(&self, keywords: &[String]) -> Result<Vec<Vec<f32>>> {
    // Batch embed all keywords at once
    self.embedding.embed(keywords).await
}
```

### 5.3 Pipeline Optimization

```rust
async fn query_with_pipeline(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
    // Stage 1: Start embedding immediately
    let embedding_future = self.embedding.embed(&[query.to_string()]);

    // Stage 2: Start keyword extraction in parallel
    let keyword_future = self.extract_keywords(query);

    // Wait for both
    let (embeddings, keywords) = tokio::join!(embedding_future, keyword_future);

    // Stage 3: Vector search with embeddings
    let vector_future = self.vector_storage.query(&embeddings?[0], params.top_k, None);

    // Stage 4: Batch graph queries
    // ...
}
```

**Expected Improvement:** 30-50% faster total query time

---

## Phase 6: Advanced Features (Week 3-4)

### 6.1 Query Explanation

Add explanation to response:

```rust
pub struct QueryExplanation {
    pub entities_used: Vec<EntityUsage>,
    pub relationships_used: Vec<RelationshipUsage>,
    pub chunks_used: Vec<ChunkUsage>,
    pub score_breakdown: ScoreBreakdown,
}

pub struct EntityUsage {
    pub name: String,
    pub relevance_score: f32,
    pub reason: String,  // "High semantic similarity to query"
}
```

### 6.2 Progressive Context Building

For streaming responses, build context incrementally:

```rust
async fn stream_query(&self, query: &str, params: &QueryParams) -> impl Stream<Item = QueryChunk> {
    // Emit chunks as they become available
    yield QueryChunk::Context(context_part);
    yield QueryChunk::Token(token);
    yield QueryChunk::Source(source);
}
```

### 6.3 Multi-Hop Reasoning

Support queries that require traversing multiple relationship hops:

```rust
async fn multi_hop_query(
    &self,
    query: &str,
    max_hops: usize,
) -> Result<QueryResult> {
    let mut visited = HashSet::new();
    let mut frontier = initial_entities;

    for hop in 0..max_hops {
        let edges = self.graph_storage
            .get_edges_batch(&frontier.iter().map(|e| e.as_str()).collect::<Vec<_>>())
            .await?;

        // Expand frontier...
    }
}
```

---

## Implementation Timeline

| Phase                 | Week | Effort | Impact |
| --------------------- | ---- | ------ | ------ |
| 1. Batch Queries      | 1    | 3 days | High   |
| 2. Token Management   | 1-2  | 2 days | High   |
| 3. Real Reranking     | 2    | 2 days | Medium |
| 4. Query Caching      | 2    | 1 day  | High   |
| 5. Parallel Execution | 3    | 2 days | Medium |
| 6. Advanced Features  | 3-4  | 4 days | Low    |

**Total Estimated Effort:** 14 developer days (~3 weeks)

---

## Success Metrics

### Performance Targets

| Metric        | Current   | Phase 1   | Phase 2      | Final   |
| ------------- | --------- | --------- | ------------ | ------- |
| Latency (p50) | 700ms     | 150ms     | 100ms        | <50ms   |
| Latency (p99) | 5s        | 500ms     | 300ms        | <200ms  |
| Throughput    | 10 qps    | 30 qps    | 50 qps       | 100 qps |
| Context Size  | Unlimited | 8K tokens | Configurable | Dynamic |

### Quality Targets

| Metric             | Current | Target                     |
| ------------------ | ------- | -------------------------- |
| Answer Relevance   | Good    | Excellent (with reranking) |
| Source Attribution | Basic   | Comprehensive              |
| Context Efficiency | Poor    | Optimal (token-aware)      |

---

## Risk Mitigation

| Risk                      | Mitigation                          |
| ------------------------- | ----------------------------------- |
| tiktoken-rs compatibility | Fall back to simple word count      |
| Cohere API rate limits    | Add rate limiting, fallback to Jina |
| Cache memory usage        | Implement LRU eviction policy       |
| Batch query complexity    | Thorough testing, gradual rollout   |

---

## Conclusion

Following this roadmap will transform EdgeQuake from a functional RAG system to a SOTA knowledge graph query engine. The most critical improvements (batch queries and token management) should be prioritized as they have the highest impact on both performance and quality.

**Next Steps:**

1. Create feature branches for each phase
2. Implement Phase 1 (batch queries) first
3. Run benchmarks after each phase
4. Document API changes
5. Update tests and CI/CD
