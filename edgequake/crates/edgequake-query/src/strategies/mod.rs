//! Legacy query strategies (benchmark-only — use `SOTAQueryEngine` in production).
//!
//! @deprecated since SPEC-017: Superseded by `sota_engine::query_pipeline`.
//! Retained for `benches/query_bench.rs` performance comparisons only.

mod config;
mod global;
mod hybrid;
mod local;
mod mix;
mod naive;

pub use config::{QueryStrategy, StrategyConfig};
pub use global::GlobalStrategy;
pub use hybrid::HybridStrategy;
pub use local::LocalStrategy;
pub use mix::MixStrategy;
pub use naive::NaiveStrategy;

use std::sync::Arc;

use crate::modes::QueryMode;
use edgequake_storage::traits::{GraphStorage, VectorStorage};

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
        QueryMode::Bypass => Box::new(NaiveStrategy::new(vector_storage)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::adapters::memory::{MemoryGraphStorage, MemoryVectorStorage};
    use edgequake_storage::GraphStorageMutateOps;
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
