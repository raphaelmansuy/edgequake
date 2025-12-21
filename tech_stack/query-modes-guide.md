# Query Modes Implementation Guide for EdgeQuake

**Technology Stack**: Rust + Graph Algorithms + Vector Search  
**Date**: 2025-12-21  
**Status**: Complete  
**Related**: [surrealdb.md](./surrealdb.md), [rust-best-practices.md](./rust-best-practices.md)

---

## Overview

EdgeQuake supports four distinct query modes (based on LightRAG architecture), each optimized for different information retrieval scenarios. This guide provides comprehensive implementation patterns for all four modes in Rust.

**Query Modes**:

1. **Naive** - Direct vector search without graph traversal
2. **Local** - Entity-centric search with 1-hop neighborhood
3. **Global** - Community-based search using graph communities
4. **Hybrid** - Combines local and global approaches

**When to Use Each Mode**:

- **Naive**: Simple factual queries, keyword search
- **Local**: Specific entity relationships, detailed context
- **Global**: High-level summaries, thematic queries
- **Hybrid**: Complex queries requiring both detail and breadth

---

## Query Mode Architecture

### QueryMode Enum

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryMode {
    /// Direct vector search without graph traversal
    Naive,
    /// Entity-centric with local neighborhood (1-2 hops)
    Local,
    /// Community-based global search
    Global,
    /// Combination of local and global
    Hybrid,
}

impl Default for QueryMode {
    fn default() -> Self {
        QueryMode::Hybrid
    }
}

impl std::str::FromStr for QueryMode {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "naive" => Ok(QueryMode::Naive),
            "local" => Ok(QueryMode::Local),
            "global" => Ok(QueryMode::Global),
            "hybrid" => Ok(QueryMode::Hybrid),
            _ => Err(format!("Invalid query mode: {}", s)),
        }
    }
}
```

### Query Request Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    /// User's natural language query
    pub query: String,
    
    /// Query mode selection
    #[serde(default)]
    pub mode: QueryMode,
    
    /// Number of top results to return
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    
    /// Optional: Workspace isolation
    pub workspace_id: Option<String>,
    
    /// Optional: Filter by entity types
    pub entity_types: Option<Vec<String>>,
    
    /// Optional: Maximum graph traversal depth (local mode)
    pub max_depth: Option<usize>,
}

fn default_top_k() -> usize {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    /// Generated answer
    pub content: String,
    
    /// Retrieved entities
    pub entities: Vec<Entity>,
    
    /// Retrieved relations
    pub relations: Vec<Relation>,
    
    /// Source documents
    pub sources: Vec<String>,
    
    /// Query metadata
    pub metadata: QueryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    /// Query mode used
    pub mode: QueryMode,
    
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    
    /// Number of entities retrieved
    pub entities_count: usize,
    
    /// Number of relations retrieved
    pub relations_count: usize,
    
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}
```

---

## Mode 1: Naive Query

**Description**: Direct vector similarity search without graph traversal.

**Use Cases**:

- Simple factual queries
- Keyword search
- Quick lookups
- When graph relationships are not needed

**Algorithm**:

1. Generate query embedding
2. Perform vector similarity search
3. Retrieve top-k most similar entities
4. Generate response from entity descriptions

### Implementation

```rust
pub struct NaiveQueryEngine {
    storage: Arc<dyn Storage>,
    llm: Arc<dyn LLMClient>,
    embedding: Arc<dyn EmbeddingClient>,
}

impl NaiveQueryEngine {
    pub async fn query(
        &self,
        query: &str,
        top_k: usize,
    ) -> Result<QueryResponse> {
        let start = std::time::Instant::now();
        
        // 1. Generate query embedding
        let query_embedding = self.embedding
            .generate_embedding(query)
            .await?;
        
        // 2. Vector search for similar entities
        let entities = self.storage
            .vector_search(&query_embedding, top_k)
            .await?;
        
        // 3. Build context from entity descriptions
        let context = entities
            .iter()
            .map(|e| {
                format!(
                    "Entity: {}\nType: {}\nDescription: {}",
                    e.name,
                    e.entity_type,
                    e.description.as_deref().unwrap_or("N/A")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        
        // 4. Generate response with LLM
        let prompt = format!(
            "Based on the following entities, answer the query.\n\n\
             Query: {}\n\n\
             Context:\n{}\n\n\
             Answer:",
            query, context
        );
        
        let content = self.llm.complete(&prompt).await?;
        
        let elapsed = start.elapsed();
        
        Ok(QueryResponse {
            content,
            entities,
            relations: vec![],
            sources: vec![],
            metadata: QueryMetadata {
                mode: QueryMode::Naive,
                processing_time_ms: elapsed.as_millis() as u64,
                entities_count: entities.len(),
                relations_count: 0,
                confidence: 0.7,
            },
        })
    }
}
```

---

## Mode 2: Local Query

**Description**: Entity-centric search with graph neighborhood traversal.

**Use Cases**:

- Queries about specific entities and their relationships
- Detailed context around a topic
- Multi-hop reasoning
- When you need immediate connections

**Algorithm**:

1. Generate query embedding
2. Find top-k seed entities via vector search
3. For each seed entity, traverse graph 1-2 hops
4. Collect connected entities and relations
5. Build subgraph context
6. Generate response

### Implementation

```rust
pub struct LocalQueryEngine {
    storage: Arc<dyn Storage>,
    llm: Arc<dyn LLMClient>,
    embedding: Arc<dyn EmbeddingClient>,
}

impl LocalQueryEngine {
    pub async fn query(
        &self,
        query: &str,
        top_k: usize,
        max_depth: usize,
    ) -> Result<QueryResponse> {
        let start = std::time::Instant::now();
        
        // 1. Generate query embedding
        let query_embedding = self.embedding
            .generate_embedding(query)
            .await?;
        
        // 2. Find seed entities
        let seed_entities = self.storage
            .vector_search(&query_embedding, top_k)
            .await?;
        
        // 3. Expand to local neighborhood
        let mut all_entities = seed_entities.clone();
        let mut all_relations = Vec::new();
        
        for seed in &seed_entities {
            // Traverse graph from each seed entity
            let (neighbors, edges) = self.storage
                .traverse_graph(&seed.id, max_depth)
                .await?;
            
            all_entities.extend(neighbors);
            all_relations.extend(edges);
        }
        
        // Deduplicate entities
        all_entities.sort_by(|a, b| a.id.cmp(&b.id));
        all_entities.dedup_by(|a, b| a.id == b.id);
        
        // Deduplicate relations
        all_relations.sort_by(|a, b| a.id.cmp(&b.id));
        all_relations.dedup_by(|a, b| a.id == b.id);
        
        // 4. Build context from subgraph
        let context = self.build_local_context(&all_entities, &all_relations);
        
        // 5. Generate response
        let prompt = format!(
            "Based on the following knowledge graph, answer the query.\n\n\
             Query: {}\n\n\
             Knowledge Graph:\n{}\n\n\
             Answer:",
            query, context
        );
        
        let content = self.llm.complete(&prompt).await?;
        
        let elapsed = start.elapsed();
        
        Ok(QueryResponse {
            content,
            entities: all_entities,
            relations: all_relations,
            sources: vec![],
            metadata: QueryMetadata {
                mode: QueryMode::Local,
                processing_time_ms: elapsed.as_millis() as u64,
                entities_count: all_entities.len(),
                relations_count: all_relations.len(),
                confidence: 0.85,
            },
        })
    }
    
    fn build_local_context(
        &self,
        entities: &[Entity],
        relations: &[Relation],
    ) -> String {
        let mut context = String::new();
        
        // Add entities
        context.push_str("Entities:\n");
        for entity in entities {
            context.push_str(&format!(
                "- {} ({}): {}\n",
                entity.name,
                entity.entity_type,
                entity.description.as_deref().unwrap_or("N/A")
            ));
        }
        
        // Add relations
        context.push_str("\nRelationships:\n");
        for relation in relations {
            context.push_str(&format!(
                "- {} --[{}]--> {} (weight: {:.2})\n",
                self.get_entity_name(&relation.source_id, entities),
                relation.relation_type,
                self.get_entity_name(&relation.target_id, entities),
                relation.weight
            ));
        }
        
        context
    }
    
    fn get_entity_name(&self, id: &str, entities: &[Entity]) -> String {
        entities
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| id.to_string())
    }
}
```

---

## Mode 3: Global Query

**Description**: Community-based search using graph communities/clusters.

**Use Cases**:

- High-level summaries
- Thematic queries
- Topic overview
- Understanding overall structure

**Algorithm**:

1. Detect communities in the graph (Louvain/Leiden algorithm)
2. Generate community summaries
3. Generate query embedding
4. Search community summaries
5. Select top-k relevant communities
6. Generate response from community context

### Implementation

```rust
pub struct GlobalQueryEngine {
    storage: Arc<dyn Storage>,
    llm: Arc<dyn LLMClient>,
    embedding: Arc<dyn EmbeddingClient>,
}

#[derive(Debug, Clone)]
pub struct Community {
    pub id: String,
    pub entities: Vec<Entity>,
    pub summary: String,
    pub embedding: Vec<f32>,
}

impl GlobalQueryEngine {
    pub async fn query(
        &self,
        query: &str,
        top_k: usize,
    ) -> Result<QueryResponse> {
        let start = std::time::Instant::now();
        
        // 1. Load or compute communities
        let communities = self.get_or_compute_communities().await?;
        
        // 2. Generate query embedding
        let query_embedding = self.embedding
            .generate_embedding(query)
            .await?;
        
        // 3. Find most relevant communities
        let relevant_communities = self
            .find_relevant_communities(&communities, &query_embedding, top_k)
            .await?;
        
        // 4. Collect entities from relevant communities
        let mut all_entities = Vec::new();
        for community in &relevant_communities {
            all_entities.extend(community.entities.clone());
        }
        
        // 5. Build context from communities
        let context = self.build_global_context(&relevant_communities);
        
        // 6. Generate response
        let prompt = format!(
            "Based on the following thematic summaries, answer the query.\n\n\
             Query: {}\n\n\
             Context:\n{}\n\n\
             Answer:",
            query, context
        );
        
        let content = self.llm.complete(&prompt).await?;
        
        let elapsed = start.elapsed();
        
        Ok(QueryResponse {
            content,
            entities: all_entities,
            relations: vec![],
            sources: vec![],
            metadata: QueryMetadata {
                mode: QueryMode::Global,
                processing_time_ms: elapsed.as_millis() as u64,
                entities_count: all_entities.len(),
                relations_count: 0,
                confidence: 0.75,
            },
        })
    }
    
    async fn get_or_compute_communities(&self) -> Result<Vec<Community>> {
        // Try to load from cache
        if let Some(communities) = self.storage.load_communities().await? {
            return Ok(communities);
        }
        
        // Compute communities using Louvain algorithm
        let communities = self.compute_communities().await?;
        
        // Cache for future use
        self.storage.save_communities(&communities).await?;
        
        Ok(communities)
    }
    
    async fn compute_communities(&self) -> Result<Vec<Community>> {
        // 1. Load full graph
        let entities = self.storage.get_all_entities().await?;
        let relations = self.storage.get_all_relations().await?;
        
        // 2. Build adjacency matrix
        let graph = self.build_graph(&entities, &relations);
        
        // 3. Run community detection (Louvain algorithm)
        let communities = self.louvain_clustering(&graph);
        
        // 4. Generate summaries for each community
        let mut result = Vec::new();
        for community_entities in communities {
            let summary = self.generate_community_summary(&community_entities).await?;
            let embedding = self.embedding.generate_embedding(&summary).await?;
            
            result.push(Community {
                id: uuid::Uuid::new_v4().to_string(),
                entities: community_entities,
                summary,
                embedding,
            });
        }
        
        Ok(result)
    }
    
    async fn generate_community_summary(&self, entities: &[Entity]) -> Result<String> {
        let entity_list = entities
            .iter()
            .map(|e| format!("{} ({})", e.name, e.entity_type))
            .collect::<Vec<_>>()
            .join(", ");
        
        let prompt = format!(
            "Generate a concise thematic summary (2-3 sentences) for this group of related entities:\n\n\
             {}\n\n\
             Summary:",
            entity_list
        );
        
        self.llm.complete(&prompt).await
    }
    
    async fn find_relevant_communities(
        &self,
        communities: &[Community],
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<Community>> {
        let mut scored: Vec<(f32, Community)> = communities
            .iter()
            .map(|c| {
                let similarity = cosine_similarity(query_embedding, &c.embedding);
                (similarity, c.clone())
            })
            .collect();
        
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        
        Ok(scored.into_iter().take(top_k).map(|(_, c)| c).collect())
    }
    
    fn build_global_context(&self, communities: &[Community]) -> String {
        communities
            .iter()
            .enumerate()
            .map(|(i, c)| {
                format!(
                    "Theme {}: {} ({} entities)\n{}",
                    i + 1,
                    c.summary,
                    c.entities.len(),
                    c.entities.iter()
                        .take(5)
                        .map(|e| format!("  - {}", e.name))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
    
    fn build_graph(&self, entities: &[Entity], relations: &[Relation]) -> Graph {
        // Build adjacency matrix from entities and relations
        // (Implementation depends on graph library)
        todo!()
    }
    
    fn louvain_clustering(&self, graph: &Graph) -> Vec<Vec<Entity>> {
        // Implement Louvain community detection
        // Use petgraph or custom implementation
        todo!()
    }
}
```

---

## Mode 4: Hybrid Query

**Description**: Combines local and global approaches for comprehensive answers.

**Use Cases**:

- Complex multi-faceted queries
- Queries requiring both detail and breadth
- Default mode for general-purpose queries

**Algorithm**:

1. Run both local and global queries in parallel
2. Merge results with weighted scoring
3. Re-rank entities by relevance
4. Generate comprehensive response

### Implementation

```rust
pub struct HybridQueryEngine {
    local: LocalQueryEngine,
    global: GlobalQueryEngine,
}

impl HybridQueryEngine {
    pub async fn query(
        &self,
        query: &str,
        top_k: usize,
    ) -> Result<QueryResponse> {
        let start = std::time::Instant::now();
        
        // 1. Run local and global queries in parallel
        let (local_result, global_result) = tokio::join!(
            self.local.query(query, top_k, 2),
            self.global.query(query, top_k)
        );
        
        let local_result = local_result?;
        let global_result = global_result?;
        
        // 2. Merge entities with weighted scoring
        let merged_entities = self.merge_entities(
            &local_result.entities,
            &global_result.entities,
            0.6, // local weight
            0.4, // global weight
        );
        
        // 3. Merge relations (from local only, global doesn't have relations)
        let relations = local_result.relations;
        
        // 4. Build hybrid context
        let context = format!(
            "Local Context (Detailed):\n{}\n\n\
             Global Context (Thematic):\n{}",
            self.build_local_snippet(&local_result),
            self.build_global_snippet(&global_result)
        );
        
        // 5. Generate comprehensive response
        let prompt = format!(
            "Based on both detailed and thematic context, provide a comprehensive answer.\n\n\
             Query: {}\n\n\
             {}\n\n\
             Answer:",
            query, context
        );
        
        let content = self.local.llm.complete(&prompt).await?;
        
        let elapsed = start.elapsed();
        
        Ok(QueryResponse {
            content,
            entities: merged_entities,
            relations,
            sources: vec![],
            metadata: QueryMetadata {
                mode: QueryMode::Hybrid,
                processing_time_ms: elapsed.as_millis() as u64,
                entities_count: merged_entities.len(),
                relations_count: relations.len(),
                confidence: 0.90,
            },
        })
    }
    
    fn merge_entities(
        &self,
        local_entities: &[Entity],
        global_entities: &[Entity],
        local_weight: f32,
        global_weight: f32,
    ) -> Vec<Entity> {
        let mut entity_scores: HashMap<String, (Entity, f32)> = HashMap::new();
        
        // Score local entities
        for (i, entity) in local_entities.iter().enumerate() {
            let score = local_weight * (1.0 - i as f32 / local_entities.len() as f32);
            entity_scores.insert(
                entity.id.clone(),
                (entity.clone(), score),
            );
        }
        
        // Score global entities (add to existing or insert)
        for (i, entity) in global_entities.iter().enumerate() {
            let score = global_weight * (1.0 - i as f32 / global_entities.len() as f32);
            entity_scores
                .entry(entity.id.clone())
                .and_modify(|(_, s)| *s += score)
                .or_insert((entity.clone(), score));
        }
        
        // Sort by score and take top-k
        let mut scored: Vec<_> = entity_scores.into_values().collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        scored.into_iter().map(|(e, _)| e).collect()
    }
    
    fn build_local_snippet(&self, result: &QueryResponse) -> String {
        result.entities
            .iter()
            .take(5)
            .map(|e| format!("- {}: {}", e.name, e.description.as_deref().unwrap_or("N/A")))
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    fn build_global_snippet(&self, result: &QueryResponse) -> String {
        result.content.lines().take(5).collect::<Vec<_>>().join("\n")
    }
}
```

---

## Query Orchestrator

### Unified Query Interface

```rust
pub struct QueryOrchestrator {
    naive: NaiveQueryEngine,
    local: LocalQueryEngine,
    global: GlobalQueryEngine,
    hybrid: HybridQueryEngine,
}

impl QueryOrchestrator {
    pub async fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        match request.mode {
            QueryMode::Naive => {
                self.naive.query(&request.query, request.top_k).await
            }
            QueryMode::Local => {
                let max_depth = request.max_depth.unwrap_or(2);
                self.local.query(&request.query, request.top_k, max_depth).await
            }
            QueryMode::Global => {
                self.global.query(&request.query, request.top_k).await
            }
            QueryMode::Hybrid => {
                self.hybrid.query(&request.query, request.top_k).await
            }
        }
    }
}
```

---

## Performance Optimization

### Caching

```rust
use moka::future::Cache;

pub struct CachedQueryOrchestrator {
    orchestrator: QueryOrchestrator,
    cache: Cache<String, QueryResponse>,
}

impl CachedQueryOrchestrator {
    pub fn new(orchestrator: QueryOrchestrator) -> Self {
        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(3600))
            .build();
        
        Self { orchestrator, cache }
    }
    
    pub async fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
        let cache_key = format!("{:?}:{}", request.mode, request.query);
        
        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(cached);
        }
        
        let response = self.orchestrator.query(request).await?;
        self.cache.insert(cache_key, response.clone()).await;
        
        Ok(response)
    }
}
```

---

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_naive_query() {
        let engine = setup_test_engine().await;
        let result = engine.query("What is Alice's role?", QueryMode::Naive, 10).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_local_query() {
        let engine = setup_test_engine().await;
        let result = engine.query("Who works with Alice?", QueryMode::Local, 10).await;
        assert!(result.is_ok());
        assert!(!result.unwrap().relations.is_empty());
    }
    
    #[tokio::test]
    async fn test_global_query() {
        let engine = setup_test_engine().await;
        let result = engine.query("What are the main themes?", QueryMode::Global, 10).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_hybrid_query() {
        let engine = setup_test_engine().await;
        let result = engine.query("Tell me about the organization", QueryMode::Hybrid, 10).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.metadata.confidence > 0.85);
    }
}
```

---

## Conclusion

This guide provides complete implementation patterns for all four query modes in LightRAG. Each mode serves specific use cases and can be selected based on query complexity and information needs.

**Key Takeaways**:

1. Naive for simple lookups
2. Local for entity-centric detailed queries
3. Global for thematic summaries
4. Hybrid for comprehensive answers (default)

**Next Steps**:

- Implement community detection algorithms
- Optimize query performance
- Add query result caching
- Create query mode selection heuristics

---

**Status**: ✅ COMPLETE - Query modes implementation guide ready
