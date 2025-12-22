# EdgeQuake Query Retrieval Analysis & Implementation Plan

## Current Implementation Status

### ✅ What's Already Implemented

1. **Query Modes** (`edgequake-query/src/modes.rs`):
   - Naive, Local, Global, Hybrid, Mix modes defined
   - Proper enum structure with serde support

2. **Query Strategies** (`edgequake-query/src/strategies.rs`):
   - NaiveStrategy: Pure vector search
   - LocalStrategy: Entity-centric with 1-hop neighbors
   - GlobalStrategy: High-degree node (hub) based
   - HybridStrategy: Combines local + global
   - MixStrategy: Weighted combination
   - All strategies pass basic unit tests

3. **Query Context** (`edgequake-query/src/context.rs`):
   - RetrievedEntity, RetrievedRelationship, RetrievedChunk
   - QueryContext aggregator

### ❌ What's Missing (Compared to LightRAG)

#### 1. **Vector Search Integration**
**LightRAG Algorithm:**
```python
# In local mode:
entity_matches = await entity_vdb.search(query, top_k=top_k)
# Then fetch entity details + relationships + chunks

# In global mode:
relation_matches = await relations_vdb.search(query, top_k=top_k)
# Then fetch relationship details + connected entities

# In hybrid mode:
local_results = local_query(...)
global_results = global_query(...)
# Round-robin merge with deduplication
```

**EdgeQuake Current:**
- Local strategy searches chunks, infers entities from chunk metadata
- Global strategy uses `get_popular_labels()` instead of vector search
- ❌ **Missing**: Direct entity VDB and relationship VDB vector searches

#### 2. **Entity and Relationship Vector Storage**
**LightRAG:**
- Separate vector DBs: `entities_vdb`, `relationships_vdb`, `chunks_vdb`
- Entity vectors: `entity_name + description`
- Relationship vectors: `keywords + src + tgt + description`

**EdgeQuake:**
- Has VectorStorage trait
- ❌ **Missing**: Population of entity and relationship vectors during ingestion
- ❌ **Missing**: Separate VDB collections for entities vs relationships

#### 3. **Keyword Extraction from Query**
**LightRAG Algorithm:**
```python
async def get_keywords_from_query(query, query_param, global_config):
    if query_param.hl_keywords or query_param.ll_keywords:
        return query_param.hl_keywords, query_param.ll_keywords
    
    # Use LLM to extract keywords
    hl_keywords, ll_keywords = await extract_keywords_only(query, ...)
    return hl_keywords, ll_keywords
```

**EdgeQuake:**
- ❌ **Missing**: LLM-based keyword extraction
- ❌ **Missing**: High-level vs low-level keyword distinction
- Currently uses raw query string for vector search

#### 4. **Context Truncation by Token Limits**
**LightRAG:**
```python
truncate_list_by_token_size(
    items,
    max_token_size=max_entity_tokens,
    tokenizer=tokenizer
)
```

**EdgeQuake:**
- Uses fixed `max_entities`, `max_chunks` counts
- ❌ **Missing**: Token-aware truncation
- ❌ **Missing**: Tokenizer integration in query module

#### 5. **Reranking Support**
**LightRAG:**
```python
if query_param.enable_rerank:
    results = await rerank_with_llm(
        query, candidates, rerank_model
    )
```

**EdgeQuake:**
- ❌ **Missing**: Reranking integration
- ❌ **Missing**: Rerank configuration in QueryEngine

#### 6. **Hybrid Mode Round-Robin Merging**
**LightRAG Algorithm:**
```python
# Round-robin merge entities
final_entities = []
seen_entities = set()
max_len = max(len(local_entities), len(global_entities))

for i in range(max_len):
    # Alternate between local and global
    if i < len(local_entities):
        entity = local_entities[i]
        if entity_name not in seen_entities:
            final_entities.append(entity)
            seen_entities.add(entity_name)
    
    if i < len(global_entities):
        entity = global_entities[i]
        if entity_name not in seen_entities:
            final_entities.append(entity)
            seen_entities.add(entity_name)
```

**EdgeQuake:**
- Current hybrid just appends local then global
- ❌ **Missing**: Round-robin merging for better diversity

#### 7. **Source ID and Reference Tracking**
**LightRAG:**
- Every entity/relationship has `source_id` (chunk IDs)
- Every chunk has `file_path` for citations
- Query results include complete reference information

**EdgeQuake:**
- Has `source_id` in graph nodes
- ❌ **Missing**: Complete reference tracking in query results
- ❌ **Missing**: File path to chunk mapping

#### 8. **Processing Info and Metadata**
**LightRAG Query Response:**
```python
{
    "entities": [...],
    "relationships": [...],
    "chunks": [...],
    "references": [...],
    "processing_info": {
        "ll_keywords": ["entity1", "entity2"],
        "hl_keywords": ["concept1", "concept2"],
        "entities_after_truncation": 10,
        "relations_after_truncation": 5,
        "final_chunks_count": 3
    }
}
```

**EdgeQuake:**
- Has basic QueryContext
- ❌ **Missing**: Detailed processing metadata
- ❌ **Missing**: Keywords used in retrieval
- ❌ **Missing**: Truncation statistics

## Implementation Plan

### Phase 1: Vector Storage Enhancement (Priority: HIGH)

#### Task 1.1: Add Entity and Relationship VDB Population
**Files:**
- `edgequake-pipeline/src/entity_extractor.rs`
- `edgequake-core/src/orchestrator.rs`

**Changes:**
```rust
// After extracting entities, add to entity VDB
let entity_content = format!("{}\n{}", entity.name, entity.description);
entity_vdb.upsert(&[
    (entity_vdb_id, entity_embedding, json!({
        "content": entity_content,
        "entity_name": entity.name,
        "entity_type": entity.entity_type,
        "source_id": chunk_id
    }))
]).await?;

// After extracting relationships, add to relationships VDB
let rel_content = format!("{}\t{}\n{}\n{}",
    rel.keywords, rel.source, rel.target, rel.description);
relationships_vdb.upsert(&[
    (rel_vdb_id, rel_embedding, json!({
        "src_id": rel.source,
        "tgt_id": rel.target,
        "keywords": rel.keywords,
        "description": rel.description,
        "source_id": chunk_id
    }))
]).await?;
```

#### Task 1.2: Update Query Strategies to Use Entity/Relationship VDB
**File:** `edgequake-query/src/strategies.rs`

**LocalStrategy changes:**
```rust
async fn execute(&self, query: &str, query_embedding: &[f32], config: &StrategyConfig) 
    -> Result<QueryContext> 
{
    let mut context = QueryContext::new();
    
    // CHANGE: Search entity VDB directly instead of chunks
    let entity_matches = self.entity_vdb
        .query(query_embedding, config.max_entities, None)
        .await?;
    
    for result in &entity_matches {
        let entity_name = result.metadata
            .get("entity_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        // Get entity from graph
        if let Some(node) = self.graph_storage.get_node(&entity_name).await? {
            // Add entity
            context.add_entity(RetrievedEntity::new(...));
            
            // Get relationships
            let edges = self.graph_storage.get_node_edges(&entity_name).await?;
            for edge in edges {
                context.add_relationship(RetrievedRelationship::new(...));
            }
            
            // Get related chunks from source_id
            let chunk_ids = parse_source_ids(&node.properties["source_id"]);
            for chunk_id in chunk_ids {
                let chunk = self.chunks_storage.get_by_id(&chunk_id).await?;
                context.add_chunk(RetrievedChunk::new(...));
            }
        }
    }
    
    Ok(context)
}
```

**GlobalStrategy changes:**
```rust
async fn execute(&self, query: &str, query_embedding: &[f32], config: &StrategyConfig) 
    -> Result<QueryContext> 
{
    let mut context = QueryContext::new();
    
    // CHANGE: Search relationships VDB instead of popular labels
    let relation_matches = self.relationships_vdb
        .query(query_embedding, config.max_entities * 2, None)
        .await?;
    
    let mut seen_relationships = HashSet::new();
    
    for result in &relation_matches {
        let src = result.metadata.get("src_id")...;
        let tgt = result.metadata.get("tgt_id")...;
        
        let rel_key = format!("{}->{}:{}", src, tgt, rel_type);
        if seen_relationships.insert(rel_key) {
            // Get relationship from graph
            if let Some(edge) = self.graph_storage.get_edge(&src, &tgt).await? {
                context.add_relationship(RetrievedRelationship::new(...));
                
                // Get connected entities
                if let Some(src_node) = self.graph_storage.get_node(&src).await? {
                    context.add_entity(RetrievedEntity::new(...));
                }
                if let Some(tgt_node) = self.graph_storage.get_node(&tgt).await? {
                    context.add_entity(RetrievedEntity::new(...));
                }
            }
        }
    }
    
    Ok(context)
}
```

### Phase 2: Keyword Extraction (Priority: HIGH)

#### Task 2.1: Add Keyword Extraction Module
**New file:** `edgequake-query/src/keywords.rs`

```rust
pub struct KeywordExtractor {
    llm_provider: Arc<dyn LLMProvider>,
}

impl KeywordExtractor {
    pub async fn extract_keywords(&self, query: &str) 
        -> Result<(Vec<String>, Vec<String>)> 
    {
        let prompt = format!(
            r#"Extract high-level and low-level keywords from this query:
Query: "{}"

Return JSON:
{{
  "high_level_keywords": ["concept1", "topic1"],
  "low_level_keywords": ["entity1", "specific_term1"]
}}
"#,
            query
        );
        
        let response = self.llm_provider.complete(&prompt).await?;
        let keywords: KeywordResponse = serde_json::from_str(&response.content)?;
        
        Ok((keywords.high_level_keywords, keywords.low_level_keywords))
    }
}
```

#### Task 2.2: Integrate Keyword Extraction into Query Engine
**File:** `edgequake-query/src/engine.rs`

```rust
pub async fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
    // Extract keywords from query
    let (hl_keywords, ll_keywords) = if request.hl_keywords.is_empty() {
        self.keyword_extractor.extract_keywords(&request.query).await?
    } else {
        (request.hl_keywords, request.ll_keywords)
    };
    
    // Use keywords for vector search
    let search_query = match request.mode {
        QueryMode::Local => ll_keywords.join(" "),
        QueryMode::Global => hl_keywords.join(" "),
        QueryMode::Hybrid | QueryMode::Mix => {
            format!("{} {}", ll_keywords.join(" "), hl_keywords.join(" "))
        }
        QueryMode::Naive => request.query.clone(),
    };
    
    // Continue with query execution...
}
```

### Phase 3: Context Truncation and Token Management (Priority: MEDIUM)

#### Task 3.1: Add Tokenizer to QueryEngine
**File:** `edgequake-query/src/engine.rs`

```rust
pub struct QueryEngineConfig {
    pub max_entity_tokens: usize,      // Default: 12000
    pub max_relation_tokens: usize,    // Default: 12000
    pub max_chunk_tokens: usize,       // Default: 4000
    pub max_total_tokens: usize,       // Default: 8000
}

pub struct QueryEngine {
    // existing fields...
    tokenizer: Arc<Tokenizer>,
    config: QueryEngineConfig,
}
```

#### Task 3.2: Implement Token-Based Truncation
**File:** `edgequake-query/src/context.rs`

```rust
impl QueryContext {
    pub fn truncate_by_tokens(&mut self, config: &QueryEngineConfig, tokenizer: &Tokenizer) {
        // Truncate entities
        let mut entity_tokens = 0;
        self.entities.retain(|e| {
            let tokens = tokenizer.encode(&e.description).len();
            if entity_tokens + tokens <= config.max_entity_tokens {
                entity_tokens += tokens;
                true
            } else {
                false
            }
        });
        
        // Truncate relationships
        let mut rel_tokens = 0;
        self.relationships.retain(|r| {
            let tokens = tokenizer.encode(&r.description).len();
            if rel_tokens + tokens <= config.max_relation_tokens {
                rel_tokens += tokens;
                true
            } else {
                false
            }
        });
        
        // Truncate chunks
        let mut chunk_tokens = 0;
        self.chunks.retain(|c| {
            let tokens = tokenizer.encode(&c.content).len();
            if chunk_tokens + tokens <= config.max_chunk_tokens {
                chunk_tokens += tokens;
                true
            } else {
                false
            }
        });
    }
}
```

### Phase 4: Enhanced Hybrid Mode (Priority: MEDIUM)

#### Task 4.1: Implement Round-Robin Merging
**File:** `edgequake-query/src/strategies.rs`

```rust
impl<V: VectorStorage, G: GraphStorage> HybridStrategy<V, G> {
    async fn execute(&self, ...) -> Result<QueryContext> {
        // Run both strategies
        let local_context = self.local_strategy.execute(...).await?;
        let global_context = self.global_strategy.execute(...).await?;
        
        let mut merged = QueryContext::new();
        
        // Round-robin merge entities
        let max_len = local_context.entities.len().max(global_context.entities.len());
        let mut seen_entities = HashSet::new();
        
        for i in 0..max_len {
            // Alternate: local first
            if i < local_context.entities.len() {
                let entity = &local_context.entities[i];
                if seen_entities.insert(entity.name.clone()) {
                    merged.add_entity(entity.clone());
                }
            }
            
            // Then global
            if i < global_context.entities.len() {
                let entity = &global_context.entities[i];
                if seen_entities.insert(entity.name.clone()) {
                    merged.add_entity(entity.clone());
                }
            }
        }
        
        // Similar for relationships...
        
        Ok(merged)
    }
}
```

### Phase 5: Reference Tracking (Priority: LOW)

#### Task 5.1: Add Reference Information to Context
**File:** `edgequake-query/src/context.rs`

```rust
#[derive(Debug, Clone, Serialize)]
pub struct QueryReference {
    pub reference_id: String,
    pub doc_id: String,
    pub doc_name: String,
    pub chunk_id: Option<String>,
    pub file_path: String,
    pub content: String,
}

pub struct QueryContext {
    pub entities: Vec<RetrievedEntity>,
    pub relationships: Vec<RetrievedRelationship>,
    pub chunks: Vec<RetrievedChunk>,
    pub references: Vec<QueryReference>,  // NEW
    pub processing_info: ProcessingInfo,  // NEW
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessingInfo {
    pub ll_keywords: Vec<String>,
    pub hl_keywords: Vec<String>,
    pub entities_before_truncation: usize,
    pub entities_after_truncation: usize,
    pub relations_before_truncation: usize,
    pub relations_after_truncation: usize,
    pub final_chunks_count: usize,
}
```

## Testing Plan

### Phase 1 Tests: Vector Storage
1. Test entity VDB population during ingestion
2. Test relationship VDB population
3. Test local strategy with entity VDB search
4. Test global strategy with relationship VDB search

### Phase 2 Tests: Keyword Extraction
1. Test keyword extraction with various queries
2. Test high-level vs low-level keyword distinction
3. Test query execution with extracted keywords

### Phase 3 Tests: Token Management
1. Test token counting for entities/relationships/chunks
2. Test truncation maintains most relevant items
3. Test total token limit enforcement

### Phase 4 Tests: Hybrid Mode
1. Test round-robin merging produces diverse results
2. Test deduplication works correctly
3. Compare against simple append approach

### Phase 5 Tests: References
1. Test reference tracking from entities to chunks
2. Test file path preservation through pipeline
3. Test reference ID generation and mapping

## Success Criteria

- [ ] All query modes return results matching LightRAG quality
- [ ] Entity and relationship vector searches work correctly
- [ ] Keyword extraction improves retrieval relevance
- [ ] Token limits prevent context overflow
- [ ] Hybrid mode provides balanced local+global view
- [ ] References enable proper citation
- [ ] All e2e tests pass with real knowledge graphs

## Next Steps

1. Start with Phase 1.1: Add entity/relationship VDB population
2. Create e2e test for local mode with entity VDB
3. Implement Phase 1.2: Update local strategy
4. Validate with multi-document knowledge graph
5. Continue through remaining phases systematically
