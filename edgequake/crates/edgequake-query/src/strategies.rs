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
            // WHY 20/60: Aligned with SOTAQueryConfig LightRAG-parity defaults.
            max_chunks: 20,
            max_entities: 60,
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
        _query: &str,
        query_embedding: &[f32],
        config: &StrategyConfig,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        // Step 1: Vector search for entities (as per LightRAG Local mode spec)
        // Local mode should search entity_vdb, not chunks
        let vector_results = self
            .vector_storage
            .query(query_embedding, config.max_entities * 2, None) // Get more for filtering
            .await?;

        // Filter to entity vectors only
        let entity_results = crate::vector_filter::filter_by_type(
            vector_results,
            crate::vector_filter::VectorType::Entity,
        );

        let mut entity_ids = HashSet::new();

        // Step 2: Extract entity IDs from vector results
        for result in entity_results.iter().take(config.max_entities) {
            if result.score >= config.min_score {
                let entity_name = result
                    .metadata
                    .get("entity_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if !entity_name.is_empty() {
                    entity_ids.insert(normalize_entity_name(&entity_name));
                }
            }
        }

        // Step 3: Retrieve entities and their local graph neighborhoods
        for entity_id in &entity_ids {
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

                // Get direct relationships (1-hop neighborhood)
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

/// Global query strategy - relationship-focused search.
pub struct GlobalStrategy<V: VectorStorage, G: GraphStorage> {
    vector_storage: Arc<V>,
    graph_storage: Arc<G>,
}

impl<V: VectorStorage, G: GraphStorage> GlobalStrategy<V, G> {
    /// Create a new global strategy.
    pub fn new(vector_storage: Arc<V>, graph_storage: Arc<G>) -> Self {
        Self {
            vector_storage,
            graph_storage,
        }
    }
}

#[async_trait]
impl<V: VectorStorage, G: GraphStorage> QueryStrategy for GlobalStrategy<V, G> {
    async fn execute(
        &self,
        _query: &str,
        query_embedding: &[f32],
        config: &StrategyConfig,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        // Step 1: Vector search for relationships (as per LightRAG Global mode spec)
        // Global mode should search relations_vdb
        let vector_results = self
            .vector_storage
            .query(query_embedding, config.max_entities * 3, None) // Get more for filtering
            .await?;

        // Filter to relationship vectors only
        let relationship_results = crate::vector_filter::filter_by_type(
            vector_results,
            crate::vector_filter::VectorType::Relationship,
        );

        let mut seen_relationships = HashSet::new();
        let mut entity_ids = HashSet::new();

        // Step 2: Extract relationships from vector results
        for result in relationship_results.iter().take(config.max_entities * 2) {
            if result.score >= config.min_score {
                let src_id = result
                    .metadata
                    .get("src_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let tgt_id = result
                    .metadata
                    .get("tgt_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let rel_type = result
                    .metadata
                    .get("relation_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("RELATED_TO");

                if !src_id.is_empty() && !tgt_id.is_empty() {
                    let rel_key = format!("{}->{}:{}", src_id, tgt_id, rel_type);

                    if seen_relationships.insert(rel_key) {
                        let description = result
                            .metadata
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        context.add_relationship(
                            RetrievedRelationship::new(src_id, tgt_id, rel_type.to_string())
                                .with_description(description),
                        );

                        // Track entities involved
                        entity_ids.insert(src_id.to_string());
                        entity_ids.insert(tgt_id.to_string());
                    }
                }
            }
        }

        // Step 3: Retrieve entity details for all entities in relationships
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
    global_strategy: GlobalStrategy<V, G>,
}

impl<V: VectorStorage, G: GraphStorage> HybridStrategy<V, G> {
    /// Create a new hybrid strategy.
    pub fn new(vector_storage: Arc<V>, graph_storage: Arc<G>) -> Self {
        Self {
            local_strategy: LocalStrategy::new(
                Arc::clone(&vector_storage),
                Arc::clone(&graph_storage),
            ),
            global_strategy: GlobalStrategy::new(vector_storage, graph_storage),
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

        let local_context = self
            .local_strategy
            .execute(query, query_embedding, &local_config)
            .await?;
        let global_context = self
            .global_strategy
            .execute(query, query_embedding, &global_config)
            .await?;

        // Merge contexts
        let mut merged = QueryContext::new();

        // Add local chunks first (more relevant)
        for chunk in &local_context.chunks {
            merged.add_chunk(chunk.clone());
        }

        // Merge entities (deduplicate)
        let mut seen_entities = HashSet::new();
        for entity in local_context
            .entities
            .iter()
            .chain(global_context.entities.iter())
        {
            if seen_entities.insert(entity.name.clone()) {
                merged.add_entity(entity.clone());
            }
        }

        // Merge relationships (deduplicate)
        let mut seen_rels = HashSet::new();
        for rel in local_context
            .relationships
            .iter()
            .chain(global_context.relationships.iter())
        {
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

        let naive_context = self
            .naive_strategy
            .execute(query, query_embedding, &naive_config)
            .await?;
        let hybrid_context = self
            .hybrid_strategy
            .execute(query, query_embedding, &hybrid_config)
            .await?;

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
        QueryMode::Local => Box::new(LocalStrategy::new(
            vector_storage.clone(),
            graph_storage.clone(),
        )),
        QueryMode::Global => Box::new(GlobalStrategy::new(vector_storage, graph_storage)),
        QueryMode::Hybrid => Box::new(HybridStrategy::new(vector_storage, graph_storage)),
        QueryMode::Mix => Box::new(MixStrategy::new(vector_storage, graph_storage)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::adapters::memory::{MemoryGraphStorage, MemoryVectorStorage};
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_strategy_config_default() {
        let config = StrategyConfig::default();
        assert_eq!(config.max_chunks, 20);
        assert_eq!(config.max_entities, 60);
        assert!((config.vector_weight - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_normalize_entity_name() {
        assert_eq!(normalize_entity_name("rust-lang"), "RUST LANG");
        assert_eq!(normalize_entity_name("hello_world"), "HELLO WORLD");
        assert_eq!(normalize_entity_name("  Test  "), "TEST");
    }

    #[test]
    fn test_strategy_config_custom() {
        let config = StrategyConfig {
            max_chunks: 5,
            max_entities: 10,
            max_relationships_per_entity: 3,
            graph_depth: 1,
            min_score: 0.2,
            vector_weight: 0.7,
            graph_weight: 0.3,
        };
        assert_eq!(config.max_chunks, 5);
        assert_eq!(config.graph_depth, 1);
        assert!((config.vector_weight - 0.7).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_naive_strategy_mode() {
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 3));
        let strategy = NaiveStrategy::new(vector_storage);
        assert_eq!(strategy.mode(), QueryMode::Naive);
    }

    #[tokio::test]
    async fn test_naive_strategy_empty_storage() {
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 3));
        vector_storage.initialize().await.unwrap();

        let strategy = NaiveStrategy::new(vector_storage);
        let config = StrategyConfig::default();

        let context = strategy
            .execute("test query", &[0.1, 0.2, 0.3], &config)
            .await
            .unwrap();
        assert!(context.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_naive_strategy_with_data() {
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 3));
        vector_storage.initialize().await.unwrap();

        // Insert some test vectors using the batch API
        let metadata = json!({
            "content": "Rust is a systems programming language.",
            "source": "test_doc"
        });
        let data = vec![("chunk1".to_string(), vec![0.1, 0.2, 0.3], metadata)];
        vector_storage.upsert(&data).await.unwrap();

        let strategy = NaiveStrategy::new(vector_storage);
        let config = StrategyConfig {
            min_score: 0.0,
            ..Default::default()
        };

        let context = strategy
            .execute("test", &[0.1, 0.2, 0.3], &config)
            .await
            .unwrap();
        assert_eq!(context.chunks.len(), 1);
        assert!(context.chunks[0].content.contains("Rust"));
    }

    #[tokio::test]
    async fn test_local_strategy_mode() {
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 3));
        let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
        let strategy = LocalStrategy::new(vector_storage, graph_storage);
        assert_eq!(strategy.mode(), QueryMode::Local);
    }

    #[tokio::test]
    async fn test_local_strategy_empty_storage() {
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 3));
        let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
        vector_storage.initialize().await.unwrap();
        graph_storage.initialize().await.unwrap();

        let strategy = LocalStrategy::new(vector_storage, graph_storage);
        let config = StrategyConfig::default();

        let context = strategy
            .execute("test query", &[0.1, 0.2, 0.3], &config)
            .await
            .unwrap();
        assert!(context.chunks.is_empty());
        assert!(context.entities.is_empty());
    }

    #[tokio::test]
    async fn test_global_strategy_mode() {
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 3));
        let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
        let strategy = GlobalStrategy::new(vector_storage, graph_storage);
        assert_eq!(strategy.mode(), QueryMode::Global);
    }

    #[tokio::test]
    async fn test_global_strategy_empty_storage() {
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 3));
        let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
        vector_storage.initialize().await.unwrap();
        graph_storage.initialize().await.unwrap();

        let strategy = GlobalStrategy::new(vector_storage, graph_storage);
        let config = StrategyConfig::default();

        let context = strategy
            .execute("test query", &[0.1, 0.2, 0.3], &config)
            .await
            .unwrap();
        assert!(context.entities.is_empty());
        assert!(context.relationships.is_empty());
    }

    #[tokio::test]
    async fn test_hybrid_strategy_mode() {
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 3));
        let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
        let strategy = HybridStrategy::new(vector_storage, graph_storage);
        assert_eq!(strategy.mode(), QueryMode::Hybrid);
    }

    #[tokio::test]
    async fn test_hybrid_strategy_empty_storage() {
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 3));
        let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
        vector_storage.initialize().await.unwrap();
        graph_storage.initialize().await.unwrap();

        let strategy = HybridStrategy::new(vector_storage, graph_storage);
        let config = StrategyConfig::default();

        let context = strategy
            .execute("test query", &[0.1, 0.2, 0.3], &config)
            .await
            .unwrap();
        assert!(context.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_mix_strategy_mode() {
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 3));
        let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
        let strategy = MixStrategy::new(vector_storage, graph_storage);
        assert_eq!(strategy.mode(), QueryMode::Mix);
    }

    #[tokio::test]
    async fn test_mix_strategy_empty_storage() {
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 3));
        let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
        vector_storage.initialize().await.unwrap();
        graph_storage.initialize().await.unwrap();

        let strategy = MixStrategy::new(vector_storage, graph_storage);
        let config = StrategyConfig::default();

        let context = strategy
            .execute("test query", &[0.1, 0.2, 0.3], &config)
            .await
            .unwrap();
        assert!(context.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_create_strategy_factory() {
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 3));
        let graph_storage = Arc::new(MemoryGraphStorage::new("test"));

        let naive = create_strategy(
            QueryMode::Naive,
            vector_storage.clone(),
            graph_storage.clone(),
        );
        assert_eq!(naive.mode(), QueryMode::Naive);

        let local = create_strategy(
            QueryMode::Local,
            vector_storage.clone(),
            graph_storage.clone(),
        );
        assert_eq!(local.mode(), QueryMode::Local);

        let global = create_strategy(
            QueryMode::Global,
            vector_storage.clone(),
            graph_storage.clone(),
        );
        assert_eq!(global.mode(), QueryMode::Global);

        let hybrid = create_strategy(
            QueryMode::Hybrid,
            vector_storage.clone(),
            graph_storage.clone(),
        );
        assert_eq!(hybrid.mode(), QueryMode::Hybrid);

        let mix = create_strategy(
            QueryMode::Mix,
            vector_storage.clone(),
            graph_storage.clone(),
        );
        assert_eq!(mix.mode(), QueryMode::Mix);
    }

    #[tokio::test]
    async fn test_strategy_with_graph_data() {
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 3));
        let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
        vector_storage.initialize().await.unwrap();
        graph_storage.initialize().await.unwrap();

        // Add a node to the graph using HashMap
        let mut props: HashMap<String, serde_json::Value> = HashMap::new();
        props.insert("entity_type".to_string(), json!("CONCEPT"));
        props.insert(
            "description".to_string(),
            json!("A systems programming language"),
        );
        graph_storage.upsert_node("RUST", props).await.unwrap();

        let strategy = GlobalStrategy::new(vector_storage, graph_storage);
        let config = StrategyConfig::default();

        // Query with "rust" term to match the entity
        let context = strategy
            .execute("rust language", &[0.1, 0.2, 0.3], &config)
            .await
            .unwrap();
        // Global strategy now looks for relationships through vector search
        // With empty relationship VDB, it should return empty context
        assert_eq!(context.entities.len(), 0);
        assert_eq!(context.relationships.len(), 0);
    }

    #[test]
    fn test_normalize_entity_name_special_chars() {
        assert_eq!(normalize_entity_name("C++"), "C++");
        assert_eq!(normalize_entity_name("node.js"), "NODE.JS");
        assert_eq!(normalize_entity_name("my-var_name"), "MY VAR NAME");
    }

    #[test]
    fn test_strategy_config_debug() {
        let config = StrategyConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("max_chunks"));
        assert!(debug_str.contains("20"));
    }

    #[test]
    fn test_strategy_config_clone() {
        let config = StrategyConfig::default();
        let cloned = config.clone();
        assert_eq!(config.max_chunks, cloned.max_chunks);
        assert_eq!(config.max_entities, cloned.max_entities);
    }
}
