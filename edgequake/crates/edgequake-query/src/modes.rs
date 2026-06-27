//! Query modes for multi-strategy retrieval.
//!
//! # Implements
//!
//! - **FEAT0101**: Naive Mode - Vector similarity only
//! - **FEAT0102**: Local Mode - Entity-centric graph
//! - **FEAT0103**: Global Mode - Relationship-vector global search
//! - **FEAT0104**: Hybrid Mode - Local + Global combined
//! - **FEAT0105**: Mix Mode - Weighted naive + graph
//! - **FEAT0106**: Bypass Mode - Direct LLM (no RAG)
//!
//! # Enforces
//!
//! - **BR0103**: Query mode must be valid enum value (parsed via `FromStr`)
//!
//! # WHY Multiple Query Modes
//!
//! Different questions require different retrieval strategies. Consider:
//!
//! - "What is machine learning?" → **Naive** (simple concept lookup)
//! - "How does Alice work with Bob?" → **Local** (entity relationships)
//! - "What are the main themes in this document?" → **Global** (relationship vectors + degree fallback)
//! - "Tell me about Project X and its impact" → **Hybrid** (entities + context)
//!
//! ## Mode Selection Guidelines
//!
//! | Question Type | Best Mode | FEAT | Why |
//! |--------------|-----------|------|-----|
//! | Factual/specific | Naive | FEAT0101 | Direct vector match, fast |
//! | Entity relationships | Local | FEAT0102 | Explores entity neighborhood |
//! | Broad/thematic | Global | FEAT0103 | Relationship-vector search; not GraphRAG community reports |
//! | Complex/multi-faceted | Hybrid | FEAT0104 | Both approaches combined |
//! | Custom weights needed | Mix | FEAT0105 | Configurable blend |
//! | Testing/debugging | Bypass | FEAT0106 | Skip RAG entirely |
//!
//! ## Performance vs Accuracy Trade-offs
//!
//! ```text
//! Mode    | Speed | Accuracy | Context Size
//! --------|-------|----------|-------------
//! Naive   | Fast  | Good     | Small (chunks only)
//! Local   | Med   | High     | Medium (entity + neighbors)
//! Global  | Med   | High     | Medium (relationship vectors + graph context)
//! Hybrid  | Slow  | Best     | Large (both approaches)
//! ```
//!
//! Mix is the production default (runtime + serde). Hybrid remains the LightRAG
//! round-robin interleave mode for explicit API requests.

use serde::{Deserialize, Serialize};

/// Query mode determining the search strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum QueryMode {
    /// Simple vector similarity search on chunks.
    /// Fast but misses graph relationships.
    Naive,

    /// Entity-centric search with local neighborhood.
    /// Good for specific entity queries.
    Local,

    /// Relationship-centric global search using high-level embeddings over
    /// relationship vectors, with degree-based fallback when no relationship
    /// vectors match. **Not** GraphRAG hierarchical community reports (SPEC-023 I2).
    Global,

    /// Weighted combination of naive and graph-based retrieval (FEAT0105 / P-G8).
    ///
    /// Runs the Local, Global, and Naive arms in parallel and blends them by
    /// *weighted score* (min-max normalized per arm, then weighted sum), not
    /// round-robin. Weights are `QueryEngineConfig::{mix_local_weight,
    /// mix_global_weight, mix_naive_weight}` and need not sum to 1. Equal
    /// weights preserve Hybrid's ordering on identical fixtures.
    ///
    /// **Production default** (runtime + serde): Mix with RRF fusion.
    #[default]
    Mix,

    /// Combines local and global approaches via round-robin interleave (LightRAG Hybrid).
    /// Balances specificity and coverage.
    Hybrid,

    /// Direct LLM query without RAG retrieval (FEAT0106).
    Bypass,
}

impl QueryMode {
    /// Get all available query modes.
    pub fn all() -> Vec<Self> {
        vec![
            Self::Naive,
            Self::Local,
            Self::Global,
            Self::Hybrid,
            Self::Mix,
            Self::Bypass,
        ]
    }

    /// Get the mode name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Naive => "naive",
            Self::Local => "local",
            Self::Global => "global",
            Self::Hybrid => "hybrid",
            Self::Mix => "mix",
            Self::Bypass => "bypass",
        }
    }

    /// Parse a mode from string (returns Option for backward compatibility).
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Whether this mode uses vector search.
    pub fn uses_vector_search(&self) -> bool {
        matches!(self, Self::Naive | Self::Local | Self::Hybrid | Self::Mix)
    }

    /// Whether this mode uses graph traversal.
    pub fn uses_graph(&self) -> bool {
        matches!(self, Self::Local | Self::Global | Self::Hybrid | Self::Mix)
    }

    /// Whether this mode skips retrieval entirely.
    pub fn is_bypass(&self) -> bool {
        matches!(self, Self::Bypass)
    }
}

use std::str::FromStr;

impl FromStr for QueryMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "naive" => Ok(Self::Naive),
            "local" => Ok(Self::Local),
            "global" => Ok(Self::Global),
            "hybrid" => Ok(Self::Hybrid),
            "mix" => Ok(Self::Mix),
            "bypass" => Ok(Self::Bypass),
            other => Err(format!("Unknown query mode: {}", other)),
        }
    }
}

impl std::fmt::Display for QueryMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_mode_all() {
        let modes = QueryMode::all();
        assert_eq!(modes.len(), 6);
    }

    #[test]
    fn test_query_mode_parsing() {
        assert_eq!(QueryMode::parse("naive"), Some(QueryMode::Naive));
        assert_eq!(QueryMode::parse("HYBRID"), Some(QueryMode::Hybrid));
        assert_eq!(QueryMode::parse("bypass"), Some(QueryMode::Bypass));
        assert_eq!(QueryMode::parse("unknown"), None);
    }

    #[test]
    fn test_query_mode_features() {
        assert!(QueryMode::Naive.uses_vector_search());
        assert!(!QueryMode::Naive.uses_graph());

        // Hybrid uses BOTH graph AND vector search for comprehensive retrieval
        assert!(QueryMode::Hybrid.uses_graph());
        assert!(QueryMode::Hybrid.uses_vector_search());

        assert!(QueryMode::Mix.uses_vector_search());
        assert!(QueryMode::Mix.uses_graph());
    }

    #[test]
    fn test_query_mode_display() {
        assert_eq!(format!("{}", QueryMode::Local), "local");
    }
}
