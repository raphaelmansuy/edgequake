//! Description summarization.
//!
//! This module provides functionality to summarize long descriptions
//! using LLMs or rule-based approaches. Supports map-reduce style
//! summarization for merging multiple entity descriptions.
//!
//! # MapReduce Summarization
//!
//! For large description sets, we use a two-phase approach:
//! 1. **Map Phase**: Chunk descriptions into groups, summarize each
//! 2. **Reduce Phase**: Recursively combine summaries until one remains
//!
//! Based on LightRAG's summarization: `lightrag/operate.py:_handle_entity_relation_summary()`

use std::sync::Arc;

use crate::error::{PipelineError, Result};
use crate::prompts::SummarizationPrompts;

/// Configuration for the summarizer.
#[derive(Debug, Clone)]
/// @implements FEAT0010
pub struct SummarizerConfig {
    /// Maximum input length before summarization triggers.
    pub max_input_length: usize,

    /// Target output length.
    pub target_length: usize,

    /// Whether to preserve key entities in the summary.
    pub preserve_entities: bool,

    /// Maximum tokens per chunk for map-reduce.
    pub max_tokens_per_chunk: usize,

    /// Force LLM summary when this many descriptions are being merged.
    pub force_llm_summary_threshold: usize,
}

impl Default for SummarizerConfig {
    fn default() -> Self {
        Self {
            max_input_length: 2048,
            target_length: 512,
            preserve_entities: true,
            max_tokens_per_chunk: 4000,
            force_llm_summary_threshold: 4,
        }
    }
}

impl SummarizerConfig {
    /// Create a config with specific target length.
    pub fn with_target_length(mut self, length: usize) -> Self {
        self.target_length = length;
        self
    }

    /// Set the force LLM threshold.
    pub fn with_force_threshold(mut self, threshold: usize) -> Self {
        self.force_llm_summary_threshold = threshold;
        self
    }
}

/// Trait for description summarization.
#[async_trait::async_trait]
pub trait DescriptionSummarizer: Send + Sync {
    /// Summarize a description.
    async fn summarize(&self, description: &str) -> Result<String>;

    /// Summarize multiple descriptions, combining them.
    async fn summarize_combined(&self, descriptions: &[&str]) -> Result<String> {
        let combined = descriptions.join(" ");
        self.summarize(&combined).await
    }
}

/// Simple rule-based summarizer that truncates at sentence boundaries.
pub struct SimpleSummarizer {
    config: SummarizerConfig,
}

impl SimpleSummarizer {
    /// Create a new simple summarizer.
    pub fn new(config: SummarizerConfig) -> Self {
        Self { config }
    }
}

impl Default for SimpleSummarizer {
    fn default() -> Self {
        Self::new(SummarizerConfig::default())
    }
}

#[async_trait::async_trait]
impl DescriptionSummarizer for SimpleSummarizer {
    async fn summarize(&self, description: &str) -> Result<String> {
        if description.len() <= self.config.target_length {
            return Ok(description.to_string());
        }

        // Split into sentences
        let sentences: Vec<&str> = description
            .split(['.', '!', '?'])
            .filter(|s| !s.trim().is_empty())
            .collect();

        // Take sentences until we reach target length
        let mut result = String::new();
        for sentence in sentences {
            let sentence = sentence.trim();
            if result.len() + sentence.len() + 2 > self.config.target_length {
                break;
            }
            if !result.is_empty() {
                result.push_str(". ");
            }
            result.push_str(sentence);
        }

        if !result.is_empty() && !result.ends_with('.') {
            result.push('.');
        }

        Ok(result)
    }
}

/// LLM-based summarizer for high-quality summaries.
pub struct LLMSummarizer {
    llm_provider: Arc<dyn edgequake_llm::LLMProvider>,
    config: SummarizerConfig,
    prompts: SummarizationPrompts,
}

impl LLMSummarizer {
    /// Create a new LLM summarizer.
    pub fn new(
        llm_provider: Arc<dyn edgequake_llm::LLMProvider>,
        config: SummarizerConfig,
    ) -> Self {
        Self {
            llm_provider,
            config,
            prompts: SummarizationPrompts,
        }
    }

    /// Create with custom prompts.
    pub fn with_prompts(
        llm_provider: Arc<dyn edgequake_llm::LLMProvider>,
        config: SummarizerConfig,
        prompts: SummarizationPrompts,
    ) -> Self {
        Self {
            llm_provider,
            config,
            prompts,
        }
    }

    /// Build the summarization prompt.
    fn build_prompt(&self, description: &str) -> String {
        self.prompts.simple_summary_prompt(description)
    }

    /// Estimate token count from text (rough approximation).
    #[allow(dead_code)]
    fn estimate_tokens(&self, text: &str) -> usize {
        // Average ~4 chars per token for English
        text.len() / 4
    }

    /// Merge multiple entity descriptions into a coherent summary.
    ///
    /// Uses map-reduce approach for large description sets:
    /// 1. Chunk descriptions to fit within token limits
    /// 2. Summarize each chunk
    /// 3. Recursively reduce until a single summary remains
    pub async fn merge_entity_descriptions(
        &self,
        entity_name: &str,
        descriptions: &[String],
    ) -> Result<String> {
        if descriptions.is_empty() {
            return Ok(String::new());
        }

        if descriptions.len() == 1 {
            return Ok(descriptions[0].clone());
        }

        // Check if we need LLM summarization
        let total_length: usize = descriptions.iter().map(|d| d.len()).sum();
        let estimated_tokens = total_length / 4; // Rough estimate

        if descriptions.len() < self.config.force_llm_summary_threshold
            && estimated_tokens < self.config.max_tokens_per_chunk
        {
            // Simple concatenation with deduplication
            return Ok(self.simple_merge(descriptions));
        }

        // Apply map-reduce for large description sets
        self.map_reduce_summarize(entity_name, descriptions).await
    }

    /// Simple merge without LLM (for small description sets).
    fn simple_merge(&self, descriptions: &[String]) -> String {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        for desc in descriptions {
            let normalized = desc.trim().to_lowercase();
            if !seen.contains(&normalized) && !desc.trim().is_empty() {
                seen.insert(normalized);
                result.push(desc.trim());
            }
        }

        result.join(" ")
    }

    /// Map-reduce summarization for large description sets.
    async fn map_reduce_summarize(
        &self,
        entity_name: &str,
        descriptions: &[String],
    ) -> Result<String> {
        // Map phase: chunk descriptions into groups
        let chunks = self.chunk_descriptions(descriptions);

        let mut intermediate_summaries = Vec::new();

        for chunk in chunks {
            let summary = self.summarize_chunk(entity_name, &chunk).await?;
            intermediate_summaries.push(summary);
        }

        // Reduce phase: if we still have multiple summaries, reduce them
        while intermediate_summaries.len() > 1 {
            let new_chunks = self.chunk_descriptions(&intermediate_summaries);

            let mut new_summaries = Vec::new();
            for chunk in new_chunks {
                let summary = self.summarize_chunk(entity_name, &chunk).await?;
                new_summaries.push(summary);
            }
            intermediate_summaries = new_summaries;
        }

        Ok(intermediate_summaries
            .into_iter()
            .next()
            .unwrap_or_default())
    }

    /// Chunk descriptions to fit within token limit.
    fn chunk_descriptions(&self, descriptions: &[String]) -> Vec<Vec<String>> {
        let mut chunks = Vec::new();
        let mut current_chunk = Vec::new();
        let mut current_tokens = 0;

        for desc in descriptions {
            let desc_tokens = desc.len() / 4; // Rough estimate

            if current_tokens + desc_tokens > self.config.max_tokens_per_chunk
                && !current_chunk.is_empty()
            {
                chunks.push(std::mem::take(&mut current_chunk));
                current_tokens = 0;
            }

            current_chunk.push(desc.clone());
            current_tokens += desc_tokens;
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }

    /// Summarize a single chunk of descriptions.
    async fn summarize_chunk(&self, entity_name: &str, descriptions: &[String]) -> Result<String> {
        let descriptions_refs: Vec<&str> = descriptions.iter().map(|s| s.as_str()).collect();
        let prompt = self
            .prompts
            .entity_summary_prompt(entity_name, &descriptions_refs);

        let response = self
            .llm_provider
            .complete(&prompt)
            .await
            .map_err(|e| PipelineError::ExtractionError(format!("LLM error: {}", e)))?;

        Ok(response.content.trim().to_string())
    }

    /// Merge relationship descriptions.
    pub async fn merge_relationship_descriptions(
        &self,
        source: &str,
        target: &str,
        descriptions: &[String],
    ) -> Result<String> {
        if descriptions.is_empty() {
            return Ok(String::new());
        }

        if descriptions.len() == 1 {
            return Ok(descriptions[0].clone());
        }

        let descriptions_refs: Vec<&str> = descriptions.iter().map(|s| s.as_str()).collect();
        let prompt = self
            .prompts
            .relationship_summary_prompt(source, target, &descriptions_refs);

        let response = self
            .llm_provider
            .complete(&prompt)
            .await
            .map_err(|e| PipelineError::ExtractionError(format!("LLM error: {}", e)))?;

        Ok(response.content.trim().to_string())
    }

    /// Reduce multiple summaries into one (for MapReduce reduce phase).
    pub async fn reduce_summaries(&self, summaries: &[String]) -> Result<String> {
        if summaries.is_empty() {
            return Ok(String::new());
        }

        if summaries.len() == 1 {
            return Ok(summaries[0].clone());
        }

        let summaries_refs: Vec<&str> = summaries.iter().map(|s| s.as_str()).collect();
        let prompt = self.prompts.reduce_summary_prompt(&summaries_refs);

        let response = self
            .llm_provider
            .complete(&prompt)
            .await
            .map_err(|e| PipelineError::ExtractionError(format!("LLM error: {}", e)))?;

        Ok(response.content.trim().to_string())
    }
}

#[async_trait::async_trait]
impl DescriptionSummarizer for LLMSummarizer {
    async fn summarize(&self, description: &str) -> Result<String> {
        if description.len() <= self.config.target_length {
            return Ok(description.to_string());
        }

        let prompt = self.build_prompt(description);

        let response = self
            .llm_provider
            .complete(&prompt)
            .await
            .map_err(|e| PipelineError::ExtractionError(format!("LLM error: {}", e)))?;

        Ok(response.content.trim().to_string())
    }
}

/// Summarize entity descriptions by combining and condensing.
pub async fn summarize_entity_description<S: DescriptionSummarizer>(
    summarizer: &S,
    existing: &str,
    new: &str,
    max_length: usize,
) -> Result<String> {
    let combined_length = existing.len() + new.len();

    if combined_length <= max_length {
        // Just combine them
        if existing.is_empty() {
            return Ok(new.to_string());
        }
        if new.is_empty() || existing.contains(new) {
            return Ok(existing.to_string());
        }
        return Ok(format!("{} {}", existing, new));
    }

    // Need to summarize
    let combined = format!("{} {}", existing, new);
    summarizer.summarize(&combined).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_summarizer_short_text() {
        let summarizer = SimpleSummarizer::default();
        let short = "This is a short description.";

        let result = summarizer.summarize(short).await.unwrap();
        assert_eq!(result, short);
    }

    #[tokio::test]
    async fn test_simple_summarizer_long_text() {
        let config = SummarizerConfig {
            target_length: 50,
            ..Default::default()
        };
        let summarizer = SimpleSummarizer::new(config);

        let long = "First sentence here. Second sentence follows. Third sentence now. Fourth one too. Fifth is last.";

        let result = summarizer.summarize(long).await.unwrap();
        assert!(result.len() <= 60); // Some margin for sentence completion
        assert!(result.ends_with('.'));
    }

    #[tokio::test]
    async fn test_summarize_combined() {
        let summarizer = SimpleSummarizer::default();
        let descriptions = vec!["First part.", "Second part."];

        let result = summarizer.summarize_combined(&descriptions).await.unwrap();
        assert!(result.contains("First") || result.contains("Second"));
    }

    #[test]
    fn test_summarizer_config_default() {
        let config = SummarizerConfig::default();
        assert_eq!(config.max_input_length, 2048);
        assert_eq!(config.target_length, 512);
        assert!(config.preserve_entities);
    }

    #[test]
    fn test_summarizer_config_custom() {
        let config = SummarizerConfig {
            max_input_length: 1024,
            target_length: 256,
            preserve_entities: false,
            max_tokens_per_chunk: 600,
            force_llm_summary_threshold: 10,
        };
        assert_eq!(config.max_input_length, 1024);
        assert!(!config.preserve_entities);
    }

    #[test]
    fn test_summarizer_config_clone() {
        let config = SummarizerConfig::default();
        let cloned = config.clone();
        assert_eq!(config.target_length, cloned.target_length);
    }

    #[test]
    fn test_summarizer_config_debug() {
        let config = SummarizerConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("max_input_length"));
        assert!(debug.contains("2048"));
    }

    #[test]
    fn test_simple_summarizer_default() {
        let summarizer = SimpleSummarizer::default();
        assert_eq!(summarizer.config.target_length, 512);
    }

    #[tokio::test]
    async fn test_summarizer_exclamation_split() {
        let config = SummarizerConfig {
            target_length: 30,
            ..Default::default()
        };
        let summarizer = SimpleSummarizer::new(config);

        let text = "Wow! Amazing! Incredible! Fantastic!";
        let result = summarizer.summarize(text).await.unwrap();
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_summarizer_question_split() {
        let config = SummarizerConfig {
            target_length: 50,
            ..Default::default()
        };
        let summarizer = SimpleSummarizer::new(config);

        let text = "What is this? How does it work? Why does it matter?";
        let result = summarizer.summarize(text).await.unwrap();
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_summarizer_empty_text() {
        let summarizer = SimpleSummarizer::default();
        let result = summarizer.summarize("").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_summarize_entity_description_both_empty() {
        let summarizer = SimpleSummarizer::default();
        let result = summarize_entity_description(&summarizer, "", "", 1000)
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_summarize_entity_description_existing_empty() {
        let summarizer = SimpleSummarizer::default();
        let result = summarize_entity_description(&summarizer, "", "new content", 1000)
            .await
            .unwrap();
        assert_eq!(result, "new content");
    }

    #[tokio::test]
    async fn test_summarize_entity_description_new_empty() {
        let summarizer = SimpleSummarizer::default();
        let result = summarize_entity_description(&summarizer, "existing content", "", 1000)
            .await
            .unwrap();
        assert_eq!(result, "existing content");
    }

    #[tokio::test]
    async fn test_summarize_entity_description_duplicate() {
        let summarizer = SimpleSummarizer::default();
        let result =
            summarize_entity_description(&summarizer, "same content", "same content", 1000)
                .await
                .unwrap();
        assert_eq!(result, "same content");
    }

    #[tokio::test]
    async fn test_summarize_entity_description_contains() {
        let summarizer = SimpleSummarizer::default();
        let result =
            summarize_entity_description(&summarizer, "existing content here", "content", 1000)
                .await
                .unwrap();
        assert_eq!(result, "existing content here");
    }
}
