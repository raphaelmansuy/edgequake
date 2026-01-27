# Algorithmic Deep Dive

## 1. Overview

This document provides an in-depth comparison of the core algorithms used in LightRAG and EdgeQuake for knowledge graph construction and querying. We analyze the theoretical foundations, implementation details, and performance implications of each approach.

---

## 2. Entity Extraction Algorithms

### LightRAG: Tuple-Delimited Extraction with Gleaning

**Algorithm Overview:**

LightRAG uses a two-phase extraction approach:

1. **Initial Extraction**: LLM call to extract entities and relationships
2. **Gleaning (Optional)**: Second LLM call using conversation history to extract missed entities

**Prompt Structure:**

```
System: Extract entities of types: PERSON, ORGANIZATION, LOCATION, CONCEPT...
User: [input text]
Assistant: [initial extraction]
User: It seems some entities may have been missed. Re-read and extract.
```

**Output Format (Tuple-Delimited):**

```
("entity"<|>ENTITY_NAME<|>ENTITY_TYPE<|>DESCRIPTION)<|COMPLETE|>
("relationship"<|>SRC<|>TGT<|>DESCRIPTION<|>KEYWORDS<|>WEIGHT)<|COMPLETE|>
```

**Gleaning Algorithm (from operate.py):**

```python
# Initial extraction
final_result = await use_llm_func(entity_extraction_prompt)
maybe_nodes, maybe_edges = _process_extraction_result(final_result)

# Gleaning (if enabled)
if entity_extract_max_gleaning > 0:
    glean_result = await use_llm_func(
        continue_extraction_prompt,
        history_messages=history  # Contains initial extraction
    )
    glean_nodes, glean_edges = _process_extraction_result(glean_result)

    # Merge - prefer longer descriptions
    for entity_name, glean_entities in glean_nodes.items():
        if entity_name in maybe_nodes:
            if len(glean_desc) > len(original_desc):
                maybe_nodes[entity_name] = glean_entities
        else:
            maybe_nodes[entity_name] = glean_entities  # New entity
```

**Performance Impact:**

- +1 LLM call per chunk when gleaning enabled
- +20-30% more entities extracted
- +50-100% latency per chunk

### EdgeQuake: JSON-Structured Single-Pass Extraction

**Algorithm Overview:**

EdgeQuake uses a single-pass extraction with JSON output:

**Output Format (JSON):**

```json
{
  "entities": [
    {
      "name": "ENTITY_NAME",
      "type": "ENTITY_TYPE",
      "description": "Description text",
      "importance": 0.8
    }
  ],
  "relationships": [
    {
      "source": "SRC_ENTITY",
      "target": "TGT_ENTITY",
      "type": "RELATION_TYPE",
      "description": "Description",
      "weight": 0.7,
      "keywords": ["keyword1", "keyword2"]
    }
  ]
}
```

**Extraction Algorithm (from extractor.rs):**

```rust
pub async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
    let prompt = self.prompts.build_prompt(chunk.content.as_str());

    let response = self.llm_provider.complete(&prompt).await?;

    // Parse JSON response
    let parsed: ExtractionOutput = serde_json::from_str(&response)?;

    // Normalize entities
    let entities = parsed.entities
        .into_iter()
        .map(|e| ExtractedEntity {
            name: normalize_name(&e.name),
            entity_type: e.entity_type.to_uppercase(),
            description: e.description,
            importance: e.importance.unwrap_or(0.5),
            source_chunk_ids: vec![chunk.id.clone()],
            ..Default::default()
        })
        .collect();

    Ok(ExtractionResult { entities, relationships })
}
```

**Performance Impact:**

- 1 LLM call per chunk
- Faster parsing (native JSON)
- Fewer entities extracted vs gleaning

### Extraction Comparison

| Aspect            | LightRAG              | EdgeQuake       | Winner             |
| ----------------- | --------------------- | --------------- | ------------------ |
| LLM calls/chunk   | 1-2                   | 1               | EdgeQuake (cost)   |
| Entity yield      | Higher (gleaning)     | Lower           | LightRAG (quality) |
| Parse complexity  | High (regex/split)    | Low (JSON)      | EdgeQuake          |
| Error handling    | Fallback parsing      | JSON validation | EdgeQuake          |
| Importance scores | Implicit (via weight) | Explicit        | EdgeQuake          |

---

## 3. Description Merging Algorithms

### LightRAG: LLM-Powered Map-Reduce Summarization

When entities appear across multiple chunks, LightRAG uses LLM to intelligently merge descriptions:

**Algorithm (from operate.py):**

```python
async def handle_entity_relation_summary_map_reduce(
    entity_or_relation_name: str,
    description_list: list[str],
    global_config: dict,
) -> tuple[str, bool]:
    """
    Map-Reduce approach:
    1. If total_tokens < summary_context_size: join directly
    2. If len(descriptions) < force_llm_summary_on_merge: no LLM needed
    3. Otherwise: split into chunks, summarize each, recurse
    """

    # Handle trivial cases
    if len(description_list) <= 1:
        return description_list[0] if description_list else "", False

    current_list = description_list[:]

    while True:
        total_tokens = sum(len(tokenizer.encode(desc)) for desc in current_list)

        # Terminal condition: fits in context
        if total_tokens <= summary_context_size or len(current_list) <= 2:
            if len(current_list) < force_llm_summary_on_merge:
                return separator.join(current_list), False  # No LLM
            else:
                # Use LLM for final summarization
                return await _summarize_descriptions(current_list), True

        # Map phase: split into context-sized chunks
        chunks = split_by_token_limit(current_list, summary_context_size)

        # Reduce phase: summarize each chunk
        new_summaries = []
        for chunk in chunks:
            if len(chunk) == 1:
                new_summaries.append(chunk[0])  # No LLM needed
            else:
                summary = await _summarize_descriptions(chunk)
                new_summaries.append(summary)

        current_list = new_summaries  # Recurse
```

**Summarization Prompt:**

```
Summarize the following descriptions about {entity_name}:
{description_list_as_jsonl}

Write a comprehensive summary of approximately {summary_length} words.
```

**Quality Characteristics:**

- ✅ Intelligent deduplication
- ✅ Consistent voice and style
- ✅ Handles contradictions
- ❌ LLM cost per merge
- ❌ Potential information loss

### EdgeQuake: Simple Concatenation

EdgeQuake uses simple string concatenation for merging:

**Algorithm (from entity.rs):**

```rust
impl GraphEntity {
    pub fn merge(&mut self, other: &GraphEntity) {
        // Append description with separator
        if !other.description.is_empty() {
            if self.description.is_empty() {
                self.description = other.description.clone();
            } else {
                self.description = format!(
                    "{}\n{}",
                    self.description,
                    other.description
                );
            }
        }

        // Merge source IDs
        for source in other.get_sources() {
            self.add_source(source);
        }
    }
}
```

**Quality Characteristics:**

- ✅ Zero LLM cost
- ✅ No information loss
- ✅ Deterministic
- ❌ Descriptions may be repetitive
- ❌ No deduplication
- ❌ Can grow unboundedly

### Merging Comparison

| Aspect         | LightRAG             | EdgeQuake           | Notes                   |
| -------------- | -------------------- | ------------------- | ----------------------- |
| Method         | LLM Map-Reduce       | Concatenation       | Fundamentally different |
| LLM cost       | $0.001-0.01/entity   | $0                  | EdgeQuake saves cost    |
| Quality        | High (coherent)      | Medium (repetitive) | LightRAG better         |
| Determinism    | Low                  | High                | EdgeQuake predictable   |
| Info loss risk | Medium               | None                | EdgeQuake preserves all |
| Scalability    | O(n log n) LLM calls | O(1)                | EdgeQuake scales better |

---

## 4. Keyword Extraction Algorithms

### LightRAG: Dual-Level Keyword Extraction

```python
# From kg_query in operate.py
async def extract_keywords(query: str, use_llm_func) -> tuple[list, list]:
    """Extract high-level and low-level keywords from query."""

    prompt = PROMPTS["keywords_extraction"].format(query=query)
    response = await use_llm_func(prompt)

    # Parse response
    # Expected format:
    # HIGH-LEVEL: theme1, concept1, topic1
    # LOW-LEVEL: entity1, name1, specific_term1

    hl_keywords = parse_line(response, "HIGH-LEVEL")
    ll_keywords = parse_line(response, "LOW-LEVEL")

    return ll_keywords, hl_keywords
```

### EdgeQuake: Intent-Aware Keyword Extraction

```rust
// From keywords.rs
#[async_trait]
pub trait KeywordExtractor: Send + Sync {
    async fn extract_extended(&self, query: &str) -> Result<ExtractedKeywords>;
}

pub struct ExtractedKeywords {
    pub high_level: Vec<String>,
    pub low_level: Vec<String>,
    pub query_intent: QueryIntent,
}

pub enum QueryIntent {
    Factual,      // Specific facts → Local mode
    Exploratory,  // Broad exploration → Global mode
    Comparative,  // Compare entities → Hybrid mode
    Analytical,   // Deep analysis → Hybrid mode
}

impl LLMKeywordExtractor {
    async fn extract_extended(&self, query: &str) -> Result<ExtractedKeywords> {
        let prompt = self.build_prompt(query);
        let response = self.llm.complete(&prompt).await?;

        // Parse JSON response with intent
        let parsed: KeywordOutput = serde_json::from_str(&response)?;

        Ok(ExtractedKeywords {
            high_level: parsed.high_level,
            low_level: parsed.low_level,
            query_intent: classify_intent(&parsed.intent),
        })
    }
}
```

### Keyword Comparison

| Feature       | LightRAG     | EdgeQuake |
| ------------- | ------------ | --------- |
| High-level    | ✅           | ✅        |
| Low-level     | ✅           | ✅        |
| Query intent  | ❌           | ✅        |
| Adaptive mode | ❌           | ✅        |
| Output format | Text parsing | JSON      |

---

## 5. Query Mode Algorithms

### Local Mode (Entity-Centric)

**LightRAG Algorithm:**

```python
async def _get_node_data(query, knowledge_graph, entities_vdb, query_param):
    # 1. Query entity VDB with low-level keywords
    results = await entities_vdb.query(
        query,  # Uses low-level keywords
        top_k=query_param.top_k
    )

    # 2. Batch get node data and degrees
    nodes, degrees = await asyncio.gather(
        knowledge_graph.get_nodes_batch(node_ids),
        knowledge_graph.node_degrees_batch(node_ids),
    )

    # 3. Sort by degree (importance)
    node_datas = sorted(node_datas, key=lambda x: x["degree"], reverse=True)

    # 4. Find related edges from top entities
    edges = await _find_most_related_edges_from_entities(
        node_datas, query_param, knowledge_graph
    )

    return node_datas, edges
```

**EdgeQuake Algorithm:**

```rust
async fn query_local(&self, keywords: &ExtractedKeywords, embeddings: &QueryEmbeddings)
    -> Result<QueryContext>
{
    // 1. Search entity VDB with low_level embedding
    let entities = self.vector_storage
        .search_entities(&embeddings.low_level, self.config.max_entities)
        .await?;

    // 2. Get related relationships
    let relationships = self.get_entity_relationships(&entities).await?;

    // 3. Retrieve chunks
    let chunks = self.retrieve_entity_chunks(&entities).await?;

    Ok(QueryContext { entities, relationships, chunks })
}
```

### Global Mode (Relationship-Centric)

**LightRAG Algorithm:**

```python
async def _get_edge_data(keywords, knowledge_graph, relationships_vdb, query_param):
    # 1. Query relationship VDB with high-level keywords
    results = await relationships_vdb.query(
        keywords,  # Uses high-level keywords
        top_k=query_param.top_k
    )

    # 2. Get edge data with degrees
    edges, degrees = await asyncio.gather(
        knowledge_graph.get_edges_batch(edge_pairs),
        knowledge_graph.edge_degrees_batch(edge_pairs),
    )

    # 3. Sort by degree
    edge_datas = sorted(edge_datas, key=lambda x: x["degree"], reverse=True)

    # 4. Find related entities from relationships
    entities = await _find_most_related_entities_from_relationships(
        edge_datas, query_param, knowledge_graph
    )

    return edge_datas, entities
```

**EdgeQuake Algorithm:**

```rust
async fn query_global(&self, keywords: &ExtractedKeywords, embeddings: &QueryEmbeddings)
    -> Result<QueryContext>
{
    // 1. Search relationship VDB with high_level embedding
    let relationships = self.vector_storage
        .search_relationships(&embeddings.high_level, self.config.max_relationships)
        .await?;

    // 2. Get related entities
    let entities = self.get_relationship_entities(&relationships).await?;

    // 3. Retrieve chunks
    let chunks = self.retrieve_relationship_chunks(&relationships).await?;

    Ok(QueryContext { entities, relationships, chunks })
}
```

### Hybrid Mode (Combined)

**LightRAG Algorithm:**

```python
async def _hybrid_query(query, ...):
    # Run both in parallel
    local_result, global_result = await asyncio.gather(
        _get_node_data(query, ...),
        _get_edge_data(query, ...)
    )

    # Merge results
    entities = merge_entities(local_result.nodes, global_result.nodes)
    relations = merge_relations(local_result.edges, global_result.edges)

    return entities, relations
```

**EdgeQuake Algorithm:**

```rust
async fn query_hybrid(&self, keywords: &ExtractedKeywords, embeddings: &QueryEmbeddings)
    -> Result<QueryContext>
{
    // Run both in parallel
    let (local_ctx, global_ctx) = tokio::try_join!(
        self.query_local(keywords, embeddings),
        self.query_global(keywords, embeddings),
    )?;

    // Merge and deduplicate
    let entities = self.merge_contexts(&local_ctx.entities, &global_ctx.entities);
    let relationships = self.merge_contexts(&local_ctx.relationships, &global_ctx.relationships);

    Ok(QueryContext { entities, relationships, ... })
}
```

---

## 6. Chunk Selection Algorithms

### WEIGHT Method (Occurrence-Based)

**LightRAG Implementation:**

```python
def pick_by_weighted_polling(entities_with_chunks, max_chunks, min_per_entity=1):
    """
    Linear gradient weighted polling:
    - Earlier entities get more chunks
    - Higher frequency chunks get priority
    """

    # Calculate weights based on entity position (linear gradient)
    total_entities = len(entities_with_chunks)
    weights = [total_entities - i for i in range(total_entities)]

    # Normalize weights
    total_weight = sum(weights)
    normalized = [w / total_weight for w in weights]

    # Allocate chunks proportionally
    allocated = []
    for entity, weight in zip(entities_with_chunks, normalized):
        num_chunks = max(min_per_entity, int(max_chunks * weight))
        chunks = get_top_chunks_for_entity(entity, num_chunks)
        allocated.extend(chunks)

    # Deduplicate and return
    return list(dict.fromkeys(allocated))[:max_chunks]
```

### VECTOR Method (Similarity-Based)

**LightRAG Implementation:**

```python
async def pick_by_vector_similarity(query, chunks_vdb, entity_chunks, max_chunks):
    """
    Query chunks VDB for similarity-based selection
    """

    # Get chunk IDs associated with entities
    candidate_chunk_ids = set()
    for entity_chunks in entity_chunks.values():
        candidate_chunk_ids.update(entity_chunks)

    # Query VDB for similar chunks
    results = await chunks_vdb.query(query, top_k=max_chunks * 2)

    # Filter to entity-related chunks
    filtered = [r for r in results if r["id"] in candidate_chunk_ids]

    # Sort by similarity score
    return sorted(filtered, key=lambda x: x["score"], reverse=True)[:max_chunks]
```

### EdgeQuake Implementation

```rust
pub enum ChunkSelectionMethod {
    Weight,
    Vector,
    Hybrid,
}

pub async fn retrieve_chunks(
    entities: &[Entity],
    method: ChunkSelectionMethod,
    max_chunks: usize,
) -> Result<Vec<Chunk>> {
    match method {
        ChunkSelectionMethod::Weight => {
            // Collect all chunk IDs from entities
            let chunk_ids: HashSet<_> = entities
                .iter()
                .flat_map(|e| e.source_chunk_ids.iter())
                .collect();

            // Count occurrences
            let mut counts: HashMap<&str, usize> = HashMap::new();
            for entity in entities {
                for chunk_id in &entity.source_chunk_ids {
                    *counts.entry(chunk_id.as_str()).or_insert(0) += 1;
                }
            }

            // Sort by frequency
            let mut sorted: Vec<_> = counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));

            // Retrieve top chunks
            let top_ids: Vec<_> = sorted.iter().take(max_chunks).map(|(id, _)| *id).collect();
            storage.get_chunks_batch(&top_ids).await
        }

        ChunkSelectionMethod::Vector => {
            // Vector similarity search
            let results = vector_storage
                .search_chunks(&query_embedding, max_chunks)
                .await?;

            // Filter to entity-related
            let entity_chunk_ids: HashSet<_> = entities
                .iter()
                .flat_map(|e| &e.source_chunk_ids)
                .collect();

            results
                .into_iter()
                .filter(|r| entity_chunk_ids.contains(&r.id))
                .collect()
        }

        ChunkSelectionMethod::Hybrid => {
            // Combine both methods
            let weight_chunks = retrieve_chunks(entities, ChunkSelectionMethod::Weight, max_chunks / 2).await?;
            let vector_chunks = retrieve_chunks(entities, ChunkSelectionMethod::Vector, max_chunks / 2).await?;

            // Merge and deduplicate
            deduplicate(weight_chunks, vector_chunks)
        }
    }
}
```

---

## 7. Token Budgeting Algorithms

### LightRAG Dynamic Allocation

```python
# From operate.py - context building
max_total_tokens = query_param.max_total_tokens or DEFAULT_MAX_TOTAL_TOKENS

# Calculate overhead
sys_prompt_tokens = len(tokenizer.encode(system_prompt))
query_tokens = len(tokenizer.encode(query))
buffer_tokens = 200  # Safety margin

# Available for content
available_tokens = max_total_tokens - sys_prompt_tokens - query_tokens - buffer_tokens

# Truncate entities first (priority 1)
truncated_entities = truncate_list_by_token_size(
    entities, max_token_size=max_entity_tokens, tokenizer=tokenizer
)
entity_tokens = count_tokens(truncated_entities)

# Truncate relations (priority 2)
relation_budget = min(max_relation_tokens, available_tokens - entity_tokens)
truncated_relations = truncate_list_by_token_size(
    relations, max_token_size=relation_budget, tokenizer=tokenizer
)
relation_tokens = count_tokens(truncated_relations)

# Remaining budget for chunks (priority 3)
chunk_budget = available_tokens - entity_tokens - relation_tokens
truncated_chunks = truncate_chunks(chunks, chunk_budget, tokenizer)
```

### EdgeQuake Balanced Allocation

```rust
pub fn balance_context(
    entities: Vec<Entity>,
    relationships: Vec<Relationship>,
    chunks: Vec<Chunk>,
    config: &TruncationConfig,
    tokenizer: &dyn Tokenizer,
) -> (Vec<Entity>, Vec<Relationship>, Vec<Chunk>) {
    // Phase 1: Entity truncation
    let truncated_entities = truncate_by_tokens(
        entities,
        config.max_entity_tokens,
        tokenizer
    );
    let entity_tokens = count_tokens(&truncated_entities, tokenizer);

    // Phase 2: Relationship truncation
    let truncated_relationships = truncate_by_tokens(
        relationships,
        config.max_relationship_tokens,
        tokenizer
    );
    let relationship_tokens = count_tokens(&truncated_relationships, tokenizer);

    // Phase 3: Chunk allocation (remaining budget)
    let used = entity_tokens + relationship_tokens;
    let chunk_budget = config.max_total_tokens.saturating_sub(used);
    let truncated_chunks = truncate_by_tokens(chunks, chunk_budget, tokenizer);

    (truncated_entities, truncated_relationships, truncated_chunks)
}
```

---

## 8. SOTA Gap Analysis

### LightRAG Advantages

1. **Gleaning (+20-30% entities)**

   - Second extraction pass catches missed entities
   - Uses conversation history for context
   - EdgeQuake impact: Missing entities = incomplete graph

2. **LLM Description Merging**

   - Coherent, deduplicated descriptions
   - Handles contradictions intelligently
   - EdgeQuake impact: Repetitive, verbose descriptions

3. **Degree-Based Ranking**

   - Node/edge degrees for importance
   - Better result ordering
   - EdgeQuake impact: Potentially less relevant results

4. **Reranking Support**
   - Post-retrieval quality filtering
   - Score threshold filtering
   - EdgeQuake impact: Lower precision

### EdgeQuake Advantages

1. **Query Intent Classification**

   - Adaptive mode selection
   - Better user experience
   - LightRAG impact: Manual mode selection

2. **JSON Structured Output**

   - Reliable parsing
   - Type safety
   - LightRAG impact: Regex parsing fragile

3. **Cost Tracking**

   - Per-operation cost monitoring
   - Budget management
   - LightRAG impact: No visibility

4. **Lineage Infrastructure**
   - Full provenance tracking
   - Citations to source
   - LightRAG impact: Limited traceability

---

## 9. Recommendations

### Priority 1: Implement Gleaning

```rust
// Proposed implementation for edgequake
pub struct GleaningExtractor {
    base_extractor: Arc<dyn EntityExtractor>,
    max_gleaning_rounds: usize,
}

impl GleaningExtractor {
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        // Initial extraction
        let mut result = self.base_extractor.extract(chunk).await?;

        // Gleaning rounds
        for _ in 0..self.max_gleaning_rounds {
            let history = self.build_history(&result);
            let glean_result = self.base_extractor
                .extract_with_history(chunk, &history)
                .await?;

            // Merge: prefer longer descriptions
            result = self.merge_results(result, glean_result);
        }

        Ok(result)
    }
}
```

### Priority 2: Implement LLM Description Merging

```rust
// Proposed implementation
pub async fn merge_descriptions_llm(
    descriptions: Vec<String>,
    entity_name: &str,
    llm: &dyn LLMProvider,
    config: &MergeConfig,
) -> Result<String> {
    let total_tokens = count_tokens(&descriptions);

    // Terminal case: fits in context
    if total_tokens <= config.context_size {
        if descriptions.len() < config.force_llm_threshold {
            return Ok(descriptions.join("\n"));
        }
        return summarize_with_llm(&descriptions, entity_name, llm).await;
    }

    // Map-reduce: split, summarize, recurse
    let chunks = split_by_tokens(&descriptions, config.context_size);
    let summaries: Vec<String> = futures::future::try_join_all(
        chunks.iter().map(|chunk| summarize_with_llm(chunk, entity_name, llm))
    ).await?;

    // Recurse with summaries
    Box::pin(merge_descriptions_llm(summaries, entity_name, llm, config)).await
}
```

### Priority 3: Add Degree-Based Ranking

```rust
// Add to GraphStorage trait
#[async_trait]
pub trait GraphStorage: Send + Sync {
    async fn node_degree(&self, id: &str) -> Result<usize>;
    async fn edge_degree(&self, src: &str, tgt: &str) -> Result<usize>;
    async fn node_degrees_batch(&self, ids: &[String]) -> Result<HashMap<String, usize>>;
}

// Use in retrieval
async fn rank_entities_by_degree(
    entities: Vec<Entity>,
    graph: &dyn GraphStorage,
) -> Result<Vec<Entity>> {
    let degrees = graph.node_degrees_batch(
        &entities.iter().map(|e| e.id.clone()).collect::<Vec<_>>()
    ).await?;

    let mut ranked: Vec<_> = entities
        .into_iter()
        .map(|e| (e.clone(), degrees.get(&e.id).copied().unwrap_or(0)))
        .collect();

    ranked.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(ranked.into_iter().map(|(e, _)| e).collect())
}
```

---

_Document Version: 1.0_
_Last Updated: 2025-01-01_
