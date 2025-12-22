//! Description summarization.
//!
//! This module provides functionality to summarize long descriptions
//! using LLMs or rule-based approaches.

use std::sync::Arc;

use crate::error::{PipelineError, Result};

/// Configuration for the summarizer.
#[derive(Debug, Clone)]
pub struct SummarizerConfig {
    /// Maximum input length before summarization triggers.
    pub max_input_length: usize,

    /// Target output length.
    pub target_length: usize,

    /// Whether to preserve key entities in the summary.
    pub preserve_entities: bool,
}

impl Default for SummarizerConfig {
    fn default() -> Self {
        Self {
            max_input_length: 2048,
            target_length: 512,
            preserve_entities: true,
        }
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
            .split(|c| c == '.' || c == '!' || c == '?')
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
pub struct LLMSummarizer<L>
where
    L: edgequake_llm::LLMProvider,
{
    llm_provider: Arc<L>,
    config: SummarizerConfig,
}

impl<L> LLMSummarizer<L>
where
    L: edgequake_llm::LLMProvider,
{
    /// Create a new LLM summarizer.
    pub fn new(llm_provider: Arc<L>, config: SummarizerConfig) -> Self {
        Self {
            llm_provider,
            config,
        }
    }

    /// Build the summarization prompt.
    fn build_prompt(&self, description: &str) -> String {
        let target_words = self.config.target_length / 6; // Rough word estimate

        format!(
            r#"Summarize the following text in approximately {target_words} words or fewer.
Keep the most important facts and relationships.
Maintain a factual, descriptive tone.

Text:
{description}

Summary:"#
        )
    }
}

#[async_trait::async_trait]
impl<L> DescriptionSummarizer for LLMSummarizer<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync,
{
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
        let result = summarize_entity_description(&summarizer, "", "", 1000).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_summarize_entity_description_existing_empty() {
        let summarizer = SimpleSummarizer::default();
        let result = summarize_entity_description(&summarizer, "", "new content", 1000).await.unwrap();
        assert_eq!(result, "new content");
    }

    #[tokio::test]
    async fn test_summarize_entity_description_new_empty() {
        let summarizer = SimpleSummarizer::default();
        let result = summarize_entity_description(&summarizer, "existing content", "", 1000).await.unwrap();
        assert_eq!(result, "existing content");
    }

    #[tokio::test]
    async fn test_summarize_entity_description_duplicate() {
        let summarizer = SimpleSummarizer::default();
        let result = summarize_entity_description(&summarizer, "same content", "same content", 1000).await.unwrap();
        assert_eq!(result, "same content");
    }

    #[tokio::test]
    async fn test_summarize_entity_description_contains() {
        let summarizer = SimpleSummarizer::default();
        let result = summarize_entity_description(&summarizer, "existing content here", "content", 1000).await.unwrap();
        assert_eq!(result, "existing content here");
    }
}
