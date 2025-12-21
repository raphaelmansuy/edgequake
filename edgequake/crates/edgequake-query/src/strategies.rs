//! Query strategies for different modes.
//!
//! Each query mode has a corresponding strategy that determines how to
//! retrieve and combine context from vector and graph storage.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;

use crate::context::{QueryContext, RetrievedChunk, RetrievedEntity, RetrievedRelationship};
use crate::error::Result;
use crate::modes::QueryMode;

use edgequake_storage::traits::{GraphStorage, VectorStorage};

/// Configuration for query strategies.
#[derive(Debug, Clone)]
pub struct StrategyConfig {
    /// Maximum chunks to retrieve.
    pub max_chunks: usize,
    
    /// Maximum entities to retrieve.
    pub max_entities: usize,
    
    /// Maximum relationships per entity.
    pub max_relationships_per_entity: usize,
    
    /// Graph traversal depth.
    pub graph_depth: usize,
    
    /// Minimum similarity score threshold.
    pub min_score: f32,
    
    /// Weight for vector search results (0.0 - 1.0).
    pub vector_weight: f32,
    
    /// Weight for graph search results (0.0 - 1.0).
    pub graph_weight: f32,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            max_chunks: 10,
            max_entities: 20,
            max_relationships_per_entity: 5,
            graph_depth: 2,
            min_score: 0.1,
            vector_weight: 0.5,
            graph_weight: 0.5,
        }
    }
}

/// A query strategy that retrieves context based on a specific mode.
#[async_trait]
pub trait QueryStrategy: Send + Sync {
    /// Execute the strategy and return context.
    async fn execute(
        &self,
        query: &str,
        query_embedding: &[f32],
        config: &StrategyConfig,
    ) -> Result<QueryContext>;

    /// Get the query mode for this strategy.
    fn mode(&self) -> QueryMode;
}

/// Naive query strategy - pure vector similarity search.
pub struct NaiveStrategy<V: VectorStorage> {
    vector_storage: Arc<V>,
}

impl<V: VectorStorage> NaiveStrategy<V> {
    /// Create a new naive strategy.
    pub fn new(vector_storage: Arc<V>) -> Self {
        Self { vector_storage }
    }
}

#[async_trait]
impl<V: VectorStorage> QueryStrategy for NaiveStrategy<V> {
    async fn execute(
        &self,
        _query: &str,
        query_embedding: &[f32],
        config: &StrategyConfig,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        // Simple vector similarity search
        let results = self
            .vector_storage
            .query(query_embedding, config.max_chunks, None)
            .await?;

        for result in results {
            if result.score >= config.min_score {
                let content = result
                    .metadata
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                context.add_chunk(RetrievedChunk::new(&result.id, content, result.score));
            }
        }

        Ok(context)
    }

    fn mode(&self) -> QueryMode {
        QueryMode::Naive
    }
}

/// Local query strategy - entity-centric search with neighborhood.
pub struct LocalStrategy<V: VectorStorage, G: GraphStorage> {
    vector_storage: Arc<V>,
    graph_storage: Arc<G>,
}

impl<V: VectorStorage, G: GraphStorage> LocalStrategy<V, G> {
    /// Create a new local strategy.
    pub fn new(vector_storage: Arc<V>, graph_storage: Arc<G>) -> Self {
        Self {
            vector_storage,
            graph_storage,
        }
    }
}

#[async_trait]
impl<V: VectorStorage, G: GraphStorage> QueryStrategy for LocalStrategy<V, G> {
    async fn execute(
        &self,
        query: &str,
        query_embedding: &[f32],
        config: &StrategyConfig,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        // Step 1: Vector search for initial chunks
        let chunk_results = self
            .vector_storage
            .query(query_embedding, config.max_chunks / 2, None)
            .await?;

        for result in &chunk_results {
            if result.score >= config.min_score {
                let content = result
                    .metadata
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                context.add_chunk(RetrievedChunk::new(&result.id, content, result.score));
            }
        }

        // Step 2: Find entities mentioned in top chunks
        let mut entity_ids = HashSet::new();
        for result in chunk_results.iter().take(3) {
            if let Some(entities) = result.metadata.get("entities") {
                if let Some(arr) = entities.as_array() {
                    for e in arr {
                        if let Some(s) = e.as_str() {
                            entity_ids.insert(normalize_entity_name(s));
                        }
                    }
                }
            }
        }

        // Step 3: Expand to include query terms as potential entities
        for word in query.split_whitespace() {
            let normalized = normalize_entity_name(word);
            if normalized.len() >= 3 {
                entity_ids.insert(normalized);
            }
        }

        // Step 4: Retrieve entities and their neighborhoods
        for entity_id in entity_ids.iter().take(config.max_entities) {
            if let Some(node) = self.graph_storage.get_node(entity_id).await? {
                let entity_type = node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string();

                let description = node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let degree = self.graph_storage.node_degree(entity_id).await?;

                context.add_entity(
                    RetrievedEntity::new(&node.id, entity_type, description).with_degree(degree),
                );

                // Get direct relationships
                let edges = self.graph_storage.get_node_edges(entity_id).await?;
                for edge in edges.iter().take(config.max_relationships_per_entity) {
                    let rel_type = edge
                        .properties
                        .get("relation_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("RELATED_TO")
                        .to_string();

                    let description = edge
                        .properties
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    context.add_relationship(
                        RetrievedRelationship::new(&edge.source, &edge.target, rel_type)
                            .with_description(description),
                    );
                }
            }
        }

        Ok(context)
    }

    fn mode(&self) -> QueryMode {
        QueryMode::Local
    }
}

/// Global query strategy - community/cluster-based search.
pub struct GlobalStrategy<G: GraphStorage> {
    graph_storage: Arc<G>,
}

impl<G: GraphStorage> GlobalStrategy<G> {
    /// Create a new global strategy.
    pub fn new(graph_storage: Arc<G>) -> Self {
        Self { graph_storage }
    }
}

#[async_trait]
impl<G: GraphStorage> QueryStrategy for GlobalStrategy<G> {
    async fn execute(
        &self,
        _query: &str,
        _query_embedding: &[f32],
        config: &StrategyConfig,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        // Global strategy focuses on high-degree entities (hubs)
        // and their communities
        let popular = self
            .graph_storage
            .get_popular_labels(config.max_entities)
            .await?;

        let mut seen_relationships = HashSet::new();

        for entity_id in popular.iter().take(config.max_entities) {
            if let Some(node) = self.graph_storage.get_node(entity_id).await? {
                let entity_type = node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string();

                let description = node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let degree = self.graph_storage.node_degree(entity_id).await?;

                context.add_entity(
                    RetrievedEntity::new(&node.id, entity_type, description).with_degree(degree),
                );

                // Get all relationships for hub entities
                let edges = self.graph_storage.get_node_edges(entity_id).await?;
                for edge in edges.iter().take(config.max_relationships_per_entity * 2) {
                    let rel_key = format!("{}->{}:{}", &edge.source, &edge.target, 
                        edge.properties.get("relation_type").and_then(|v| v.as_str()).unwrap_or(""));
                    
                    if seen_relationships.insert(rel_key) {
                        let rel_type = edge
                            .properties
                            .get("relation_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("RELATED_TO")
                            .to_string();

                        let description = edge
                            .properties
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        context.add_relationship(
                            RetrievedRelationship::new(&edge.source, &edge.target, rel_type)
                                .with_description(description),
                        );
                    }
                }
            }
        }

        Ok(context)
    }

    fn mode(&self) -> QueryMode {
        QueryMode::Global
    }
}

/// Hybrid query strategy - combines local and global approaches.
pub struct HybridStrategy<V: VectorStorage, G: GraphStorage> {
    local_strategy: LocalStrategy<V, G>,
    global_strategy: GlobalStrategy<G>,
}

impl<V: VectorStorage, G: GraphStorage> HybridStrategy<V, G> {
    /// Create a new hybrid strategy.
    pub fn new(vector_storage: Arc<V>, graph_storage: Arc<G>) -> Self {
        Self {
            local_strategy: LocalStrategy::new(Arc::clone(&vector_storage), Arc::clone(&graph_storage)),
            global_strategy: GlobalStrategy::new(graph_storage),
        }
    }
}

#[async_trait]
impl<V: VectorStorage, G: GraphStorage> QueryStrategy for HybridStrategy<V, G> {
    async fn execute(
        &self,
        query: &str,
        query_embedding: &[f32],
        config: &StrategyConfig,
    ) -> Result<QueryContext> {
        // Run both strategies with reduced limits
        let mut local_config = config.clone();
        local_config.max_chunks /= 2;
        local_config.max_entities /= 2;

        let mut global_config = config.clone();
        global_config.max_entities /= 2;

        let local_context = self.local_strategy.execute(query, query_embedding, &local_config).await?;
        let global_context = self.global_strategy.execute(query, query_embedding, &global_config).await?;

        // Merge contexts
        let mut merged = QueryContext::new();

        // Add local chunks first (more relevant)
        for chunk in &local_context.chunks {
            merged.add_chunk(chunk.clone());
        }

        // Merge entities (deduplicate)
        let mut seen_entities = HashSet::new();
        for entity in local_context.entities.iter().chain(global_context.entities.iter()) {
            if seen_entities.insert(entity.name.clone()) {
                merged.add_entity(entity.clone());
            }
        }

        // Merge relationships (deduplicate)
        let mut seen_rels = HashSet::new();
        for rel in local_context.relationships.iter().chain(global_context.relationships.iter()) {
            let key = format!("{}->{}:{}", rel.source, rel.target, rel.relation_type);
            if seen_rels.insert(key) {
                merged.add_relationship(rel.clone());
            }
        }

        Ok(merged)
    }

    fn mode(&self) -> QueryMode {
        QueryMode::Hybrid
    }
}

/// Mix query strategy - weighted combination of naive and graph-based.
pub struct MixStrategy<V: VectorStorage, G: GraphStorage> {
    naive_strategy: NaiveStrategy<V>,
    hybrid_strategy: HybridStrategy<V, G>,
}

impl<V: VectorStorage, G: GraphStorage> MixStrategy<V, G> {
    /// Create a new mix strategy.
    pub fn new(vector_storage: Arc<V>, graph_storage: Arc<G>) -> Self {
        Self {
            naive_strategy: NaiveStrategy::new(Arc::clone(&vector_storage)),
            hybrid_strategy: HybridStrategy::new(vector_storage, graph_storage),
        }
    }
}

#[async_trait]
impl<V: VectorStorage, G: GraphStorage> QueryStrategy for MixStrategy<V, G> {
    async fn execute(
        &self,
        query: &str,
        query_embedding: &[f32],
        config: &StrategyConfig,
    ) -> Result<QueryContext> {
        // Weight-based combination
        let vector_count = (config.max_chunks as f32 * config.vector_weight).ceil() as usize;
        let graph_count = (config.max_entities as f32 * config.graph_weight).ceil() as usize;

        let mut naive_config = config.clone();
        naive_config.max_chunks = vector_count.max(1);

        let mut hybrid_config = config.clone();
        hybrid_config.max_entities = graph_count.max(1);
        hybrid_config.max_chunks = 0; // Don't duplicate chunk retrieval

        let naive_context = self.naive_strategy.execute(query, query_embedding, &naive_config).await?;
        let hybrid_context = self.hybrid_strategy.execute(query, query_embedding, &hybrid_config).await?;

        // Combine with weights
        let mut merged = QueryContext::new();

        // Add naive chunks
        for chunk in &naive_context.chunks {
            merged.add_chunk(chunk.clone());
        }

        // Add hybrid chunks (if any)
        for chunk in &hybrid_context.chunks {
            merged.add_chunk(chunk.clone());
        }

        // Add all entities from hybrid
        for entity in &hybrid_context.entities {
            merged.add_entity(entity.clone());
        }

        // Add all relationships from hybrid
        for rel in &hybrid_context.relationships {
            merged.add_relationship(rel.clone());
        }

        Ok(merged)
    }

    fn mode(&self) -> QueryMode {
        QueryMode::Mix
    }
}

/// Normalize an entity name for consistent lookup.
fn normalize_entity_name(name: &str) -> String {
    name.trim().to_uppercase().replace(['-', '_'], " ")
}

/// Create a strategy for the given mode.
pub fn create_strategy<V, G>(
    mode: QueryMode,
    vector_storage: Arc<V>,
    graph_storage: Arc<G>,
) -> Box<dyn QueryStrategy>
where
    V: VectorStorage + 'static,
    G: GraphStorage + 'static,
{
    match mode {
        QueryMode::Naive => Box::new(NaiveStrategy::new(vector_storage)),
        QueryMode::Local => Box::new(LocalStrategy::new(vector_storage, graph_storage)),
        QueryMode::Global => Box::new(GlobalStrategy::new(graph_storage)),
        QueryMode::Hybrid => Box::new(HybridStrategy::new(vector_storage, graph_storage)),
        QueryMode::Mix => Box::new(MixStrategy::new(vector_storage, graph_storage)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_config_default() {
        let config = StrategyConfig::default();
        assert_eq!(config.max_chunks, 10);
        assert_eq!(config.max_entities, 20);
        assert!((config.vector_weight - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_normalize_entity_name() {
        assert_eq!(normalize_entity_name("rust-lang"), "RUST LANG");
        assert_eq!(normalize_entity_name("hello_world"), "HELLO WORLD");
        assert_eq!(normalize_entity_name("  Test  "), "TEST");
    }
}
