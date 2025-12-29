//! Query modes.

use serde::{Deserialize, Serialize};

/// Query mode determining the search strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryMode {
    /// Simple vector similarity search on chunks.
    /// Fast but misses graph relationships.
    Naive,

    /// Entity-centric search with local neighborhood.
    /// Good for specific entity queries.
    Local,

    /// Community-based search using graph clusters.
    /// Good for broad topic queries.
    Global,

    /// Combines local and global approaches.
    /// Balances specificity and coverage.
    Hybrid,

    /// Weighted combination of naive and graph-based.
    /// Most flexible, configurable weights.
    Mix,
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
        }
    }

    /// Parse a mode from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "naive" => Some(Self::Naive),
            "local" => Some(Self::Local),
            "global" => Some(Self::Global),
            "hybrid" => Some(Self::Hybrid),
            "mix" => Some(Self::Mix),
            _ => None,
        }
    }

    /// Whether this mode uses vector search.
    pub fn uses_vector_search(&self) -> bool {
        // Hybrid should use BOTH vector search AND graph traversal
        matches!(self, Self::Naive | Self::Local | Self::Hybrid | Self::Mix)
    }

    /// Whether this mode uses graph traversal.
    pub fn uses_graph(&self) -> bool {
        matches!(self, Self::Local | Self::Global | Self::Hybrid | Self::Mix)
    }
}

impl Default for QueryMode {
    fn default() -> Self {
        Self::Hybrid
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
        assert_eq!(modes.len(), 5);
    }

    #[test]
    fn test_query_mode_parsing() {
        assert_eq!(QueryMode::from_str("naive"), Some(QueryMode::Naive));
        assert_eq!(QueryMode::from_str("HYBRID"), Some(QueryMode::Hybrid));
        assert_eq!(QueryMode::from_str("unknown"), None);
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
