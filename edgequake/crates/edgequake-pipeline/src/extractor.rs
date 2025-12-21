//! Entity and relationship extraction.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::chunker::TextChunk;
use crate::error::{PipelineError, Result};

/// Result of entity and relationship extraction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// Extracted entities.
    pub entities: Vec<ExtractedEntity>,

    /// Extracted relationships.
    pub relationships: Vec<ExtractedRelationship>,

    /// Source chunk ID.
    pub source_chunk_id: String,

    /// Processing metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ExtractionResult {
    /// Create a new empty extraction result.
    pub fn new(source_chunk_id: impl Into<String>) -> Self {
        Self {
            entities: Vec::new(),
            relationships: Vec::new(),
            source_chunk_id: source_chunk_id.into(),
            metadata: HashMap::new(),
        }
    }

    /// Add an entity.
    pub fn add_entity(&mut self, entity: ExtractedEntity) {
        self.entities.push(entity);
    }

    /// Add a relationship.
    pub fn add_relationship(&mut self, rel: ExtractedRelationship) {
        self.relationships.push(rel);
    }
}

/// An extracted entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// Entity name (normalized).
    pub name: String,

    /// Entity type (e.g., "PERSON", "ORGANIZATION", "CONCEPT").
    pub entity_type: String,

    /// Description of the entity.
    pub description: String,

    /// Importance score (0.0 to 1.0).
    pub importance: f32,

    /// Source text spans.
    pub source_spans: Vec<String>,
}

impl ExtractedEntity {
    /// Create a new extracted entity.
    pub fn new(
        name: impl Into<String>,
        entity_type: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            entity_type: entity_type.into(),
            description: description.into(),
            importance: 0.5,
            source_spans: Vec::new(),
        }
    }

    /// Set the importance score.
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Add a source span.
    pub fn with_source_span(mut self, span: impl Into<String>) -> Self {
        self.source_spans.push(span.into());
        self
    }
}

/// An extracted relationship between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    /// Source entity name.
    pub source: String,

    /// Target entity name.
    pub target: String,

    /// Relationship type/description.
    pub relation_type: String,

    /// Relationship description.
    pub description: String,

    /// Weight/strength (0.0 to 1.0).
    pub weight: f32,

    /// Keywords associated with this relationship.
    pub keywords: Vec<String>,
}

impl ExtractedRelationship {
    /// Create a new extracted relationship.
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        relation_type: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            relation_type: relation_type.into(),
            description: String::new(),
            weight: 0.5,
            keywords: Vec::new(),
        }
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the weight.
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Add keywords.
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }
}

/// Trait for entity extraction implementations.
#[async_trait]
pub trait EntityExtractor: Send + Sync {
    /// Extract entities and relationships from a text chunk.
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult>;

    /// Extract from multiple chunks in batch.
    async fn extract_batch(&self, chunks: &[TextChunk]) -> Result<Vec<ExtractionResult>> {
        let mut results = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            results.push(self.extract(chunk).await?);
        }
        Ok(results)
    }

    /// Get the name of this extractor.
    fn name(&self) -> &str;
}

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
                let entity = ExtractedEntity::new(&name, entity_type, &name)
                    .with_source_span(cap.as_str());
                result.add_entity(entity);
            }
        }

        Ok(result)
    }

    fn name(&self) -> &str {
        "simple"
    }
}

/// LLM-based entity extractor using structured prompts.
pub struct LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider,
{
    llm_provider: std::sync::Arc<L>,
    entity_types: Vec<String>,
}

impl<L> LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider,
{
    /// Create a new LLM extractor.
    pub fn new(llm_provider: std::sync::Arc<L>) -> Self {
        Self {
            llm_provider,
            entity_types: vec![
                "PERSON".to_string(),
                "ORGANIZATION".to_string(),
                "LOCATION".to_string(),
                "EVENT".to_string(),
                "CONCEPT".to_string(),
                "TECHNOLOGY".to_string(),
                "PRODUCT".to_string(),
            ],
        }
    }

    /// Create with custom entity types.
    pub fn with_entity_types(mut self, types: Vec<String>) -> Self {
        self.entity_types = types;
        self
    }

    /// Build the extraction prompt.
    fn build_prompt(&self, text: &str) -> String {
        let entity_types_str = self.entity_types.join(", ");

        format!(
            r#"Extract entities and relationships from the following text.

## Entity Types
{entity_types_str}

## Output Format
Respond with valid JSON in this exact format:
{{
  "entities": [
    {{"name": "Entity Name", "type": "ENTITY_TYPE", "description": "Brief description"}}
  ],
  "relationships": [
    {{"source": "Source Entity", "target": "Target Entity", "type": "RELATIONSHIP_TYPE", "description": "Brief description"}}
  ]
}}

## Text to Analyze
{text}

## JSON Response"#
        )
    }

    /// Parse the LLM response into extraction result.
    fn parse_response(&self, response: &str, chunk_id: &str) -> Result<ExtractionResult> {
        let mut result = ExtractionResult::new(chunk_id);

        // Try to extract JSON from the response
        let json_str = extract_json_from_response(response);

        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PipelineError::ExtractionError(format!("Invalid JSON: {}", e)))?;

        // Extract entities
        if let Some(entities) = parsed.get("entities").and_then(|v| v.as_array()) {
            for entity_val in entities {
                if let (Some(name), Some(entity_type), Some(description)) = (
                    entity_val.get("name").and_then(|v| v.as_str()),
                    entity_val.get("type").and_then(|v| v.as_str()),
                    entity_val.get("description").and_then(|v| v.as_str()),
                ) {
                    result.add_entity(ExtractedEntity::new(name, entity_type, description));
                }
            }
        }

        // Extract relationships
        if let Some(relationships) = parsed.get("relationships").and_then(|v| v.as_array()) {
            for rel_val in relationships {
                if let (Some(source), Some(target), Some(rel_type)) = (
                    rel_val.get("source").and_then(|v| v.as_str()),
                    rel_val.get("target").and_then(|v| v.as_str()),
                    rel_val.get("type").and_then(|v| v.as_str()),
                ) {
                    let description = rel_val
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    result.add_relationship(
                        ExtractedRelationship::new(source, target, rel_type)
                            .with_description(description),
                    );
                }
            }
        }

        Ok(result)
    }
}

#[async_trait]
impl<L> EntityExtractor for LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync,
{
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        let prompt = self.build_prompt(&chunk.content);

        let response = self
            .llm_provider
            .complete(&prompt)
            .await
            .map_err(|e| PipelineError::ExtractionError(format!("LLM error: {}", e)))?;

        self.parse_response(&response.content, &chunk.id)
    }

    fn name(&self) -> &str {
        "llm"
    }
}

/// Extract JSON from a potentially wrapped LLM response.
fn extract_json_from_response(response: &str) -> String {
    let response = response.trim();

    // Try to find JSON block markers
    if let Some(start) = response.find("```json") {
        if let Some(end) = response[start + 7..].find("```") {
            return response[start + 7..start + 7 + end].trim().to_string();
        }
    }

    // Try to find JSON starting with {
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            if end > start {
                return response[start..=end].to_string();
            }
        }
    }

    response.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extracted_entity_builder() {
        let entity = ExtractedEntity::new("John Doe", "PERSON", "A person named John")
            .with_importance(0.8)
            .with_source_span("John Doe is a developer");

        assert_eq!(entity.name, "John Doe");
        assert_eq!(entity.entity_type, "PERSON");
        assert_eq!(entity.importance, 0.8);
        assert_eq!(entity.source_spans.len(), 1);
    }

    #[test]
    fn test_extracted_relationship_builder() {
        let rel = ExtractedRelationship::new("Alice", "Bob", "KNOWS")
            .with_description("Alice knows Bob from work")
            .with_weight(0.9)
            .with_keywords(vec!["colleague".to_string(), "friend".to_string()]);

        assert_eq!(rel.source, "Alice");
        assert_eq!(rel.target, "Bob");
        assert_eq!(rel.weight, 0.9);
        assert_eq!(rel.keywords.len(), 2);
    }

    #[tokio::test]
    async fn test_simple_extractor() {
        let extractor = SimpleExtractor::new().unwrap();
        let chunk = TextChunk::new("chunk-1", "John Doe works at Acme Corp.", 0, 0, 30);

        let result = extractor.extract(&chunk).await.unwrap();

        // Should find "John Doe" as a person
        assert!(result.entities.iter().any(|e| e.name == "John Doe"));
    }

    #[test]
    fn test_extraction_result() {
        let mut result = ExtractionResult::new("chunk-1");

        result.add_entity(ExtractedEntity::new("Test", "CONCEPT", "A test"));
        result.add_relationship(ExtractedRelationship::new("A", "B", "RELATED_TO"));

        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.relationships.len(), 1);
    }
}
