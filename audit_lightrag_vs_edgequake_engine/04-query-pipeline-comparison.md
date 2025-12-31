# Query Pipeline Deep Comparison

## 1. Query Modes Overview

Both implementations support similar query modes, inspired by the LightRAG paper:

| Mode       | Description                 | Entity Focus          | Relationship Focus  |
| ---------- | --------------------------- | --------------------- | ------------------- |
| **Local**  | Entity-centric search       | ✅ High               | ✅ Via entity edges |
| **Global** | Relationship-centric search | ✅ Via edge endpoints | ✅ High             |
| **Hybrid** | Combined local + global     | ✅ High               | ✅ High             |
| **Mix**    | Weighted naive + KG         | ✅ Medium             | ✅ Medium           |
| **Naive**  | Pure vector similarity      | ❌ None               | ❌ None             |

---

## 2. Query Pipeline Architecture

### LightRAG Query Pipeline

```
Query
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 1: Keyword Extraction                                 │
│   - LLM call for high-level (themes, concepts)              │
│   - LLM call for low-level (entities, specifics)            │
│   - Cache results                                           │
│   Code: operate.py:kg_query → keyword extraction section    │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 2: KG Search (_perform_kg_search)                     │
│   - Query embeddings generation                             │
│   - Mode-specific VDB queries                               │
│   - Graph traversal for related data                        │
│   - Chunk tracking                                          │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 3: Token Truncation (_apply_token_truncation)         │
│   - max_entity_tokens limit                                 │
│   - max_relation_tokens limit                               │
│   - Filter to truncated subset                              │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 4: Chunk Merging (_merge_all_chunks)                  │
│   - Entity-related chunks                                   │
│   - Relationship-related chunks                             │
│   - Vector chunks (for mix mode)                            │
│   - Round-robin deduplication                               │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 5: Context Building (_build_context_str)              │
│   - Dynamic token allocation                                │
│   - chunk_token_limit calculation                           │
│   - Optional reranking (rerank_model_func)                  │
│   - Reference list generation                               │
│   - Raw data structure (entities, relationships, chunks)    │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 6: LLM Generation                                     │
│   - Cache check (query hash)                                │
│   - System prompt + context + query                         │
│   - Streaming or non-streaming                              │
│   - Response post-processing                                │
└─────────────────────────────────────────────────────────────┘
```

**Code References:**

- [lightrag/operate.py](lightrag/operate.py) - `kg_query`, `naive_query`
- Lines 3600-5000 contain query implementation

### EdgeQuake Query Pipeline

```
Query
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 1: Keyword Extraction (with caching)                  │
│   - CachedKeywordExtractor                                  │
│   - Extract high_level and low_level keywords               │
│   - Determine QueryIntent                                   │
│   Code: sota_engine.rs:query() step 1                       │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 2: Mode Selection                                     │
│   - Use explicit mode OR                                    │
│   - Adaptive: QueryIntent.recommended_mode()                │
│   Code: sota_engine.rs:query() step 2                       │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 3: Embedding Computation                              │
│   - QueryEmbeddings::compute()                              │
│   - Batch: query, high_level, low_level                     │
│   - Single provider call                                    │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 4: Mode-Specific Retrieval                            │
│   - query_local / query_global / query_hybrid / etc.        │
│   - Vector similarity search                                │
│   - Graph traversal                                         │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 5: Context Balancing                                  │
│   - balance_context() with TruncationConfig                 │
│   - Token-aware truncation                                  │
│   - Priority-based selection                                │
└─────────────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Stage 6: Answer Generation                                  │
│   - build_prompt()                                          │
│   - generate_answer() or stream                             │
│   - QueryResponse with context                              │
└─────────────────────────────────────────────────────────────┘
```

**Code References:**

- [edgequake-query/src/sota_engine.rs](edgequake/crates/edgequake-query/src/sota_engine.rs)
- [edgequake-query/src/strategies/](edgequake/crates/edgequake-query/src/strategies/)
- [edgequake-query/src/truncation.rs](edgequake/crates/edgequake-query/src/truncation.rs)

---

## 3. Keyword Extraction Comparison

### LightRAG Keyword Extraction

```python
# From operate.py - kg_query function
# Extract keywords using LLM
kw_prompt = PROMPTS["keywords_extraction"].format(query=query)
ll_keywords, hl_keywords = await extract_keywords(kw_prompt, use_llm_func)

# Keywords are comma-separated strings
# hl_keywords: "concept1, concept2, theme1"
# ll_keywords: "entity1, entity2, specific_term"
```

**Prompt (PROMPTS["keywords_extraction"]):**

```
Given a query, extract two types of keywords:
1. High-level keywords: Abstract themes, concepts, or topics
2. Low-level keywords: Specific entities, names, or technical terms

Query: {query}

Output format:
HIGH-LEVEL: keyword1, keyword2, ...
LOW-LEVEL: keyword1, keyword2, ...
```

**Features:**

- ✅ Separate high/low level extraction
- ✅ LLM response caching
- ❌ No query intent classification
- ❌ No adaptive mode selection

### EdgeQuake Keyword Extraction

```rust
// edgequake-query/src/keywords.rs
#[async_trait]
pub trait KeywordExtractor: Send + Sync {
    async fn extract(&self, query: &str) -> Result<Keywords>;
    async fn extract_extended(&self, query: &str) -> Result<ExtractedKeywords>;
}

#[derive(Debug, Clone)]
pub struct ExtractedKeywords {
    pub high_level: Vec<String>,
    pub low_level: Vec<String>,
    pub query_intent: QueryIntent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryIntent {
    Factual,      // Specific facts → Local mode
    Exploratory,  // Broad exploration → Global mode
    Comparative,  // Compare entities → Hybrid mode
    Analytical,   // Deep analysis → Hybrid mode
}

impl QueryIntent {
    pub fn recommended_mode(&self) -> QueryMode {
        match self {
            QueryIntent::Factual => QueryMode::Local,
            QueryIntent::Exploratory => QueryMode::Global,
            QueryIntent::Comparative => QueryMode::Hybrid,
            QueryIntent::Analytical => QueryMode::Hybrid,
        }
    }
}
```

**Features:**

- ✅ Separate high/low level extraction
- ✅ Query intent classification
- ✅ Adaptive mode recommendation
- ✅ Keyword caching (InMemoryKeywordCache, PostgresKeywordCache)
- ✅ Configurable TTL

### Keyword Extraction Comparison

| Feature             | LightRAG      | EdgeQuake          |
| ------------------- | ------------- | ------------------ |
| High-level keywords | ✅            | ✅                 |
| Low-level keywords  | ✅            | ✅                 |
| Query intent        | ❌            | ✅                 |
| Adaptive mode       | ❌            | ✅                 |
| Caching             | ✅ LLM cache  | ✅ Dedicated cache |
| Cache TTL           | Session-based | ✅ Configurable    |

---

## 4. Mode-Specific Retrieval Comparison

### Local Mode

**LightRAG:**

```python
# _get_node_data - entity-centric retrieval
async def _get_node_data(query, knowledge_graph_inst, entities_vdb, query_param):
    # 1. Query entity VDB with low-level keywords
    results = await entities_vdb.query(query, top_k=query_param.top_k)

    # 2. Batch get node data and degrees
    nodes_dict, degrees_dict = await asyncio.gather(
        knowledge_graph_inst.get_nodes_batch(node_ids),
        knowledge_graph_inst.node_degrees_batch(node_ids),
    )

    # 3. Get related edges from entities
    use_relations = await _find_most_related_edges_from_entities(
        node_datas, query_param, knowledge_graph_inst
    )

    return node_datas, use_relations
```

**EdgeQuake:**

```rust
// sota_engine.rs - query_local
async fn query_local(
    &self,
    keywords: &ExtractedKeywords,
    embeddings: &QueryEmbeddings,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<QueryContext> {
    // 1. Search entity VDB with low_level embedding
    let entities = self.vector_storage
        .search_entities(&embeddings.low_level, self.config.max_entities, tenant_id)
        .await?;

    // 2. Get related relationships from graph
    let relationships = self.get_entity_relationships(&entities).await?;

    // 3. Retrieve related chunks
    let chunks = self.retrieve_entity_chunks(&entities).await?;

    Ok(QueryContext { entities, relationships, chunks, ... })
}
```

### Global Mode

**LightRAG:**

```python
# _get_edge_data - relationship-centric retrieval
async def _get_edge_data(keywords, knowledge_graph_inst, relationships_vdb, query_param):
    # 1. Query relationship VDB with high-level keywords
    results = await relationships_vdb.query(keywords, top_k=query_param.top_k)

    # 2. Batch get edge data
    edge_data_dict = await knowledge_graph_inst.get_edges_batch(edge_pairs_dicts)

    # 3. Get related entities from relationships
    use_entities = await _find_most_related_entities_from_relationships(
        edge_datas, query_param, knowledge_graph_inst
    )

    return edge_datas, use_entities
```

**EdgeQuake:**

```rust
// sota_engine.rs - query_global
async fn query_global(
    &self,
    keywords: &ExtractedKeywords,
    embeddings: &QueryEmbeddings,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<QueryContext> {
    // 1. Search relationship VDB with high_level embedding
    let relationships = self.vector_storage
        .search_relationships(&embeddings.high_level, self.config.max_relationships, tenant_id)
        .await?;

    // 2. Get related entities from relationships
    let entities = self.get_relationship_entities(&relationships).await?;

    // 3. Retrieve related chunks
    let chunks = self.retrieve_relationship_chunks(&relationships).await?;

    Ok(QueryContext { entities, relationships, chunks, ... })
}
```

### Mode Comparison Matrix

| Aspect           | LightRAG                            | EdgeQuake                   |
| ---------------- | ----------------------------------- | --------------------------- |
| Local embedding  | Low-level keywords text             | Low-level embedding vector  |
| Global embedding | High-level keywords text            | High-level embedding vector |
| Batch operations | ✅ get_nodes_batch, get_edges_batch | ✅ Trait methods            |
| Degree ranking   | ✅ Node/edge degrees                | ❌ Not implemented          |
| Chunk selection  | WEIGHT or VECTOR                    | Configurable                |

---

## 5. Chunk Selection Comparison

### LightRAG Chunk Selection

```python
# Two methods: WEIGHT and VECTOR

# WEIGHT: Linear gradient weighted polling
def pick_by_weighted_polling(entities_with_chunks, max_chunks, min_per_entity=1):
    """
    Priority based on:
    1. Entity position (earlier = higher priority)
    2. Chunk occurrence frequency (higher = higher priority)
    """
    # Linear gradient: earlier entities get more chunks
    # Round-robin with frequency-weighted selection

# VECTOR: Embedding similarity
async def pick_by_vector_similarity(query, text_chunks_storage, chunks_vdb, ...):
    """
    Query chunks VDB with query embedding
    Filter to entity-related chunks
    Sort by cosine similarity
    """
```

**Configuration:**

```python
kg_chunk_pick_method = global_config.get("kg_chunk_pick_method", "WEIGHT")
# Options: "WEIGHT" or "VECTOR"
```

### EdgeQuake Chunk Selection

```rust
// edgequake-query/src/chunk_retrieval.rs
pub enum ChunkSelectionMethod {
    Weight,   // Occurrence-based
    Vector,   // Similarity-based
    Hybrid,   // Combined
}

pub async fn retrieve_chunks_from_entities(
    entities: &[RetrievedEntity],
    storage: &dyn KVStorage,
    method: ChunkSelectionMethod,
) -> Result<Vec<RetrievedChunk>> {
    // Get chunk IDs from entity source_chunk_ids
    // Apply selection method
    // Retrieve chunk content
}
```

### Chunk Selection Comparison

| Feature       | LightRAG       | EdgeQuake    |
| ------------- | -------------- | ------------ |
| WEIGHT method | ✅             | ✅ (partial) |
| VECTOR method | ✅             | ✅           |
| Hybrid method | ❌             | ✅           |
| Configurable  | ✅             | ✅           |
| Deduplication | ✅ Round-robin | ✅ Set-based |

---

## 6. Token Budgeting Comparison

### LightRAG Token Budgeting

```python
# Dynamic token allocation
max_total_tokens = query_param.max_total_tokens or DEFAULT_MAX_TOTAL_TOKENS

# Calculate overhead
sys_prompt_tokens = len(tokenizer.encode(pre_sys_prompt))
query_tokens = len(tokenizer.encode(query))
kg_context_tokens = len(tokenizer.encode(pre_kg_context))
buffer_tokens = 200

# Available for chunks
available_chunk_tokens = max_total_tokens - (
    sys_prompt_tokens + kg_context_tokens + query_tokens + buffer_tokens
)

# Truncate entities and relations first
entities_context = truncate_list_by_token_size(
    entities_context, max_token_size=max_entity_tokens, tokenizer=tokenizer
)
relations_context = truncate_list_by_token_size(
    relations_context, max_token_size=max_relation_tokens, tokenizer=tokenizer
)

# Then truncate chunks to remaining budget
truncated_chunks = await process_chunks_unified(
    ..., chunk_token_limit=available_chunk_tokens
)
```

### EdgeQuake Token Budgeting

```rust
// edgequake-query/src/truncation.rs
#[derive(Debug, Clone)]
pub struct TruncationConfig {
    pub max_entities: usize,
    pub max_relationships: usize,
    pub max_chunks: usize,
    pub max_entity_tokens: usize,
    pub max_relationship_tokens: usize,
    pub max_chunk_tokens: usize,
    pub max_total_tokens: usize,
}

pub fn balance_context(
    entities: Vec<RetrievedEntity>,
    relationships: Vec<RetrievedRelationship>,
    chunks: Vec<RetrievedChunk>,
    config: &TruncationConfig,
    tokenizer: &dyn Tokenizer,
) -> (Vec<RetrievedEntity>, Vec<RetrievedRelationship>, Vec<RetrievedChunk>) {
    // 1. Truncate entities to max_entity_tokens
    let truncated_entities = truncate_entities(entities, config.max_entity_tokens, tokenizer);

    // 2. Truncate relationships to max_relationship_tokens
    let truncated_relationships = truncate_relationships(
        relationships, config.max_relationship_tokens, tokenizer
    );

    // 3. Calculate remaining budget for chunks
    let used_tokens = count_tokens(&truncated_entities, tokenizer)
        + count_tokens(&truncated_relationships, tokenizer);
    let chunk_budget = config.max_total_tokens.saturating_sub(used_tokens);

    // 4. Truncate chunks to remaining budget
    let truncated_chunks = truncate_chunks(chunks, chunk_budget, tokenizer);

    (truncated_entities, truncated_relationships, truncated_chunks)
}
```

### Token Budgeting Comparison

| Feature              | LightRAG            | EdgeQuake           |
| -------------------- | ------------------- | ------------------- |
| Dynamic allocation   | ✅                  | ✅                  |
| Entity token limit   | ✅ Configurable     | ✅ Configurable     |
| Relation token limit | ✅ Configurable     | ✅ Configurable     |
| Chunk token limit    | ✅ Remaining budget | ✅ Remaining budget |
| Buffer tokens        | ✅ 200 fixed        | ❌ Not explicit     |
| Tokenizer            | TiktokenTokenizer   | SimpleTokenizer     |

---

## 7. Reranking Comparison

### LightRAG Reranking

```python
# Optional reranking support
rerank_model_func: Callable[..., object] | None = field(default=None)
min_rerank_score: float = field(default=DEFAULT_MIN_RERANK_SCORE)

# In context building
if query_param.enable_rerank and rerank_model_func:
    chunks = await rerank_model_func(query, chunks)
    chunks = [c for c in chunks if c["score"] >= min_rerank_score]
```

**Features:**

- ✅ Configurable rerank model
- ✅ Minimum score threshold
- ✅ Applied after chunk retrieval

### EdgeQuake Reranking

❌ **Not implemented**

EdgeQuake does not currently have reranking support.

### Reranking Comparison

| Feature         | LightRAG | EdgeQuake |
| --------------- | -------- | --------- |
| Reranking       | ✅       | ❌        |
| Custom model    | ✅       | ❌        |
| Score threshold | ✅       | ❌        |

---

## 8. Streaming Comparison

### LightRAG Streaming

```python
# AsyncIterator-based streaming
async def kg_query(..., stream: bool = False) -> QueryResult:
    if query_param.stream:
        response = await use_model_func(
            query,
            system_prompt=sys_prompt,
            stream=True,
        )
        # response is AsyncIterator[str]
        return QueryResult(response_iterator=response, raw_data=raw_data, is_streaming=True)
    else:
        response = await use_model_func(query, system_prompt=sys_prompt)
        return QueryResult(content=response, raw_data=raw_data)
```

### EdgeQuake Streaming

```rust
// BoxStream-based streaming
pub async fn query_stream(
    &self,
    request: QueryRequest,
) -> Result<futures::stream::BoxStream<'static, Result<String>>> {
    // ... all retrieval steps ...

    // Build prompt and stream response
    let prompt = self.build_prompt(&request.query, &final_context);

    self.llm_provider
        .stream(&prompt)
        .await
        .map(|stream| stream.map(|res| res.map_err(QueryError::from)).boxed())
        .map_err(QueryError::from)
}

// Combined stream with context
pub async fn query_stream_with_context(
    &self,
    request: QueryRequest,
) -> Result<(QueryContext, futures::stream::BoxStream<'static, Result<String>>)> {
    // Returns both context (for displaying sources) and stream
}
```

### Streaming Comparison

| Feature             | LightRAG         | EdgeQuake          |
| ------------------- | ---------------- | ------------------ |
| Streaming           | ✅ AsyncIterator | ✅ BoxStream       |
| Context with stream | ❌ Separate call | ✅ Combined method |
| Error handling      | Try/except       | Result mapping     |

---

## 9. Caching Comparison

### LightRAG Caching

```python
# Query cache with hash
args_hash = compute_args_hash(
    query_param.mode,
    query,
    query_param.response_type,
    query_param.top_k,
    ...
)

cached_result = await handle_cache(
    hashing_kv, args_hash, user_query, query_param.mode, cache_type="query"
)

if cached_result is not None:
    return QueryResult(content=cached_result[0], raw_data=...)

# After LLM call
await save_to_cache(hashing_kv, CacheData(...))
```

### EdgeQuake Caching

```rust
// Keyword cache
let keyword_extractor: Arc<dyn KeywordExtractor> = Arc::new(
    CachedKeywordExtractor::new(
        base_extractor,
        cache,
        Duration::from_secs(config.keyword_cache_ttl_secs),
    )
);

// Query caching not explicitly implemented in SOTA engine
// Would need to be added at API layer
```

### Caching Comparison

| Feature            | LightRAG              | EdgeQuake          |
| ------------------ | --------------------- | ------------------ |
| Keyword cache      | ✅ LLM response cache | ✅ Dedicated cache |
| Query result cache | ✅                    | ❌ Not in engine   |
| Cache TTL          | Session-based         | ✅ Configurable    |
| Cache key          | Hash of params        | Hash of query      |

---

## 10. Response Structure Comparison

### LightRAG QueryResult

```python
@dataclass
class QueryResult:
    content: str = ""                    # Non-streaming response
    response_iterator: AsyncIterator[str] | None = None  # Streaming
    raw_data: dict = field(default_factory=dict)  # Structured data
    is_streaming: bool = False

# raw_data structure:
{
    "status": "success",
    "data": {
        "entities": [...],
        "relationships": [...],
        "chunks": [...],
        "references": [...]
    },
    "metadata": {
        "query_mode": "hybrid",
        "keywords": {"high_level": [...], "low_level": [...]},
        "processing_info": {...}
    }
}
```

### EdgeQuake QueryResponse

```rust
#[derive(Debug, Clone)]
pub struct QueryResponse {
    pub answer: String,
    pub context: QueryContext,
    pub mode: QueryMode,
    pub stats: QueryStats,
}

#[derive(Debug, Clone)]
pub struct QueryContext {
    pub entities: Vec<RetrievedEntity>,
    pub relationships: Vec<RetrievedRelationship>,
    pub chunks: Vec<RetrievedChunk>,
    pub token_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct QueryStats {
    pub embedding_time_ms: u64,
    pub retrieval_time_ms: u64,
    pub generation_time_ms: u64,
    pub total_time_ms: u64,
    pub context_tokens: usize,
    pub generated_tokens: usize,
}
```

### Response Comparison

| Feature        | LightRAG       | EdgeQuake       |
| -------------- | -------------- | --------------- |
| Answer content | ✅             | ✅              |
| Entities       | ✅             | ✅              |
| Relationships  | ✅             | ✅              |
| Chunks         | ✅             | ✅              |
| References     | ✅             | ❌ Separate     |
| Keywords       | ✅ In metadata | ❌ Not returned |
| Timing stats   | ❌             | ✅              |
| Token counts   | ❌             | ✅              |

---

## 11. Summary and Recommendations

### Feature Gap Analysis

| Feature                | LightRAG | EdgeQuake  | Priority |
| ---------------------- | -------- | ---------- | -------- |
| Adaptive mode          | ❌       | ✅         | ✅ Keep  |
| Query intent           | ❌       | ✅         | ✅ Keep  |
| Reranking              | ✅       | ❌         | P1       |
| Query caching          | ✅       | ❌         | P2       |
| Degree ranking         | ✅       | ❌         | P2       |
| WEIGHT chunk selection | ✅ Full  | ✅ Partial | P2       |
| Keywords in response   | ✅       | ❌         | P3       |
| Timing stats           | ❌       | ✅         | ✅ Keep  |

### Recommended Actions for EdgeQuake

1. **P1: Implement Reranking**

   - Add `RerankerProvider` trait
   - Integrate after chunk retrieval
   - Support score threshold filtering

2. **P2: Query Result Caching**

   - Add cache at API layer
   - Hash query + params + mode
   - Configurable TTL

3. **P2: Degree-based Ranking**

   - Add node/edge degree calculation
   - Use for entity/relationship ordering
   - Improves result relevance

4. **P3: Include Keywords in Response**
   - Add to QueryResponse
   - Useful for UI display and debugging

---

_Document Version: 1.0_
_Last Updated: 2025-12-31_
