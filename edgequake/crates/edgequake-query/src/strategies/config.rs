//! Configuration and trait definitions for query strategies.

use async_trait::async_trait;

use crate::context::QueryContext;
use crate::error::Result;
use crate::modes::QueryMode;

/// Configuration for query strategies.
///
/// WHY: vector_weight + graph_weight are advisory blend coefficients, not
/// required to sum to 1.0. Strategies normalise internally. Defaults
/// (0.5 / 0.5) give equal weight to semantic similarity and graph
/// topology — the LightRAG-parity baseline.
///
/// ```text
///  Query ──► VectorSearch ──(× vector_weight)──┐
///            GraphSearch  ──(× graph_weight)───┤► merge & rank
/// ```
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_config_defaults() {
        let c = StrategyConfig::default();
        assert_eq!(c.max_chunks, 20);
        assert_eq!(c.max_entities, 60);
        assert_eq!(c.max_relationships_per_entity, 5);
        assert_eq!(c.graph_depth, 2);
        assert!((c.min_score - 0.1).abs() < f32::EPSILON);
        assert!((c.vector_weight - 0.5).abs() < f32::EPSILON);
        assert!((c.graph_weight - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_strategy_config_custom() {
        let c = StrategyConfig {
            max_chunks: 10,
            max_entities: 30,
            max_relationships_per_entity: 3,
            graph_depth: 1,
            min_score: 0.5,
            vector_weight: 0.7,
            graph_weight: 0.3,
        };
        assert_eq!(c.max_chunks, 10);
        assert!((c.vector_weight - 0.7).abs() < f32::EPSILON);
    }
}
