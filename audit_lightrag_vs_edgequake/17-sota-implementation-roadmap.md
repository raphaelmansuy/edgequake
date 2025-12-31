# EdgeQuake SOTA Query Implementation Roadmap

> **From 30% Feature Parity to State-of-the-Art**
> Based on Code Audit: 2025-12-31

---

## Current State Assessment

EdgeQuake's query engine is at **~30% feature parity** with LightRAG. To become SOTA, we need to:
1. Close the feature gap with LightRAG
2. Add innovations that go beyond LightRAG
3. Leverage Rust's performance advantages

---

## Phase 1: Foundation Fixes (Critical - 2 Weeks)

### 1.1 Implement Real Keyword Extraction

**Current State:** Stub implementation that splits words
**Target State:** LLM-based with caching

```rust
// New file: crates/edgequake-query/src/keywords/llm_extractor.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedKeywords {
    pub high_level: Vec<String>,
    pub low_level: Vec<String>,
    pub query_intent: QueryIntent,  // Beyond LightRAG!
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryIntent {
    Factual,      // "What is X?"
    Relational,   // "How does X relate to Y?"
    Exploratory,  // "Tell me about X"
    Comparative,  // "Compare X and Y"
    Procedural,   // "How to do X?"
}

pub struct LLMKeywordExtractor {
    llm_provider: Arc<dyn LLMProvider>,
    cache: Arc<dyn KeywordCache>,
}

#[async_trait]
impl KeywordExtractor for LLMKeywordExtractor {
    async fn extract(&self, query: &str) -> Result<ExtractedKeywords> {
        // 1. Check cache first
        let cache_key = self.compute_cache_key(query);
        if let Some(cached) = self.cache.get(&cache_key).await? {
            return Ok(cached);
        }
        
        // 2. Build prompt with examples
        let prompt = self.build_extraction_prompt(query);
        
        // 3. Call LLM with JSON mode
        let response = self.llm_provider
            .complete_json::<KeywordsResponse>(&prompt)
            .await?;
        
        // 4. Parse and validate
        let keywords = ExtractedKeywords {
            high_level: response.high_level_keywords,
            low_level: response.low_level_keywords,
            query_intent: self.classify_intent(&response),
        };
        
        // 5. Cache result
        self.cache.set(&cache_key, &keywords).await?;
        
        Ok(keywords)
    }
}
```

**SOTA Innovation:** Query intent classification for adaptive retrieval strategy.

### 1.2 Separate Vector Databases

**Current State:** Single unified vector storage
**Target State:** Dedicated DBs with semantic separation

```rust
// New file: crates/edgequake-query/src/vector_stores.rs

pub struct QueryVectorStores {
    /// Entity vectors indexed by description embeddings
    pub entities: Arc<dyn VectorStorage>,
    
    /// Relationship vectors indexed by description embeddings
    pub relationships: Arc<dyn VectorStorage>,
    
    /// Chunk vectors indexed by content embeddings
    pub chunks: Arc<dyn VectorStorage>,
}

impl QueryVectorStores {
    pub async fn search_by_mode(
        &self,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        mode: QueryMode,
    ) -> Result<SearchResults> {
        match mode {
            QueryMode::Local => {
                // Search entities with low-level keyword embedding
                let entities = self.entities
                    .query(&embeddings.low_level, MAX_ENTITIES)
                    .await?;
                // Get edges from entities (not separate search)
                Ok(SearchResults::local(entities))
            }
            QueryMode::Global => {
                // Search relationships with high-level keyword embedding
                let relationships = self.relationships
                    .query(&embeddings.high_level, MAX_RELATIONS)
                    .await?;
                // Get entities from relationships
                Ok(SearchResults::global(relationships))
            }
            QueryMode::Hybrid => {
                // Run both in parallel
                let (local, global) = tokio::join!(
                    self.search_by_mode(keywords, embeddings, QueryMode::Local),
                    self.search_by_mode(keywords, embeddings, QueryMode::Global),
                );
                Ok(SearchResults::merge(local?, global?))
            }
            QueryMode::Mix => {
                // Hybrid + direct chunk search
                let (hybrid, chunks) = tokio::join!(
                    self.search_by_mode(keywords, embeddings, QueryMode::Hybrid),
                    self.chunks.query(&embeddings.query, MAX_CHUNKS),
                );
                Ok(SearchResults::mix(hybrid?, chunks?))
            }
            QueryMode::Naive => {
                // Direct chunk search only
                let chunks = self.chunks
                    .query(&embeddings.query, MAX_CHUNKS)
                    .await?;
                Ok(SearchResults::naive(chunks))
            }
        }
    }
}

pub struct QueryEmbeddings {
    pub query: Vec<f32>,      // Original query embedding
    pub high_level: Vec<f32>, // High-level keywords embedding
    pub low_level: Vec<f32>,  // Low-level keywords embedding
}

impl QueryEmbeddings {
    pub async fn compute(
        keywords: &ExtractedKeywords,
        query: &str,
        embedder: &dyn EmbeddingProvider,
    ) -> Result<Self> {
        // Batch embed all three in one call
        let texts = vec![
            query.to_string(),
            keywords.high_level.join(", "),
            keywords.low_level.join(", "),
        ];
        let embeddings = embedder.embed_batch(&texts).await?;
        
        Ok(Self {
            query: embeddings[0].clone(),
            high_level: embeddings[1].clone(),
            low_level: embeddings[2].clone(),
        })
    }
}
```

**SOTA Innovation:** Multiple embeddings per query for semantic precision.

### 1.3 Source ID Tracking During Ingestion

**Current State:** Entities don't track source chunks
**Target State:** Full provenance chain

```rust
// Modify: crates/edgequake-pipeline/src/extraction.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: String,
    pub description: String,
    pub source_ids: Vec<SourceReference>,  // NEW!
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReference {
    pub chunk_id: String,
    pub document_id: String,
    pub file_path: String,
    pub char_offset: usize,
    pub char_length: usize,
}

// During extraction
impl EntityExtractor {
    pub async fn extract_with_provenance(
        &self,
        chunk: &Chunk,
    ) -> Result<Vec<ExtractedEntity>> {
        let raw_entities = self.extract_entities(&chunk.content).await?;
        
        raw_entities.into_iter().map(|e| {
            ExtractedEntity {
                name: e.name,
                entity_type: e.entity_type,
                description: e.description,
                source_ids: vec![SourceReference {
                    chunk_id: chunk.id.clone(),
                    document_id: chunk.document_id.clone(),
                    file_path: chunk.file_path.clone(),
                    char_offset: e.span.start,
                    char_length: e.span.end - e.span.start,
                }],
            }
        }).collect()
    }
}

// During graph storage
impl GraphStorage for PostgresAGE {
    async fn upsert_node(&self, id: &str, props: HashMap<String, Value>) -> Result<()> {
        // Store source_ids as JSONB array
        let source_ids = props.get("source_ids")
            .map(|v| serde_json::to_string(v).unwrap())
            .unwrap_or("[]".to_string());
        
        sqlx::query(r#"
            SELECT * FROM cypher('edgequake', $$
                MERGE (n {id: $id})
                SET n.source_ids = $source_ids::jsonb
                    n.description = $description
                RETURN n
            $$) AS (n agtype)
        "#)
        .bind(&id)
        .bind(&source_ids)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}
```

**SOTA Innovation:** Character-level provenance for citation highlighting.

---

## Phase 2: Query Pipeline Enhancement (2 Weeks)

### 2.1 Implement Full Reranking

**Current State:** API stub only
**Target State:** Production reranking with multiple backends

```rust
// New file: crates/edgequake-query/src/rerank/mod.rs

use async_trait::async_trait;

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
    pub metadata: HashMap<String, Value>,
}

pub struct RerankedDocument {
    pub document: RankableDocument,
    pub relevance_score: f32,
    pub original_rank: usize,
}

// Cohere implementation
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
        // Handle token limits
        let (chunked_docs, original_indices) = self.chunk_for_rerank(&documents);
        
        let response = self.client
            .post("https://api.cohere.ai/v1/rerank")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "query": query,
                "documents": chunked_docs,
                "top_n": top_k,
            }))
            .send()
            .await?;
        
        let result: CohereRerankResponse = response.json().await?;
        
        // Reconstruct with original indices
        self.reconstruct_rankings(result, original_indices, documents)
    }
}

// Local reranker using cross-encoder (SOTA!)
pub struct LocalCrossEncoderReranker {
    model: ort::Session,  // ONNX runtime
}

#[async_trait]
impl Reranker for LocalCrossEncoderReranker {
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<RankableDocument>,
        top_k: usize,
    ) -> Result<Vec<RerankedDocument>> {
        // Run cross-encoder locally - no API calls!
        let pairs: Vec<(String, String)> = documents
            .iter()
            .map(|d| (query.to_string(), d.content.clone()))
            .collect();
        
        let scores = self.model.run_batch(&pairs)?;
        
        let mut ranked: Vec<_> = documents
            .into_iter()
            .zip(scores)
            .enumerate()
            .map(|(i, (doc, score))| RerankedDocument {
                document: doc,
                relevance_score: score,
                original_rank: i,
            })
            .collect();
        
        ranked.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        ranked.truncate(top_k);
        
        Ok(ranked)
    }
}
```

**SOTA Innovation:** Local cross-encoder reranking (no external API, lower latency).

### 2.2 Dynamic Token Budgeting

**Current State:** Fixed proportional reduction
**Target State:** Priority-based dynamic allocation

```rust
// New file: crates/edgequake-query/src/token_budget.rs

pub struct TokenBudget {
    pub total_limit: usize,
    pub system_prompt_reserve: usize,
    pub query_tokens: usize,
    pub response_reserve: usize,
}

impl TokenBudget {
    pub fn compute_allocations(&self, context: &RawContext) -> TokenAllocations {
        let available = self.total_limit 
            - self.system_prompt_reserve 
            - self.query_tokens 
            - self.response_reserve;
        
        // Priority: Entities > Chunks > Relationships
        // Entities provide core semantics
        // Chunks provide grounding evidence  
        // Relationships provide connections
        
        let entity_tokens = context.entities.iter()
            .map(|e| self.count_entity_tokens(e))
            .sum::<usize>();
        
        let chunk_tokens = context.chunks.iter()
            .map(|c| self.count_tokens(&c.content))
            .sum::<usize>();
        
        let relation_tokens = context.relationships.iter()
            .map(|r| self.count_relation_tokens(r))
            .sum::<usize>();
        
        let total_requested = entity_tokens + chunk_tokens + relation_tokens;
        
        if total_requested <= available {
            // Everything fits!
            return TokenAllocations::full(entity_tokens, chunk_tokens, relation_tokens);
        }
        
        // Need to cut - use priority-based allocation
        // Step 1: Try to keep all entities
        if entity_tokens <= available {
            let remaining = available - entity_tokens;
            // Split remaining between chunks (70%) and relations (30%)
            let chunk_budget = (remaining as f32 * 0.7) as usize;
            let relation_budget = remaining - chunk_budget;
            
            return TokenAllocations {
                entity_budget: entity_tokens,
                chunk_budget: chunk_budget.min(chunk_tokens),
                relation_budget: relation_budget.min(relation_tokens),
            };
        }
        
        // Step 2: Need to cut entities too
        let entity_budget = (available as f32 * 0.5) as usize;
        let chunk_budget = (available as f32 * 0.35) as usize;
        let relation_budget = available - entity_budget - chunk_budget;
        
        TokenAllocations {
            entity_budget,
            chunk_budget,
            relation_budget,
        }
    }
}

pub struct TokenAllocations {
    pub entity_budget: usize,
    pub chunk_budget: usize,
    pub relation_budget: usize,
}
```

**SOTA Innovation:** Priority-based allocation instead of proportional reduction.

### 2.3 Implement Query Caching

**Current State:** No caching
**Target State:** Multi-level cache with smart invalidation

```rust
// New file: crates/edgequake-query/src/cache.rs

use sha2::{Sha256, Digest};

pub struct QueryCache {
    keyword_cache: Arc<dyn Cache<ExtractedKeywords>>,
    context_cache: Arc<dyn Cache<QueryContext>>,
    response_cache: Arc<dyn Cache<QueryResponse>>,
    invalidation_tracker: Arc<InvalidationTracker>,
}

impl QueryCache {
    pub fn compute_cache_key(&self, request: &QueryRequest) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&request.query);
        hasher.update(&request.mode.to_string());
        hasher.update(&request.max_results.unwrap_or(10).to_le_bytes());
        // Include tenant/workspace for isolation
        if let Some(tenant_id) = &request.tenant_id {
            hasher.update(tenant_id);
        }
        hex::encode(hasher.finalize())
    }
    
    pub async fn get_or_compute<T, F, Fut>(
        &self,
        cache: &Arc<dyn Cache<T>>,
        key: &str,
        compute: F,
    ) -> Result<T>
    where
        T: Clone + Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        // Check cache
        if let Some(cached) = cache.get(key).await? {
            tracing::debug!(key = %key, "Cache hit");
            return Ok(cached);
        }
        
        // Compute
        let result = compute().await?;
        
        // Store with TTL
        cache.set_with_ttl(key, &result, Duration::from_secs(3600)).await?;
        
        Ok(result)
    }
    
    /// Invalidate caches when documents change
    pub async fn invalidate_for_document(&self, document_id: &str) {
        // Find all cache keys that used this document
        let affected_keys = self.invalidation_tracker
            .get_keys_for_document(document_id)
            .await;
        
        for key in affected_keys {
            self.context_cache.delete(&key).await.ok();
            self.response_cache.delete(&key).await.ok();
        }
    }
}
```

**SOTA Innovation:** Document-aware cache invalidation.

---

## Phase 3: Graph-Aware Retrieval (2 Weeks)

### 3.1 Chunk Retrieval from Knowledge Graph

**Current State:** Placeholder implementation
**Target State:** Full source_id-based retrieval

```rust
// New file: crates/edgequake-query/src/chunk_graph_linker.rs

pub struct ChunkGraphLinker {
    graph_storage: Arc<dyn GraphStorage>,
    kv_storage: Arc<dyn KVStorage>,
    vector_storage: Arc<dyn VectorStorage>,
}

impl ChunkGraphLinker {
    /// Find chunks related to retrieved entities
    pub async fn chunks_from_entities(
        &self,
        entities: &[RetrievedEntity],
        method: ChunkSelectionMethod,
        query_embedding: Option<&[f32]>,
        max_chunks: usize,
    ) -> Result<Vec<LinkedChunk>> {
        // Step 1: Collect all source_ids from entities
        let mut chunk_frequency: HashMap<String, ChunkInfo> = HashMap::new();
        
        for entity in entities {
            let node = self.graph_storage.get_node(&entity.name).await?;
            if let Some(source_ids) = node.and_then(|n| n.properties.get("source_ids")) {
                let refs: Vec<SourceReference> = serde_json::from_value(source_ids.clone())?;
                
                for source_ref in refs {
                    let entry = chunk_frequency
                        .entry(source_ref.chunk_id.clone())
                        .or_insert(ChunkInfo {
                            chunk_id: source_ref.chunk_id,
                            document_id: source_ref.document_id,
                            file_path: source_ref.file_path,
                            entity_count: 0,
                            entities: Vec::new(),
                        });
                    entry.entity_count += 1;
                    entry.entities.push(entity.name.clone());
                }
            }
        }
        
        if chunk_frequency.is_empty() {
            return Ok(Vec::new());
        }
        
        // Step 2: Apply selection method
        let selected_ids = match method {
            ChunkSelectionMethod::Weight => {
                // Sort by entity frequency (more entities = more relevant)
                let mut sorted: Vec<_> = chunk_frequency.into_iter().collect();
                sorted.sort_by(|a, b| b.1.entity_count.cmp(&a.1.entity_count));
                sorted.into_iter()
                    .take(max_chunks)
                    .map(|(id, _)| id)
                    .collect()
            }
            ChunkSelectionMethod::Vector => {
                if let Some(embedding) = query_embedding {
                    // Get embeddings for candidate chunks
                    let chunk_ids: Vec<_> = chunk_frequency.keys().cloned().collect();
                    self.rerank_by_similarity(embedding, &chunk_ids, max_chunks).await?
                } else {
                    // Fall back to weight
                    self.chunks_from_entities(entities, ChunkSelectionMethod::Weight, None, max_chunks)
                        .await?
                        .into_iter()
                        .map(|c| c.id)
                        .collect()
                }
            }
            ChunkSelectionMethod::Hybrid => {
                // Combine both methods with 50/50 split
                let by_weight = self.chunks_from_entities(
                    entities, ChunkSelectionMethod::Weight, query_embedding, max_chunks / 2
                ).await?;
                let by_vector = self.chunks_from_entities(
                    entities, ChunkSelectionMethod::Vector, query_embedding, max_chunks / 2
                ).await?;
                
                // Merge with deduplication
                let mut seen = HashSet::new();
                let mut result = Vec::new();
                for chunk in by_weight.into_iter().chain(by_vector) {
                    if seen.insert(chunk.id.clone()) {
                        result.push(chunk.id);
                    }
                }
                result
            }
        };
        
        // Step 3: Batch retrieve chunk content
        let chunks = self.kv_storage.get_batch(&selected_ids).await?;
        
        Ok(chunks.into_iter()
            .filter_map(|(id, content)| {
                let info = chunk_frequency.get(&id)?;
                Some(LinkedChunk {
                    id,
                    content,
                    document_id: info.document_id.clone(),
                    file_path: info.file_path.clone(),
                    entity_count: info.entity_count,
                    linked_entities: info.entities.clone(),
                })
            })
            .collect())
    }
}

#[derive(Debug)]
pub struct LinkedChunk {
    pub id: String,
    pub content: String,
    pub document_id: String,
    pub file_path: String,
    pub entity_count: usize,
    pub linked_entities: Vec<String>,
}
```

**SOTA Innovation:** Hybrid chunk selection combining weight and vector methods.

### 3.2 Batch Graph Operations

**Current State:** Individual queries (N+1 problem)
**Target State:** Batch operations

```rust
// Modify: crates/edgequake-storage/src/traits.rs

#[async_trait]
pub trait GraphStorage: Send + Sync {
    // Existing
    async fn get_node(&self, id: &str) -> Result<Option<Node>>;
    
    // NEW: Batch operations
    async fn get_nodes_batch(&self, ids: &[&str]) -> Result<HashMap<String, Node>> {
        // Default implementation: sequential (can be overridden)
        let mut result = HashMap::new();
        for id in ids {
            if let Some(node) = self.get_node(id).await? {
                result.insert(id.to_string(), node);
            }
        }
        Ok(result)
    }
    
    async fn get_edges_batch(&self, pairs: &[(String, String)]) -> Result<HashMap<(String, String), Edge>>;
    
    async fn node_degrees_batch(&self, ids: &[&str]) -> Result<HashMap<String, usize>>;
    
    async fn edge_degrees_batch(&self, pairs: &[(String, String)]) -> Result<HashMap<(String, String), usize>>;
}

// PostgreSQL AGE implementation
impl GraphStorage for PostgresAGE {
    async fn get_nodes_batch(&self, ids: &[&str]) -> Result<HashMap<String, Node>> {
        // Single query with IN clause
        let query = format!(r#"
            SELECT * FROM cypher('edgequake', $$
                MATCH (n)
                WHERE n.id IN ['{}']
                RETURN n.id, properties(n)
            $$) AS (id agtype, props agtype)
        "#, ids.join("', '"));
        
        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await?;
        
        let mut result = HashMap::new();
        for row in rows {
            let id: String = row.try_get("id")?;
            let props: Value = row.try_get("props")?;
            result.insert(id.clone(), Node { id, properties: parse_props(props) });
        }
        Ok(result)
    }
}
```

**SOTA Innovation:** PostgreSQL AGE array-based batch queries.

---

## Phase 4: SOTA Innovations (Beyond LightRAG) (2 Weeks)

### 4.1 Query Intent-Adaptive Retrieval

```rust
// New file: crates/edgequake-query/src/adaptive_retrieval.rs

pub struct AdaptiveRetriever {
    strategies: HashMap<QueryIntent, Box<dyn RetrievalStrategy>>,
}

impl AdaptiveRetriever {
    pub async fn retrieve(
        &self,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        config: &RetrievalConfig,
    ) -> Result<QueryContext> {
        let strategy = self.strategies
            .get(&keywords.query_intent)
            .ok_or_else(|| Error::UnknownIntent)?;
        
        strategy.execute(keywords, embeddings, config).await
    }
}

/// Different strategies for different intents
#[async_trait]
pub trait RetrievalStrategy: Send + Sync {
    async fn execute(
        &self,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        config: &RetrievalConfig,
    ) -> Result<QueryContext>;
}

/// Factual queries: prioritize entities
pub struct FactualStrategy;

#[async_trait]
impl RetrievalStrategy for FactualStrategy {
    async fn execute(&self, ...) -> Result<QueryContext> {
        // More entities, fewer relationships
        // Higher precision, lower recall
    }
}

/// Relational queries: prioritize edges
pub struct RelationalStrategy;

#[async_trait]
impl RetrievalStrategy for RelationalStrategy {
    async fn execute(&self, ...) -> Result<QueryContext> {
        // Start from relationship search
        // Expand to connected entities
        // Include path information
    }
}

/// Exploratory queries: broad coverage
pub struct ExploratoryStrategy;

#[async_trait]
impl RetrievalStrategy for ExploratoryStrategy {
    async fn execute(&self, ...) -> Result<QueryContext> {
        // Community-based retrieval
        // Include diverse sources
        // Higher recall, accept lower precision
    }
}

/// Comparative queries: multi-entity focus
pub struct ComparativeStrategy;

#[async_trait]
impl RetrievalStrategy for ComparativeStrategy {
    async fn execute(&self, ...) -> Result<QueryContext> {
        // Identify comparison entities from keywords
        // Retrieve parallel information for each
        // Structure for side-by-side comparison
    }
}
```

**SOTA Innovation:** Intent-aware retrieval strategy selection.

### 4.2 Multi-Hop Reasoning Path Retrieval

```rust
// New file: crates/edgequake-query/src/reasoning_paths.rs

pub struct ReasoningPathRetriever {
    graph_storage: Arc<dyn GraphStorage>,
    path_cache: Arc<dyn PathCache>,
}

impl ReasoningPathRetriever {
    /// Find reasoning paths between entities in query
    pub async fn find_paths(
        &self,
        source_entities: &[String],
        target_entities: &[String],
        max_hops: usize,
    ) -> Result<Vec<ReasoningPath>> {
        let mut paths = Vec::new();
        
        for source in source_entities {
            for target in target_entities {
                if source == target { continue; }
                
                // BFS for shortest paths
                let found_paths = self.bfs_paths(source, target, max_hops).await?;
                paths.extend(found_paths);
            }
        }
        
        // Score paths by relevance
        paths.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        Ok(paths)
    }
    
    async fn bfs_paths(
        &self,
        source: &str,
        target: &str,
        max_hops: usize,
    ) -> Result<Vec<ReasoningPath>> {
        // Use PostgreSQL AGE's built-in path finding
        let query = r#"
            SELECT * FROM cypher('edgequake', $$
                MATCH path = shortestPath((a)-[*1..max_hops]->(b))
                WHERE a.id = $source AND b.id = $target
                RETURN path, length(path) as hops
                ORDER BY hops
                LIMIT 5
            $$) AS (path agtype, hops int)
        "#;
        
        let rows = sqlx::query(query)
            .bind(source)
            .bind(target)
            .bind(max_hops as i32)
            .fetch_all(&self.pool)
            .await?;
        
        // Parse paths
        rows.into_iter()
            .map(|row| self.parse_path(row))
            .collect()
    }
}

#[derive(Debug)]
pub struct ReasoningPath {
    pub nodes: Vec<String>,
    pub edges: Vec<String>,
    pub descriptions: Vec<String>,
    pub score: f32,
}

impl ReasoningPath {
    pub fn to_context_string(&self) -> String {
        // Format as: A -[relation]-> B -[relation]-> C
        let mut parts = Vec::new();
        for i in 0..self.nodes.len() {
            parts.push(self.nodes[i].clone());
            if i < self.edges.len() {
                parts.push(format!("-[{}]->", self.edges[i]));
            }
        }
        parts.join(" ")
    }
}
```

**SOTA Innovation:** Multi-hop reasoning path retrieval for complex queries.

### 4.3 Confidence-Weighted Context

```rust
// New file: crates/edgequake-query/src/confidence.rs

pub struct ConfidenceScorer {
    embedding_provider: Arc<dyn EmbeddingProvider>,
}

impl ConfidenceScorer {
    /// Score each piece of context for query relevance
    pub async fn score_context(
        &self,
        query_embedding: &[f32],
        context: &mut QueryContext,
    ) -> Result<()> {
        // Score entities
        for entity in &mut context.entities {
            let entity_text = format!("{}: {}", entity.name, entity.description);
            let entity_embedding = self.embedding_provider.embed_one(&entity_text).await?;
            entity.confidence = cosine_similarity(query_embedding, &entity_embedding);
        }
        
        // Score relationships
        for rel in &mut context.relationships {
            let rel_text = format!("{} {} {}", rel.source, rel.relation_type, rel.target);
            let rel_embedding = self.embedding_provider.embed_one(&rel_text).await?;
            rel.confidence = cosine_similarity(query_embedding, &rel_embedding);
        }
        
        // Score chunks
        for chunk in &mut context.chunks {
            chunk.confidence = chunk.score; // Already from vector search
        }
        
        // Sort all by confidence
        context.entities.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        context.relationships.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        context.chunks.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        Ok(())
    }
}

// Include confidence in context string for LLM
impl QueryContext {
    pub fn to_confidence_context_string(&self) -> String {
        let mut parts = Vec::new();
        
        parts.push("## Entities (sorted by relevance)".to_string());
        for entity in &self.entities {
            parts.push(format!(
                "- **{}** ({}) [conf: {:.2}]: {}",
                entity.name, entity.entity_type, entity.confidence, entity.description
            ));
        }
        
        // ... similar for relationships and chunks
        
        parts.join("\n")
    }
}
```

**SOTA Innovation:** Per-element confidence scores for LLM grounding.

### 4.4 Streaming RAG with Progressive Context

```rust
// New file: crates/edgequake-query/src/streaming.rs

pub struct StreamingQueryEngine {
    query_engine: QueryEngine,
}

impl StreamingQueryEngine {
    /// Progressive retrieval + generation
    pub async fn query_stream(
        &self,
        request: QueryRequest,
    ) -> Result<impl Stream<Item = StreamEvent>> {
        let (tx, rx) = mpsc::channel(100);
        
        tokio::spawn(async move {
            // Phase 1: Quick entity context (fast)
            tx.send(StreamEvent::Status("Retrieving entities...")).await.ok();
            let entities = self.retrieve_entities(&request).await?;
            tx.send(StreamEvent::Context(ContextUpdate::Entities(entities.clone()))).await.ok();
            
            // Start generation with partial context
            let partial_context = QueryContext { entities: entities.clone(), ..Default::default() };
            let gen_handle = self.start_generation(&request, &partial_context);
            
            // Phase 2: Get relationships in parallel
            let relationships = self.retrieve_relationships(&request).await?;
            tx.send(StreamEvent::Context(ContextUpdate::Relationships(relationships))).await.ok();
            
            // Phase 3: Get chunks
            let chunks = self.retrieve_chunks(&request).await?;
            tx.send(StreamEvent::Context(ContextUpdate::Chunks(chunks))).await.ok();
            
            // Stream generation tokens
            while let Some(token) = gen_handle.next().await {
                tx.send(StreamEvent::Token(token)).await.ok();
            }
            
            tx.send(StreamEvent::Done).await.ok();
        });
        
        Ok(ReceiverStream::new(rx))
    }
}

pub enum StreamEvent {
    Status(String),
    Context(ContextUpdate),
    Token(String),
    Done,
}

pub enum ContextUpdate {
    Entities(Vec<RetrievedEntity>),
    Relationships(Vec<RetrievedRelationship>),
    Chunks(Vec<RetrievedChunk>),
}
```

**SOTA Innovation:** Progressive context streaming with early generation start.

---

## Implementation Timeline

| Phase | Duration | Key Deliverables | Feature Parity |
|-------|----------|------------------|----------------|
| **Phase 1** | 2 weeks | Keyword extraction, Separate VDBs, Source tracking | 60% |
| **Phase 2** | 2 weeks | Reranking, Token budgeting, Caching | 85% |
| **Phase 3** | 2 weeks | Chunk linking, Batch operations | 95% |
| **Phase 4** | 2 weeks | Intent-adaptive, Multi-hop, Confidence scoring | 120% (SOTA) |

**Total: 8 weeks to SOTA**

---

## Success Metrics

### Feature Parity Metrics
- [ ] Keyword extraction uses LLM with caching
- [ ] Separate entity/relationship/chunk vector DBs
- [ ] Full source_id tracking in pipeline
- [ ] Cohere + local reranking working
- [ ] Query response caching with invalidation

### Performance Metrics
- Query latency P50 < 500ms
- Query latency P99 < 2000ms
- Cache hit rate > 60% for similar queries
- Batch operations reduce graph queries by 80%

### Quality Metrics
- NDCG@10 improvement over naive RAG
- Human preference A/B tests vs LightRAG
- Citation accuracy (do cited chunks contain answer?)

---

## Rust-Specific Advantages to Leverage

1. **Zero-copy parsing** for large context building
2. **Parallel iterators** (rayon) for batch scoring
3. **SIMD** for embedding similarity calculations
4. **Compile-time guarantees** for cache key typing
5. **Async all the way** with tokio for I/O parallelism

---

*This roadmap is based on the 2025-12-31 code audit. Priorities may shift based on user feedback and performance profiling.*
