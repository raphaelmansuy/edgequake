//! LLM-based keyword extraction with caching.
//!
//! This module provides the production-grade keyword extractor
//! that uses an LLM to extract high-level and low-level keywords.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{QueryError, Result};
use edgequake_llm::LLMProvider;

use super::cache::KeywordCache;
use super::extractor::{ExtractedKeywords, KeywordExtractor, Keywords};
use super::intent::QueryIntent;

/// LLM-based keyword extractor.
///
/// Uses a language model to extract semantically meaningful keywords
/// from queries, matching the LightRAG approach.
pub struct LLMKeywordExtractor {
    llm_provider: Arc<dyn LLMProvider>,
}

impl LLMKeywordExtractor {
    /// Create a new LLM keyword extractor.
    pub fn new(llm_provider: Arc<dyn LLMProvider>) -> Self {
        Self { llm_provider }
    }

    /// Extract keywords using a specific LLM provider override.
    ///
    /// WHY: This method exists to support user-selected LLM providers.
    /// When a user explicitly chooses an LLM (e.g., OpenAI GPT-4) for their query,
    /// ALL LLM operations in the query pipeline must use that same provider,
    /// not the server's default. This includes keyword extraction.
    ///
    /// CRITICAL: Without this method, keyword extraction would always use
    /// the server's default LLM (often Ollama), even when the user selected
    /// a different provider. This leads to inconsistent behavior and unexpected costs.
    ///
    /// # Arguments
    /// * `query` - The query text to extract keywords from
    /// * `llm_override` - The user-selected LLM provider to use for extraction
    ///
    /// # Returns
    /// Extracted keywords with high-level concepts, low-level entities, and query intent
    ///
    /// # Example
    /// ```rust,ignore
    /// // User selected OpenAI GPT-4 in the UI
    /// let user_llm = openai_provider.clone();
    /// let keywords = extractor.extract_with_provider(query, user_llm).await?;
    /// // Now keyword extraction uses OpenAI, not the default Ollama
    /// ```
    pub async fn extract_with_provider(
        &self,
        query: &str,
        llm_override: Arc<dyn LLMProvider>,
    ) -> Result<ExtractedKeywords> {
        let prompt = self.build_prompt(query);

        // Use the provided LLM override instead of self.llm_provider
        let response = llm_override
            .complete(&prompt)
            .await
            .map_err(QueryError::from)?;

        let mut extracted = self.parse_response(&response.content)?;
        extracted.cache_key = self.cache_key(query);

        Ok(extracted)
    }

    /// Build the keyword extraction prompt.
    ///
    /// This prompt is designed to match LightRAG's extraction quality.
    pub fn build_prompt(&self, query: &str) -> String {
        format!(
            r#"Extract high-level and low-level keywords from the following query, and classify the query intent.

## Definitions

**High-level keywords**: Abstract concepts, themes, or topics that represent the broader context or domain of the query. These are used to find relevant relationships and global patterns in a knowledge graph.
Examples: "artificial intelligence", "climate change", "software architecture", "healthcare outcomes"

**Low-level keywords**: Specific entities, technical terms, proper nouns, or concrete concepts. These are used to find specific entities in a knowledge graph.
Examples: "GPT-4", "Sarah Chen", "PostgreSQL", "neural network", "Microsoft"

**Query Intent**:
- factual: Questions asking for facts about a specific thing ("What is X?", "Who is Y?")
- relational: Questions about connections between things ("How does X relate to Y?")
- exploratory: Broad questions seeking overview or understanding ("Tell me about X")
- comparative: Questions comparing multiple things ("Compare X and Y")
- procedural: Questions about processes or steps ("How to do X?")

## Query
"{query}"

## Output Format
Respond ONLY with valid JSON:
{{
  "high_level_keywords": ["concept1", "concept2", ...],
  "low_level_keywords": ["entity1", "term1", ...],
  "query_intent": "factual|relational|exploratory|comparative|procedural"
}}

## Examples

Query: "How does machine learning improve healthcare outcomes?"
{{
  "high_level_keywords": ["machine learning", "healthcare", "improvement", "outcomes"],
  "low_level_keywords": ["ML algorithms", "medical diagnosis", "patient data", "clinical trials"],
  "query_intent": "relational"
}}

Query: "What is the relationship between OpenAI and Microsoft?"
{{
  "high_level_keywords": ["business relationship", "partnership", "technology collaboration"],
  "low_level_keywords": ["OpenAI", "Microsoft", "GPT", "Azure", "investment"],
  "query_intent": "relational"
}}

Query: "Who is Sarah Chen and what is her role in the research project?"
{{
  "high_level_keywords": ["research", "team roles", "project leadership"],
  "low_level_keywords": ["Sarah Chen", "researcher", "project"],
  "query_intent": "factual"
}}

Query: "Compare Python and Rust for systems programming"
{{
  "high_level_keywords": ["programming languages", "systems programming", "language comparison"],
  "low_level_keywords": ["Python", "Rust", "performance", "memory safety", "type system"],
  "query_intent": "comparative"
}}

Now extract keywords from the query above. Respond with JSON only:"#
        )
    }

    /// Parse the LLM response into ExtractedKeywords.
    fn parse_response(&self, response: &str) -> Result<ExtractedKeywords> {
        // Try to find JSON in the response
        let json_str = self.extract_json(response)?;

        // Parse the JSON
        let parsed: LLMKeywordResponse = match serde_json::from_str(&json_str) {
            Ok(p) => p,
            Err(e) => {
                // Try json_repair-style fixes
                if let Ok(fixed) = self.try_fix_json(&json_str) {
                    if let Ok(parsed) = serde_json::from_str::<LLMKeywordResponse>(&fixed) {
                        parsed
                    } else {
                        return Err(QueryError::Internal(format!(
                            "Failed to parse keyword JSON after fix attempt: {}. Response: {}",
                            e, json_str
                        )));
                    }
                } else {
                    return Err(QueryError::Internal(format!(
                        "Failed to parse keyword JSON: {}. Response: {}",
                        e, json_str
                    )));
                }
            }
        };

        Ok(ExtractedKeywords::new(
            parsed.high_level_keywords,
            parsed.low_level_keywords,
            QueryIntent::from_str_loose(&parsed.query_intent),
        ))
    }

    /// Extract JSON from a potentially messy LLM response.
    fn extract_json(&self, response: &str) -> Result<String> {
        let trimmed = response.trim();

        // If it starts with {, assume it's pure JSON
        if trimmed.starts_with('{') {
            // Find the matching closing brace
            let mut depth = 0;
            let mut end_pos = 0;
            for (i, c) in trimmed.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end_pos = i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if end_pos > 0 {
                return Ok(trimmed[..end_pos].to_string());
            }
        }

        // Try to find JSON in markdown code block
        if let Some(start) = trimmed.find("```json") {
            if let Some(end) = trimmed[start..]
                .find("```\n")
                .or(trimmed[start..].rfind("```"))
            {
                let json_start = start + 7; // Skip ```json
                return Ok(trimmed[json_start..start + end].trim().to_string());
            }
        }

        // Try to find any { ... } block
        if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                return Ok(trimmed[start..=end].to_string());
            }
        }

        Err(QueryError::Internal(format!(
            "Could not find JSON in response: {}",
            trimmed
        )))
    }

    /// Try to fix common JSON issues.
    fn try_fix_json(&self, json: &str) -> Result<String> {
        // Replace single quotes with double quotes
        let fixed = json.replace('\'', "\"");

        // Remove trailing commas
        let fixed = regex::Regex::new(r",\s*\}")
            .map(|re| re.replace_all(&fixed, "}").to_string())
            .unwrap_or(fixed);

        let fixed = regex::Regex::new(r",\s*\]")
            .map(|re| re.replace_all(&fixed, "]").to_string())
            .unwrap_or(fixed);

        Ok(fixed)
    }
}

/// LLM response structure for keyword extraction.
#[derive(Debug, Deserialize, Serialize)]
struct LLMKeywordResponse {
    high_level_keywords: Vec<String>,
    low_level_keywords: Vec<String>,
    #[serde(default = "default_intent")]
    query_intent: String,
}

fn default_intent() -> String {
    "exploratory".to_string()
}

#[async_trait]
impl KeywordExtractor for LLMKeywordExtractor {
    async fn extract(&self, query: &str) -> Result<Keywords> {
        let prompt = self.build_prompt(query);

        let response = self
            .llm_provider
            .complete(&prompt)
            .await
            .map_err(QueryError::from)?;

        let extracted = self.parse_response(&response.content)?;

        Ok(extracted.to_simple())
    }

    async fn extract_extended(&self, query: &str) -> Result<ExtractedKeywords> {
        let prompt = self.build_prompt(query);

        let response = self
            .llm_provider
            .complete(&prompt)
            .await
            .map_err(QueryError::from)?;

        let mut extracted = self.parse_response(&response.content)?;
        extracted.cache_key = self.cache_key(query);

        Ok(extracted)
    }

    /// Override to use the provided LLM when available.
    ///
    /// WHY: When a user selects an LLM provider (e.g., OpenAI GPT-4), this ensures
    /// that keyword extraction uses the SAME provider as the rest of the query pipeline.
    /// Without this, keyword extraction would always use the server default LLM,
    /// creating inconsistent behavior and unexpected costs.
    ///
    /// CRITICAL: This method is the fix for the bug where user-selected LLM was
    /// ignored during keyword extraction. Always call this method when an LLM
    /// override is provided, not `extract_extended()`.
    async fn extract_with_llm_override(
        &self,
        query: &str,
        llm_override: Option<std::sync::Arc<dyn crate::LLMProvider>>,
    ) -> Result<ExtractedKeywords> {
        match llm_override {
            Some(llm) => {
                // Use the provided LLM (user's selection)
                tracing::debug!(
                    query = %query,
                    "Using LLM override for keyword extraction (user-selected provider)"
                );
                self.extract_with_provider(query, llm).await
            }
            None => {
                // Fall back to default LLM (server's default)
                tracing::debug!(
                    query = %query,
                    "Using default LLM for keyword extraction (no user selection)"
                );
                self.extract_extended(query).await
            }
        }
    }
}

/// Cached keyword extractor that wraps another extractor.
///
/// Provides multi-level caching with configurable TTL.
pub struct CachedKeywordExtractor {
    inner: Arc<dyn KeywordExtractor>,
    cache: Arc<dyn KeywordCache>,
    ttl: Duration,
}

impl CachedKeywordExtractor {
    /// Create a new cached extractor.
    pub fn new(
        inner: Arc<dyn KeywordExtractor>,
        cache: Arc<dyn KeywordCache>,
        ttl: Duration,
    ) -> Self {
        Self { inner, cache, ttl }
    }

    /// Create with default TTL (24 hours).
    pub fn with_default_ttl(
        inner: Arc<dyn KeywordExtractor>,
        cache: Arc<dyn KeywordCache>,
    ) -> Self {
        Self::new(inner, cache, Duration::from_secs(24 * 60 * 60))
    }
}

#[async_trait]
impl KeywordExtractor for CachedKeywordExtractor {
    async fn extract(&self, query: &str) -> Result<Keywords> {
        let extracted = self.extract_extended(query).await?;
        Ok(extracted.to_simple())
    }

    async fn extract_extended(&self, query: &str) -> Result<ExtractedKeywords> {
        let cache_key = self.cache_key(query);

        // Check cache first
        if let Ok(Some(cached)) = self.cache.get(&cache_key).await {
            tracing::debug!(query = %query, "Keyword cache hit");
            return Ok(cached);
        }

        tracing::debug!(query = %query, "Keyword cache miss, extracting...");

        // Extract keywords
        let mut extracted = self.inner.extract_extended(query).await?;
        extracted.cache_key = cache_key.clone();

        // Cache the result
        if let Err(e) = self.cache.set(&cache_key, &extracted, Some(self.ttl)).await {
            tracing::warn!(error = %e, "Failed to cache keywords");
        }

        Ok(extracted)
    }

    /// Delegate to inner extractor with LLM override.
    ///
    /// WHY: Caching layer is transparent - it should pass through LLM overrides
    /// to the underlying extractor (typically LLMKeywordExtractor) which will
    /// use the user-selected LLM provider for keyword extraction.
    ///
    /// NOTE: We don't cache based on LLM provider - cache key is query-only.
    /// This is intentional: keyword quality should be similar across providers,
    /// and per-provider caching would fragment the cache unnecessarily.
    async fn extract_with_llm_override(
        &self,
        query: &str,
        llm_override: Option<std::sync::Arc<dyn crate::LLMProvider>>,
    ) -> Result<ExtractedKeywords> {
        let cache_key = self.cache_key(query);

        // Check cache first (independent of LLM provider)
        if let Ok(Some(cached)) = self.cache.get(&cache_key).await {
            tracing::debug!(query = %query, "Keyword cache hit (with LLM override)");
            return Ok(cached);
        }

        tracing::debug!(query = %query, "Keyword cache miss (with LLM override), extracting...");

        // Extract keywords with LLM override
        let mut extracted = self
            .inner
            .extract_with_llm_override(query, llm_override)
            .await?;
        extracted.cache_key = cache_key.clone();

        // Cache the result
        if let Err(e) = self.cache.set(&cache_key, &extracted, Some(self.ttl)).await {
            tracing::warn!(error = %e, "Failed to cache keywords");
        }

        Ok(extracted)
    }

    fn cache_key(&self, query: &str) -> String {
        self.inner.cache_key(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_extractor_prompt_generation() {
        let llm = Arc::new(edgequake_llm::MockProvider::new());
        let extractor = LLMKeywordExtractor::new(llm);

        let prompt = extractor.build_prompt("What is AI?");
        assert!(prompt.contains("What is AI?"));
        assert!(prompt.contains("high_level_keywords"));
        assert!(prompt.contains("low_level_keywords"));
        assert!(prompt.contains("query_intent"));
    }

    #[test]
    fn test_json_extraction() {
        let llm = Arc::new(edgequake_llm::MockProvider::new());
        let extractor = LLMKeywordExtractor::new(llm);

        // Pure JSON
        let json = r#"{"high_level_keywords": ["ai"], "low_level_keywords": ["gpt"], "query_intent": "factual"}"#;
        let result = extractor.extract_json(json).unwrap();
        assert!(result.contains("high_level_keywords"));

        // JSON with surrounding text
        let messy = r#"Here is the JSON:
{"high_level_keywords": ["ai"], "low_level_keywords": ["gpt"], "query_intent": "factual"}
Done!"#;
        let result = extractor.extract_json(messy).unwrap();
        assert!(result.contains("high_level_keywords"));
    }

    #[test]
    fn test_parse_response() {
        let llm = Arc::new(edgequake_llm::MockProvider::new());
        let extractor = LLMKeywordExtractor::new(llm);

        let response = r#"{
            "high_level_keywords": ["machine learning", "healthcare"],
            "low_level_keywords": ["GPT-4", "diagnosis"],
            "query_intent": "relational"
        }"#;

        let extracted = extractor.parse_response(response).unwrap();
        assert_eq!(extracted.high_level.len(), 2);
        assert_eq!(extracted.low_level.len(), 2);
        assert_eq!(extracted.query_intent, QueryIntent::Relational);
    }
}
