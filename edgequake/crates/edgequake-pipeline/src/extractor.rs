//! Entity and relationship extraction.

use async_trait::async_trait;
use edgequake_llm::traits::ChatMessage;
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

    /// Input tokens used for this extraction.
    pub input_tokens: usize,

    /// Output tokens generated for this extraction.
    pub output_tokens: usize,

    /// Extraction time in milliseconds.
    pub extraction_time_ms: u64,
}

impl ExtractionResult {
    /// Create a new empty extraction result.
    pub fn new(source_chunk_id: impl Into<String>) -> Self {
        Self {
            entities: Vec::new(),
            relationships: Vec::new(),
            source_chunk_id: source_chunk_id.into(),
            metadata: HashMap::new(),
            input_tokens: 0,
            output_tokens: 0,
            extraction_time_ms: 0,
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

    /// Set token usage information.
    pub fn with_token_usage(mut self, input_tokens: usize, output_tokens: usize) -> Self {
        self.input_tokens = input_tokens;
        self.output_tokens = output_tokens;
        self
    }

    /// Set extraction timing.
    pub fn with_timing(mut self, extraction_time_ms: u64) -> Self {
        self.extraction_time_ms = extraction_time_ms;
        self
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

    /// Entity embedding.
    pub embedding: Option<Vec<f32>>,
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
            embedding: None,
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

    /// Relationship embedding (for similarity search).
    /// Computed from: keywords + source + target + description
    pub embedding: Option<Vec<f32>>,
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
            embedding: None,
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
    
    /// Get the model name used by this extractor (if applicable).
    fn model_name(&self) -> &str {
        "unknown"
    }
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

/// LLM-based entity extractor using structured prompts.
pub struct LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    llm_provider: std::sync::Arc<L>,
    entity_types: Vec<String>,
}

impl<L> LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
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
    L: edgequake_llm::LLMProvider + Send + Sync + ?Sized,
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
    
    fn model_name(&self) -> &str {
        self.llm_provider.model()
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

/// SOTA LLM-based entity extractor using tuple-format prompts.
///
/// This extractor uses the SOTA prompt system ported from LightRAG,
/// featuring tuple-based output format for more robust parsing.
pub struct SOTAExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    llm_provider: std::sync::Arc<L>,
    entity_types: Vec<String>,
    prompts: crate::prompts::EntityExtractionPrompts,
    parser: crate::prompts::HybridExtractionParser,
    language: String,
}

impl<L> SOTAExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    /// Create a new SOTA extractor with default settings.
    pub fn new(llm_provider: std::sync::Arc<L>) -> Self {
        Self {
            llm_provider,
            entity_types: crate::prompts::default_entity_types(),
            prompts: crate::prompts::EntityExtractionPrompts::default(),
            parser: crate::prompts::HybridExtractionParser::new(true),
            language: "English".to_string(),
        }
    }

    /// Set custom entity types.
    pub fn with_entity_types(mut self, types: Vec<String>) -> Self {
        self.entity_types = types;
        self
    }

    /// Set output language.
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    /// Set custom prompts.
    pub fn with_prompts(mut self, prompts: crate::prompts::EntityExtractionPrompts) -> Self {
        self.prompts = prompts;
        self
    }
}

#[async_trait]
impl<L> EntityExtractor for SOTAExtractor<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync + ?Sized,
{
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        let start = std::time::Instant::now();

        // Build system and user prompts
        let system_prompt = self.prompts.system_prompt(&self.entity_types, &self.language);
        let user_prompt = self.prompts.user_prompt(&chunk.content, &self.entity_types, &self.language);

        // Create chat messages for system + user prompt
        let messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ];

        // Make LLM call using chat interface
        let response = self
            .llm_provider
            .chat(&messages, None)
            .await
            .map_err(|e| PipelineError::ExtractionError(format!("LLM error: {}", e)))?;

        // Parse response using hybrid parser
        let mut result = self.parser.parse(&response.content, &chunk.id)?;

        // Add token usage from response
        result.input_tokens = response.prompt_tokens;
        result.output_tokens = response.completion_tokens;
        result.extraction_time_ms = start.elapsed().as_millis() as u64;

        // Add source chunk line info to metadata
        result.metadata.insert(
            "extractor".to_string(),
            serde_json::json!("sota"),
        );
        result.metadata.insert(
            "language".to_string(),
            serde_json::json!(self.language),
        );
        result.metadata.insert(
            "model".to_string(),
            serde_json::json!(response.model),
        );

        Ok(result)
    }

    fn name(&self) -> &str {
        "sota"
    }

    fn model_name(&self) -> &str {
        self.llm_provider.model()
    }
}

/// Configuration for gleaning (re-extraction).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GleaningConfig {
    /// Maximum number of gleaning iterations.
    pub max_gleaning: usize,
    /// Whether to continue extraction even if first pass finds entities.
    pub always_glean: bool,
}

impl Default for GleaningConfig {
    fn default() -> Self {
        Self {
            max_gleaning: 1, // LightRAG default
            always_glean: false,
        }
    }
}

/// A wrapper extractor that performs gleaning (re-extraction) to find missed entities.
///
/// This implements GAP-018: Max Gleaning from LightRAG.
/// Gleaning is the process of asking the LLM to look again at the text
/// for any entities or relationships that might have been missed in the first pass.
pub struct GleaningExtractor<L: edgequake_llm::LLMProvider + Send + Sync + 'static> {
    /// The underlying LLM provider.
    llm_provider: std::sync::Arc<L>,
    /// The base extractor to use.
    base_extractor: std::sync::Arc<dyn EntityExtractor>,
    /// Gleaning configuration.
    config: GleaningConfig,
}

impl<L> GleaningExtractor<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync + 'static,
{
    /// Create a new gleaning extractor.
    pub fn new(
        llm_provider: std::sync::Arc<L>,
        base_extractor: std::sync::Arc<dyn EntityExtractor>,
    ) -> Self {
        Self {
            llm_provider,
            base_extractor,
            config: GleaningConfig::default(),
        }
    }

    /// Set the gleaning configuration.
    pub fn with_config(mut self, config: GleaningConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the maximum gleaning iterations.
    pub fn with_max_gleaning(mut self, max: usize) -> Self {
        self.config.max_gleaning = max;
        self
    }

    /// Build the gleaning prompt.
    fn build_gleaning_prompt(&self, text: &str, previous_entities: &[String]) -> String {
        let prev_entities_str = previous_entities.join(", ");

        format!(
            r#"MANY entities and relationships were missed in the last extraction. 
Please identify any ADDITIONAL entities and relationships that were not already captured.

## Already Identified Entities
{prev_entities_str}

## Instructions
Look for entities and relationships that were missed in the previous extraction.
Focus on:
- Implicit entities (mentioned indirectly)
- Additional relationships between known entities
- Contextual entities (dates, locations, concepts)

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

## Text to Re-Analyze
{text}

## JSON Response"#
        )
    }

    /// Parse gleaning response.
    fn parse_gleaning_response(
        &self,
        response: &str,
    ) -> Result<(Vec<ExtractedEntity>, Vec<ExtractedRelationship>)> {
        let json_str = extract_json_from_response(response);

        let parsed: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
            PipelineError::ExtractionError(format!("Invalid JSON in gleaning: {}", e))
        })?;

        let mut entities = Vec::new();
        let mut relationships = Vec::new();

        // Extract entities
        if let Some(entity_arr) = parsed.get("entities").and_then(|v| v.as_array()) {
            for entity_val in entity_arr {
                if let (Some(name), Some(entity_type), Some(description)) = (
                    entity_val.get("name").and_then(|v| v.as_str()),
                    entity_val.get("type").and_then(|v| v.as_str()),
                    entity_val.get("description").and_then(|v| v.as_str()),
                ) {
                    entities.push(ExtractedEntity::new(name, entity_type, description));
                }
            }
        }

        // Extract relationships
        if let Some(rel_arr) = parsed.get("relationships").and_then(|v| v.as_array()) {
            for rel_val in rel_arr {
                if let (Some(source), Some(target), Some(rel_type)) = (
                    rel_val.get("source").and_then(|v| v.as_str()),
                    rel_val.get("target").and_then(|v| v.as_str()),
                    rel_val.get("type").and_then(|v| v.as_str()),
                ) {
                    let description = rel_val
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    relationships.push(
                        ExtractedRelationship::new(source, target, rel_type)
                            .with_description(description),
                    );
                }
            }
        }

        Ok((entities, relationships))
    }

    /// Merge gleaning results with original results.
    fn merge_results(
        &self,
        original: &mut ExtractionResult,
        glean_entities: Vec<ExtractedEntity>,
        glean_relationships: Vec<ExtractedRelationship>,
    ) {
        // For entities: compare descriptions and keep the better (longer) one
        for glean_entity in glean_entities {
            let existing = original
                .entities
                .iter_mut()
                .find(|e| e.name.to_uppercase() == glean_entity.name.to_uppercase());

            if let Some(existing) = existing {
                // Keep the entity with the longer description
                if glean_entity.description.len() > existing.description.len() {
                    existing.description = glean_entity.description;
                    existing.entity_type = glean_entity.entity_type;
                }
            } else {
                // New entity from gleaning
                original.entities.push(glean_entity);
            }
        }

        // For relationships: compare and keep better descriptions
        for glean_rel in glean_relationships {
            let existing = original.relationships.iter_mut().find(|r| {
                r.source.to_uppercase() == glean_rel.source.to_uppercase()
                    && r.target.to_uppercase() == glean_rel.target.to_uppercase()
            });

            if let Some(existing) = existing {
                if glean_rel.description.len() > existing.description.len() {
                    existing.description = glean_rel.description;
                    existing.relation_type = glean_rel.relation_type;
                }
            } else {
                original.relationships.push(glean_rel);
            }
        }
    }
}

#[async_trait]
impl<L> EntityExtractor for GleaningExtractor<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync + 'static,
{
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        // First pass: use base extractor
        let mut result = self.base_extractor.extract(chunk).await?;

        // Skip gleaning if disabled
        if self.config.max_gleaning == 0 {
            return Ok(result);
        }

        // Perform gleaning iterations
        for iteration in 0..self.config.max_gleaning {
            tracing::debug!(
                chunk_id = %chunk.id,
                iteration = iteration,
                "Performing gleaning iteration"
            );

            // Collect entity names for the prompt
            let entity_names: Vec<String> =
                result.entities.iter().map(|e| e.name.clone()).collect();

            // Build and execute gleaning prompt
            let gleaning_prompt = self.build_gleaning_prompt(&chunk.content, &entity_names);

            let response = self
                .llm_provider
                .complete(&gleaning_prompt)
                .await
                .map_err(|e| {
                    PipelineError::ExtractionError(format!("Gleaning LLM error: {}", e))
                })?;

            // Parse gleaning results
            match self.parse_gleaning_response(&response.content) {
                Ok((glean_entities, glean_relationships)) => {
                    let new_entities = glean_entities.len();
                    let new_relationships = glean_relationships.len();

                    self.merge_results(&mut result, glean_entities, glean_relationships);

                    tracing::debug!(
                        chunk_id = %chunk.id,
                        iteration = iteration,
                        new_entities = new_entities,
                        new_relationships = new_relationships,
                        "Gleaning completed"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        chunk_id = %chunk.id,
                        iteration = iteration,
                        error = %e,
                        "Gleaning parse error, continuing"
                    );
                }
            }
        }

        // Record gleaning metadata
        result.metadata.insert(
            "gleaning_iterations".to_string(),
            serde_json::json!(self.config.max_gleaning),
        );

        Ok(result)
    }

    fn name(&self) -> &str {
        "gleaning"
    }
    
    fn model_name(&self) -> &str {
        self.llm_provider.model()
    }
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
