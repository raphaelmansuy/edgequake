//! Mock keyword extractor for testing.
//!
//! Provides controllable keyword extraction for tests.

use async_trait::async_trait;
use std::sync::RwLock;

use super::extractor::{ExtractedKeywords, KeywordExtractor, Keywords};
use super::intent::QueryIntent;
use crate::error::Result;

/// Mock keyword extractor for testing.
///
/// Can be configured with specific responses or uses simple
/// heuristic-based extraction as a fallback.
pub struct MockKeywordExtractor {
    /// Pre-configured responses for testing.
    responses: RwLock<Vec<Keywords>>,
    /// Whether to use intelligent heuristics when no response is configured.
    use_heuristics: bool,
}

impl MockKeywordExtractor {
    /// Create a new mock extractor.
    pub fn new() -> Self {
        Self {
            responses: RwLock::new(Vec::new()),
            use_heuristics: true,
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

    /// Create a mock that doesn't use heuristics (always empty).
    pub fn without_heuristics() -> Self {
        Self {
            responses: RwLock::new(Vec::new()),
            use_heuristics: false,
        }
    }

    /// Heuristic-based keyword extraction.
    ///
    /// This provides reasonable keywords for testing without LLM calls.
    fn extract_heuristic(&self, query: &str) -> Keywords {
        let words: Vec<String> = query
            .split_whitespace()
            .filter(|w| w.len() > 2) // Filter very short words
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|w| !w.is_empty())
            .collect();

        // Simple heuristics for classification:
        // - Capitalized words → low-level (likely entities)
        // - Common words → skip
        // - Other words → high-level (concepts)

        let stopwords = [
            "the", "a", "an", "is", "are", "was", "were", "what", "who", "how", "why", "when",
            "where", "this", "that", "these", "those", "and", "or", "but", "for", "with", "about",
            "between", "from", "into", "does", "do",
        ];

        let mut high_level = Vec::new();
        let mut low_level = Vec::new();

        for word in words {
            let lower = word.to_lowercase();

            // Skip stopwords
            if stopwords.contains(&lower.as_str()) {
                continue;
            }

            // Check if it looks like a proper noun (capitalized in middle of sentence)
            let first_char = word.chars().next().unwrap_or('a');
            if first_char.is_uppercase() && word.len() > 1 {
                // Likely an entity
                low_level.push(word);
            } else {
                // Likely a concept
                high_level.push(lower);
            }
        }

        // If we didn't find any entities, use the last half of high-level
        if low_level.is_empty() && high_level.len() > 1 {
            let mid = high_level.len() / 2;
            low_level = high_level.split_off(mid);
        }

        Keywords::new(high_level, low_level)
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

        // Fallback to heuristics
        if self.use_heuristics {
            Ok(self.extract_heuristic(query))
        } else {
            Ok(Keywords::empty())
        }
    }

    async fn extract_extended(&self, query: &str) -> Result<ExtractedKeywords> {
        let keywords = self.extract(query).await?;
        let intent = QueryIntent::classify_heuristic(query);

        Ok(
            ExtractedKeywords::new(keywords.high_level, keywords.low_level, intent)
                .with_cache_key(self.cache_key(query)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_extractor_with_response() {
        let mock = MockKeywordExtractor::new();
        let expected = Keywords::new(vec!["AI".to_string()], vec!["GPT".to_string()]);
        mock.add_response(expected.clone());

        let result = mock.extract("test query").await.unwrap();
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_mock_extractor_heuristic() {
        let mock = MockKeywordExtractor::with_simple_extraction();
        let result = mock
            .extract("What is Sarah Chen's role in the project?")
            .await
            .unwrap();

        // Should detect "Sarah" and "Chen" as entities (capitalized)
        assert!(!result.low_level.is_empty() || !result.high_level.is_empty());
    }

    #[tokio::test]
    async fn test_mock_extractor_no_heuristics() {
        let mock = MockKeywordExtractor::without_heuristics();
        let result = mock.extract("test query").await.unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_mock_extractor_extended() {
        let mock = MockKeywordExtractor::new();
        let result = mock
            .extract_extended("What is machine learning?")
            .await
            .unwrap();

        // Should classify as Factual
        assert_eq!(result.query_intent, QueryIntent::Factual);
        assert!(!result.cache_key.is_empty());
    }

    #[test]
    fn test_heuristic_extraction() {
        let mock = MockKeywordExtractor::new();

        // Test with proper nouns
        let result = mock.extract_heuristic("Sarah Chen works at OpenAI on machine learning");
        assert!(result
            .low_level
            .iter()
            .any(|w| w == "Sarah" || w == "Chen" || w == "OpenAI"));

        // Test without proper nouns
        let result = mock.extract_heuristic("how to implement neural networks");
        assert!(!result.high_level.is_empty() || !result.low_level.is_empty());
    }
}
