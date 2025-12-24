//! Keyword extraction for RAG queries.
//!
//! This module provides LLM-based keyword extraction that identifies
//! high-level (conceptual) and low-level (specific) keywords from queries,
//! enabling global and mix query modes.
//!
//! Based on LightRAG's keyword extraction: `lightrag/prompt.py` - `PROMPTS["keywords_extraction"]`

use crate::error::{Error, Result};
use edgequake_llm::traits::LLMProvider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Keywords extracted from a query.
///
/// High-level keywords focus on overarching concepts or themes,
/// while low-level keywords focus on specific entities, details, or concrete terms.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractedKeywords {
    /// High-level conceptual keywords (for global mode)
    pub high_level: Vec<String>,
    /// Low-level specific keywords (for local mode)
    pub low_level: Vec<String>,
}

impl ExtractedKeywords {
    /// Create empty keywords.
    pub fn empty() -> Self {
        Self {
            high_level: Vec::new(),
            low_level: Vec::new(),
        }
    }

    /// Check if there are any keywords.
    pub fn is_empty(&self) -> bool {
        self.high_level.is_empty() && self.low_level.is_empty()
    }

    /// Get total number of keywords.
    pub fn len(&self) -> usize {
        self.high_level.len() + self.low_level.len()
    }

    /// Get all keywords as a single list.
    pub fn all(&self) -> Vec<&str> {
        self.high_level
            .iter()
            .chain(self.low_level.iter())
            .map(|s| s.as_str())
            .collect()
    }
}

/// Extracts keywords from queries for retrieval.
///
/// Uses an LLM to analyze the query and separate broad concepts
/// from specific terms. Results are cached to avoid redundant LLM calls.
pub struct KeywordExtractor {
    llm: Arc<dyn LLMProvider>,
    cache: RwLock<HashMap<String, ExtractedKeywords>>,
    cache_enabled: bool,
}

impl KeywordExtractor {
    /// Create a new keyword extractor.
    pub fn new(llm: Arc<dyn LLMProvider>) -> Self {
        Self {
            llm,
            cache: RwLock::new(HashMap::new()),
            cache_enabled: true,
        }
    }

    /// Enable or disable caching.
    pub fn with_cache(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        self
    }

    /// Extract keywords from a query.
    ///
    /// Uses an LLM to identify high-level conceptual keywords and
    /// low-level specific keywords from the query text.
    pub async fn extract(&self, query: &str) -> Result<ExtractedKeywords> {
        let query_normalized = query.trim();
        if query_normalized.is_empty() {
            return Ok(ExtractedKeywords::empty());
        }

        // Check cache first
        if self.cache_enabled {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(query_normalized) {
                tracing::debug!(query = %query_normalized, "Keyword cache hit");
                return Ok(cached.clone());
            }
        }

        // Extract keywords via LLM
        let prompt = self.build_extraction_prompt(query_normalized);
        let response =
            self.llm.complete(&prompt).await.map_err(|e| {
                Error::internal(format!("LLM error during keyword extraction: {}", e))
            })?;

        let keywords = self.parse_keywords(&response.content)?;

        tracing::debug!(
            query = %query_normalized,
            high_level = ?keywords.high_level,
            low_level = ?keywords.low_level,
            "Extracted keywords from query"
        );

        // Store in cache
        if self.cache_enabled {
            let mut cache = self.cache.write().await;
            cache.insert(query_normalized.to_string(), keywords.clone());
        }

        Ok(keywords)
    }

    /// Clear the keyword cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get the number of cached entries.
    pub async fn cache_size(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Build the keyword extraction prompt.
    ///
    /// This prompt is ported from LightRAG: `lightrag/prompt.py` - `PROMPTS["keywords_extraction"]`
    fn build_extraction_prompt(&self, query: &str) -> String {
        format!(
            r#"---Role---
You are a helpful assistant tasked with identifying both high-level and low-level keywords in the user's query.

---Goal---
Given the query, list both high-level and low-level keywords. High-level keywords focus on overarching concepts or themes, while low-level keywords focus on specific entities, details, or concrete terms.

---Instructions---
- Output the keywords in JSON format.
- The JSON should have two keys: "high_level_keywords" and "low_level_keywords".
- Each key should contain a list of strings (keywords).
- Extract 2-5 high-level keywords that capture the main themes or concepts.
- Extract 3-8 low-level keywords that are specific terms, entities, or details.
- Do not include stopwords or generic terms.

######################
-Examples-
######################
Example 1:
Query: "How does international trade influence global economic stability?"
################
Output:
{{
  "high_level_keywords": ["International trade", "Global economic stability", "Economic impact"],
  "low_level_keywords": ["Trade agreements", "Tariffs", "Currency exchange", "Imports", "Exports"]
}}
#############################
Example 2:
Query: "What is the role of mitochondria in cellular respiration?"
################
Output:
{{
  "high_level_keywords": ["Cellular respiration", "Energy production", "Cell biology"],
  "low_level_keywords": ["Mitochondria", "ATP", "Electron transport chain", "Krebs cycle", "Oxygen"]
}}
#############################
Example 3:
Query: "Describe the impact of climate change on polar bear populations"
################
Output:
{{
  "high_level_keywords": ["Climate change", "Wildlife conservation", "Arctic ecosystem"],
  "low_level_keywords": ["Polar bears", "Sea ice", "Habitat loss", "Population decline", "Melting glaciers"]
}}
#############################
-Real Data-
######################
Query: {query}
######################
Output:
"#,
            query = query
        )
    }

    /// Parse keywords from LLM response.
    fn parse_keywords(&self, response: &str) -> Result<ExtractedKeywords> {
        // Extract JSON from response (handle markdown code blocks)
        let json_str = self.extract_json(response);

        // Try to parse the JSON
        #[derive(Deserialize)]
        struct KeywordsResponse {
            high_level_keywords: Option<Vec<String>>,
            low_level_keywords: Option<Vec<String>>,
        }

        match serde_json::from_str::<KeywordsResponse>(json_str.trim()) {
            Ok(parsed) => Ok(ExtractedKeywords {
                high_level: parsed.high_level_keywords.unwrap_or_default(),
                low_level: parsed.low_level_keywords.unwrap_or_default(),
            }),
            Err(e) => {
                // Try alternate parsing strategies
                if let Some(keywords) = self.try_alternative_parse(response) {
                    Ok(keywords)
                } else {
                    tracing::warn!(
                        error = %e,
                        response = %response,
                        "Failed to parse keywords from LLM response"
                    );
                    // Return empty keywords rather than failing
                    Ok(ExtractedKeywords::empty())
                }
            }
        }
    }

    /// Extract JSON from a response that may contain markdown code blocks.
    fn extract_json<'a>(&self, response: &'a str) -> &'a str {
        // Try to find JSON in markdown code block
        if let Some(start) = response.find("```json") {
            let after_marker = &response[start + 7..];
            if let Some(end) = after_marker.find("```") {
                return after_marker[..end].trim();
            }
        }

        // Try generic code block
        if let Some(start) = response.find("```") {
            let after_marker = &response[start + 3..];
            if let Some(end) = after_marker.find("```") {
                return after_marker[..end].trim();
            }
        }

        // Try to find bare JSON object
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                return &response[start..=end];
            }
        }

        response.trim()
    }

    /// Try alternative parsing strategies for malformed responses.
    fn try_alternative_parse(&self, response: &str) -> Option<ExtractedKeywords> {
        // Try to extract keywords from a list-like format
        let mut high_level = Vec::new();
        let mut low_level = Vec::new();
        let mut current_section = None;

        for line in response.lines() {
            let line = line.trim().to_lowercase();

            if line.contains("high-level") || line.contains("high_level") || line.contains("themes")
            {
                current_section = Some("high");
            } else if line.contains("low-level")
                || line.contains("low_level")
                || line.contains("specific")
            {
                current_section = Some("low");
            } else if let Some(section) = current_section {
                // Try to extract keyword from line
                let keyword = line
                    .trim_start_matches(|c: char| !c.is_alphabetic())
                    .trim_end_matches(|c: char| !c.is_alphabetic())
                    .to_string();

                if !keyword.is_empty() && keyword.len() > 2 {
                    match section {
                        "high" => high_level.push(keyword),
                        "low" => low_level.push(keyword),
                        _ => {}
                    }
                }
            }
        }

        if high_level.is_empty() && low_level.is_empty() {
            None
        } else {
            Some(ExtractedKeywords {
                high_level,
                low_level,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_keywords_valid_json() {
        let extractor = KeywordExtractor::new(Arc::new(edgequake_llm::MockProvider::new()));

        let json = r#"{"high_level_keywords": ["AI", "ML"], "low_level_keywords": ["neural networks", "deep learning"]}"#;
        let result = extractor.parse_keywords(json).unwrap();

        assert_eq!(result.high_level, vec!["AI", "ML"]);
        assert_eq!(result.low_level, vec!["neural networks", "deep learning"]);
    }

    #[test]
    fn test_parse_keywords_with_code_block() {
        let extractor = KeywordExtractor::new(Arc::new(edgequake_llm::MockProvider::new()));

        let response = r#"Here are the keywords:
```json
{"high_level_keywords": ["Technology"], "low_level_keywords": ["Computer"]}
```
"#;
        let result = extractor.parse_keywords(response).unwrap();

        assert_eq!(result.high_level, vec!["Technology"]);
        assert_eq!(result.low_level, vec!["Computer"]);
    }

    #[test]
    fn test_parse_keywords_empty_response() {
        let extractor = KeywordExtractor::new(Arc::new(edgequake_llm::MockProvider::new()));

        let result = extractor.parse_keywords("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_extracted_keywords_methods() {
        let keywords = ExtractedKeywords {
            high_level: vec!["A".to_string(), "B".to_string()],
            low_level: vec!["C".to_string()],
        };

        assert!(!keywords.is_empty());
        assert_eq!(keywords.len(), 3);
        assert_eq!(keywords.all(), vec!["A", "B", "C"]);
    }

    #[test]
    fn test_empty_keywords() {
        let keywords = ExtractedKeywords::empty();
        assert!(keywords.is_empty());
        assert_eq!(keywords.len(), 0);
    }
}
