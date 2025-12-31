//! Core keyword extraction traits and types.
//!
//! This module defines the core abstractions for keyword extraction
//! that are implemented by LLM-based and mock extractors.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::intent::QueryIntent;
use crate::error::Result;

/// Extracted keywords from a query.
///
/// This is the simple form without metadata, used for backward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Keywords {
    /// High-level keywords: concepts, themes, topics.
    /// Used in Global mode to search relationship vectors.
    pub high_level: Vec<String>,

    /// Low-level keywords: entities, specific terms.
    /// Used in Local mode to search entity vectors.
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

    /// Convert to extended keywords with default intent.
    pub fn into_extracted(self) -> ExtractedKeywords {
        ExtractedKeywords {
            high_level: self.high_level,
            low_level: self.low_level,
            query_intent: QueryIntent::Exploratory,
            cache_key: String::new(),
            extracted_at: chrono::Utc::now(),
        }
    }
}

impl Default for Keywords {
    fn default() -> Self {
        Self::empty()
    }
}

/// Extended keywords with metadata for caching and adaptive retrieval.
///
/// This is the SOTA form that includes:
/// - Query intent classification
/// - Cache key for deduplication
/// - Extraction timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedKeywords {
    /// High-level keywords: concepts, themes, topics.
    pub high_level: Vec<String>,

    /// Low-level keywords: entities, specific terms.
    pub low_level: Vec<String>,

    /// Classified query intent for adaptive retrieval.
    pub query_intent: QueryIntent,

    /// Cache key (hash of query + mode).
    pub cache_key: String,

    /// When keywords were extracted.
    pub extracted_at: chrono::DateTime<chrono::Utc>,
}

impl ExtractedKeywords {
    /// Create new extracted keywords.
    pub fn new(high_level: Vec<String>, low_level: Vec<String>, query_intent: QueryIntent) -> Self {
        Self {
            high_level,
            low_level,
            query_intent,
            cache_key: String::new(),
            extracted_at: chrono::Utc::now(),
        }
    }

    /// Set the cache key.
    pub fn with_cache_key(mut self, key: String) -> Self {
        self.cache_key = key;
        self
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

    /// Convert to simple Keywords (drops metadata).
    pub fn to_simple(&self) -> Keywords {
        Keywords {
            high_level: self.high_level.clone(),
            low_level: self.low_level.clone(),
        }
    }
}

impl From<Keywords> for ExtractedKeywords {
    fn from(keywords: Keywords) -> Self {
        keywords.into_extracted()
    }
}

impl From<ExtractedKeywords> for Keywords {
    fn from(extracted: ExtractedKeywords) -> Self {
        extracted.to_simple()
    }
}

/// Trait for keyword extraction.
///
/// Implementations must extract high-level and low-level keywords
/// from a query string. The extraction can be:
/// - LLM-based (production, accurate)
/// - Rule-based (fast, less accurate)
/// - Mock (testing)
#[async_trait]
pub trait KeywordExtractor: Send + Sync {
    /// Extract keywords from a query.
    ///
    /// Returns simple Keywords for backward compatibility.
    async fn extract(&self, query: &str) -> Result<Keywords>;

    /// Extract keywords with full metadata.
    ///
    /// Default implementation wraps `extract()` and adds metadata.
    async fn extract_extended(&self, query: &str) -> Result<ExtractedKeywords> {
        let keywords = self.extract(query).await?;
        let intent = QueryIntent::classify_heuristic(query);
        Ok(ExtractedKeywords::new(
            keywords.high_level,
            keywords.low_level,
            intent,
        ))
    }

    /// Extract keywords with caching hint.
    ///
    /// Implementations that support caching can use this to compute
    /// a stable cache key.
    fn cache_key(&self, query: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        hex::encode(hasher.finalize())
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

    #[test]
    fn test_extracted_keywords_conversion() {
        let simple = Keywords::new(vec!["theme".to_string()], vec!["entity".to_string()]);

        let extracted: ExtractedKeywords = simple.clone().into();
        assert_eq!(extracted.high_level, simple.high_level);
        assert_eq!(extracted.low_level, simple.low_level);

        let back: Keywords = extracted.into();
        assert_eq!(back.high_level, simple.high_level);
    }
}
