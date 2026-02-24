//! Simple regex-based entity extractor for testing.

use async_trait::async_trait;
use std::collections::HashMap;

use super::{EntityExtractor, ExtractedEntity, ExtractionResult};
use crate::chunker::TextChunk;
use crate::error::{PipelineError, Result};

/// Simple regex-based entity extractor for testing.
pub struct SimpleExtractor {
    /// Entity type patterns.
    patterns: HashMap<String, regex::Regex>,
}

impl SimpleExtractor {
    /// Create a new simple extractor with default patterns.
    pub fn new() -> Result<Self> {
        let mut patterns = HashMap::new();

        // Simple patterns for common entity types
        patterns.insert(
            "PERSON".to_string(),
            regex::Regex::new(r"\b[A-Z][a-z]+ [A-Z][a-z]+\b")
                .map_err(|e| PipelineError::ConfigError(e.to_string()))?,
        );

        patterns.insert(
            "ORGANIZATION".to_string(),
            regex::Regex::new(r"\b[A-Z][A-Za-z]+ (?:Inc|Corp|LLC|Ltd|Company)\b")
                .map_err(|e| PipelineError::ConfigError(e.to_string()))?,
        );

        Ok(Self { patterns })
    }
}

impl Default for SimpleExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create SimpleExtractor")
    }
}

#[async_trait]
impl EntityExtractor for SimpleExtractor {
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        let mut result = ExtractionResult::new(&chunk.id);

        for (entity_type, pattern) in &self.patterns {
            for cap in pattern.find_iter(&chunk.content) {
                let name = cap.as_str().to_string();
                let entity =
                    ExtractedEntity::new(&name, entity_type, &name).with_source_span(cap.as_str());
                result.add_entity(entity);
            }
        }

        Ok(result)
    }

    fn name(&self) -> &str {
        "simple"
    }
}
