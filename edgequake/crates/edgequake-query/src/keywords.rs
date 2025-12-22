//! Keyword extraction for query processing.
//!
//! This module provides functionality to extract high-level and low-level keywords
//! from queries to improve retrieval quality.
//!
//! - **High-level keywords**: Abstract concepts, themes, topics (used in Global mode)
//! - **Low-level keywords**: Specific entities, technical terms (used in Local mode)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::{QueryError, Result};
use edgequake_llm::LLMProvider;

/// Extracted keywords from a query.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Keywords {
    /// High-level keywords: concepts, themes, topics.
    pub high_level: Vec<String>,

    /// Low-level keywords: entities, specific terms.
    pub low_level: Vec<String>,
}

impl Keywords {
    /// Create new keywords.
    pub fn new(high_level: Vec<String>, low_level: Vec<String>) -> Self {
        Self {
            high_level,
            low_level,
        }
    }

    /// Create empty keywords.
    pub fn empty() -> Self {
        Self {
            high_level: Vec::new(),
            low_level: Vec::new(),
        }
    }

    /// Check if both levels are empty.
    pub fn is_empty(&self) -> bool {
        self.high_level.is_empty() && self.low_level.is_empty()
    }

    /// Get all keywords combined.
    pub fn all_keywords(&self) -> Vec<String> {
        let mut all = self.high_level.clone();
        all.extend(self.low_level.clone());
        all
    }
}

/// Trait for keyword extraction.
#[async_trait]
pub trait KeywordExtractor: Send + Sync {
    /// Extract high-level and low-level keywords from a query.
    async fn extract(&self, query: &str) -> Result<Keywords>;
}

/// LLM-based keyword extractor.
pub struct LLMKeywordExtractor {
    llm_provider: Arc<dyn LLMProvider>,
}

impl LLMKeywordExtractor {
    /// Create a new LLM keyword extractor.
    pub fn new(llm_provider: Arc<dyn LLMProvider>) -> Self {
        Self { llm_provider }
    }

    /// Build the keyword extraction prompt.
    fn build_prompt(&self, query: &str) -> String {
        format!(
            r#"Extract high-level and low-level keywords from the following query.

High-level keywords are abstract concepts, themes, or topics (e.g., "artificial intelligence", "climate change", "software architecture").
Low-level keywords are specific entities, technical terms, or proper nouns (e.g., "GPT-4", "neural network", "PostgreSQL").

Query: "{query}"

Respond ONLY with valid JSON in this exact format:
{{
  "high_level_keywords": ["concept1", "concept2"],
  "low_level_keywords": ["entity1", "entity2", "term1"]
}}

Examples:

Query: "How does machine learning improve healthcare outcomes?"
{{
  "high_level_keywords": ["machine learning", "healthcare", "outcomes", "improvement"],
  "low_level_keywords": ["ML algorithms", "medical diagnosis", "patient data"]
}}

Query: "What is the relationship between OpenAI and Microsoft?"
{{
  "high_level_keywords": ["business relationship", "partnership", "collaboration"],
  "low_level_keywords": ["OpenAI", "Microsoft", "GPT", "Azure"]
}}

Query: "Explain quantum computing applications in cryptography"
{{
  "high_level_keywords": ["quantum computing", "applications", "cryptography", "security"],
  "low_level_keywords": ["qubits", "Shor's algorithm", "quantum key distribution", "encryption"]
}}

Now extract keywords from the query above. Respond with JSON only:"#
        )
    }
}

#[async_trait]
impl KeywordExtractor for LLMKeywordExtractor {
    async fn extract(&self, query: &str) -> Result<Keywords> {
        // Build prompt
        let prompt = self.build_prompt(query);

        // Call LLM
        let response = self
            .llm_provider
            .complete(&prompt)
            .await
            .map_err(|e| QueryError::LlmError(e))?;

        // Parse JSON response
        let keywords: Keywords = serde_json::from_str(&response.content).map_err(|e| {
            QueryError::Internal(format!("Failed to parse keywords JSON: {}", e))
        })?;

        Ok(keywords)
    }
}

/// Mock keyword extractor for testing.
pub struct MockKeywordExtractor {
    /// Pre-configured responses for testing.
    responses: std::sync::RwLock<Vec<Keywords>>,
}

impl MockKeywordExtractor {
    /// Create a new mock extractor.
    pub fn new() -> Self {
        Self {
            responses: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// Add a response to return.
    pub fn add_response(&self, keywords: Keywords) {
        self.responses.write().unwrap().push(keywords);
    }

    /// Create a mock with simple word extraction (splits on spaces).
    pub fn with_simple_extraction() -> Self {
        Self::new()
    }
}

impl Default for MockKeywordExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl KeywordExtractor for MockKeywordExtractor {
    async fn extract(&self, query: &str) -> Result<Keywords> {
        // Try to pop a pre-configured response
        if let Ok(mut responses) = self.responses.write() {
            if !responses.is_empty() {
                return Ok(responses.remove(0));
            }
        }

        // Fallback: Simple word-based extraction
        let words: Vec<String> = query
            .split_whitespace()
            .filter(|w| w.len() > 3) // Filter short words
            .map(|w| {
                w.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase()
            })
            .collect();

        // Split into high/low level (simple heuristic)
        let mid = words.len() / 2;
        let high_level = words[..mid].to_vec();
        let low_level = words[mid..].to_vec();

        Ok(Keywords::new(high_level, low_level))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords_creation() {
        let keywords = Keywords::new(
            vec!["AI".to_string(), "healthcare".to_string()],
            vec!["GPT-4".to_string(), "diagnosis".to_string()],
        );

        assert_eq!(keywords.high_level.len(), 2);
        assert_eq!(keywords.low_level.len(), 2);
        assert!(!keywords.is_empty());
    }

    #[test]
    fn test_empty_keywords() {
        let keywords = Keywords::empty();
        assert!(keywords.is_empty());
        assert_eq!(keywords.all_keywords().len(), 0);
    }

    #[test]
    fn test_all_keywords() {
        let keywords = Keywords::new(vec!["concept".to_string()], vec!["entity".to_string()]);

        let all = keywords.all_keywords();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&"concept".to_string()));
        assert!(all.contains(&"entity".to_string()));
    }

    #[tokio::test]
    async fn test_mock_extractor_with_response() {
        let mock = MockKeywordExtractor::new();
        let expected = Keywords::new(vec!["AI".to_string()], vec!["GPT".to_string()]);
        mock.add_response(expected.clone());

        let result = mock.extract("test query").await.unwrap();
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_mock_extractor_simple_extraction() {
        let mock = MockKeywordExtractor::with_simple_extraction();
        let result = mock.extract("machine learning healthcare").await.unwrap();

        assert!(!result.is_empty());
        assert!(result.high_level.len() > 0 || result.low_level.len() > 0);
    }

    #[test]
    fn test_llm_extractor_prompt_generation() {
        let llm = Arc::new(edgequake_llm::MockProvider::new());
        let extractor = LLMKeywordExtractor::new(llm);

        let prompt = extractor.build_prompt("What is AI?");
        assert!(prompt.contains("What is AI?"));
        assert!(prompt.contains("high_level_keywords"));
        assert!(prompt.contains("low_level_keywords"));
        assert!(prompt.contains("JSON"));
    }
}
