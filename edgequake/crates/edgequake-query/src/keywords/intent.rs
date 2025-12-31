//! Query intent classification for adaptive retrieval strategy.
//!
//! Query intent determines which retrieval strategy to use:
//! - Factual: Entity-focused (Local mode preferred)
//! - Relational: Relationship-focused (Global mode preferred)  
//! - Exploratory: Broad coverage (Hybrid mode preferred)
//! - Comparative: Multi-entity (Special handling)
//! - Procedural: Step-by-step (Chunk-focused)

use serde::{Deserialize, Serialize};

/// Query intent classification for adaptive retrieval.
///
/// This is a SOTA innovation beyond LightRAG - we classify
/// the query intent to select the optimal retrieval strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum QueryIntent {
    /// "What is X?" - Facts about a single entity
    /// Preferred mode: Local (entity-centric)
    Factual,

    /// "How does X relate to Y?" - Connections between entities
    /// Preferred mode: Global (relationship-centric)
    Relational,

    /// "Tell me about X" - Broad exploration
    /// Preferred mode: Hybrid (comprehensive)
    #[default]
    Exploratory,

    /// "Compare X and Y" - Multiple entities in parallel
    /// Preferred mode: Hybrid with parallel entity retrieval
    Comparative,

    /// "How to do X?" - Step-by-step instructions
    /// Preferred mode: Mix (chunks important for procedures)
    Procedural,
}

impl QueryIntent {
    /// Get the recommended query mode for this intent.
    pub fn recommended_mode(&self) -> crate::modes::QueryMode {
        match self {
            QueryIntent::Factual => crate::modes::QueryMode::Local,
            QueryIntent::Relational => crate::modes::QueryMode::Global,
            QueryIntent::Exploratory => crate::modes::QueryMode::Hybrid,
            QueryIntent::Comparative => crate::modes::QueryMode::Hybrid,
            QueryIntent::Procedural => crate::modes::QueryMode::Mix,
        }
    }

    /// Parse from string (case-insensitive).
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "factual" => QueryIntent::Factual,
            "relational" => QueryIntent::Relational,
            "exploratory" => QueryIntent::Exploratory,
            "comparative" => QueryIntent::Comparative,
            "procedural" => QueryIntent::Procedural,
            _ => QueryIntent::Exploratory, // Default fallback
        }
    }

    /// Heuristic classification based on query text patterns.
    ///
    /// This is a fast fallback when LLM classification is unavailable.
    pub fn classify_heuristic(query: &str) -> Self {
        let lower = query.to_lowercase();

        // Procedural indicators
        if lower.starts_with("how to ")
            || lower.starts_with("how do ")
            || lower.contains("step by step")
            || lower.contains("instructions")
            || lower.contains("guide")
        {
            return QueryIntent::Procedural;
        }

        // Comparative indicators
        if lower.contains(" vs ")
            || lower.contains(" versus ")
            || lower.contains("compare ")
            || lower.contains("difference between")
            || lower.contains("similarities between")
        {
            return QueryIntent::Comparative;
        }

        // Relational indicators
        if lower.contains(" relate ")
            || lower.contains("relationship between")
            || lower.contains("connection between")
            || lower.contains("linked to")
            || lower.contains("associated with")
            || lower.starts_with("how does ")
            || lower.starts_with("how are ")
        {
            return QueryIntent::Relational;
        }

        // Factual indicators
        if lower.starts_with("what is ")
            || lower.starts_with("what are ")
            || lower.starts_with("who is ")
            || lower.starts_with("who are ")
            || lower.starts_with("when ")
            || lower.starts_with("where ")
            || lower.starts_with("define ")
        {
            return QueryIntent::Factual;
        }

        // Exploratory indicators (default)
        if lower.starts_with("tell me about")
            || lower.starts_with("explain ")
            || lower.starts_with("describe ")
            || lower.contains("overview")
        {
            return QueryIntent::Exploratory;
        }

        // Default to exploratory for unknown patterns
        QueryIntent::Exploratory
    }
}

impl std::fmt::Display for QueryIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryIntent::Factual => write!(f, "factual"),
            QueryIntent::Relational => write!(f, "relational"),
            QueryIntent::Exploratory => write!(f, "exploratory"),
            QueryIntent::Comparative => write!(f, "comparative"),
            QueryIntent::Procedural => write!(f, "procedural"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factual_detection() {
        assert_eq!(
            QueryIntent::classify_heuristic("What is machine learning?"),
            QueryIntent::Factual
        );
        assert_eq!(
            QueryIntent::classify_heuristic("Who is Sarah Chen?"),
            QueryIntent::Factual
        );
    }

    #[test]
    fn test_relational_detection() {
        assert_eq!(
            QueryIntent::classify_heuristic("How does Sarah relate to the project?"),
            QueryIntent::Relational
        );
        assert_eq!(
            QueryIntent::classify_heuristic("What is the relationship between A and B?"),
            QueryIntent::Relational
        );
    }

    #[test]
    fn test_comparative_detection() {
        assert_eq!(
            QueryIntent::classify_heuristic("Compare Python vs Rust"),
            QueryIntent::Comparative
        );
        assert_eq!(
            QueryIntent::classify_heuristic("What's the difference between X and Y?"),
            QueryIntent::Comparative
        );
    }

    #[test]
    fn test_procedural_detection() {
        assert_eq!(
            QueryIntent::classify_heuristic("How to install PostgreSQL?"),
            QueryIntent::Procedural
        );
        assert_eq!(
            QueryIntent::classify_heuristic("Step by step guide to setup"),
            QueryIntent::Procedural
        );
    }

    #[test]
    fn test_exploratory_detection() {
        assert_eq!(
            QueryIntent::classify_heuristic("Tell me about the project"),
            QueryIntent::Exploratory
        );
        assert_eq!(
            QueryIntent::classify_heuristic("Explain quantum computing"),
            QueryIntent::Exploratory
        );
    }

    #[test]
    fn test_default_to_exploratory() {
        assert_eq!(
            QueryIntent::classify_heuristic("Random query without clear intent"),
            QueryIntent::Exploratory
        );
    }

    #[test]
    fn test_recommended_modes() {
        assert_eq!(
            QueryIntent::Factual.recommended_mode(),
            crate::modes::QueryMode::Local
        );
        assert_eq!(
            QueryIntent::Relational.recommended_mode(),
            crate::modes::QueryMode::Global
        );
    }
}
