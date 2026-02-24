//! Gleaning (re-extraction) extractor for finding missed entities.
//!
//! @implements FEAT0305

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{
    extract_json_from_response, EntityExtractor, ExtractedEntity, ExtractedRelationship,
    ExtractionResult,
};
use crate::chunker::TextChunk;
use crate::error::{PipelineError, Result};

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
/// # WHY: Multi-Pass Extraction (Gleaning)
///
/// LLMs often miss entities in a single pass due to:
/// - Attention limits on long texts
/// - Implicit entities (e.g., "the company" referring to earlier-mentioned "Apple")
/// - Context overload when many entities are present
///
/// **Gleaning Strategy:**
/// 1. First pass: Normal extraction with base extractor
/// 2. Subsequent passes: Re-prompt LLM with "What did you miss?"
///    - Include previously-found entities to avoid duplicates
///    - Focus on implicit/indirect entity mentions
///
/// **LightRAG Research Finding:**
/// - 1-2 gleaning iterations improve recall by 15-25%
/// - Diminishing returns after 2 iterations
/// - Cost: Each iteration = 1 additional LLM call
///
/// This implements GAP-018: Max Gleaning from LightRAG.
pub struct GleaningExtractor {
    /// The underlying LLM provider.
    llm_provider: std::sync::Arc<dyn edgequake_llm::LLMProvider>,
    /// The base extractor to use.
    base_extractor: std::sync::Arc<dyn EntityExtractor>,
    /// Gleaning configuration.
    config: GleaningConfig,
}

impl GleaningExtractor {
    /// Create a new gleaning extractor.
    pub fn new(
        llm_provider: std::sync::Arc<dyn edgequake_llm::LLMProvider>,
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
impl EntityExtractor for GleaningExtractor {
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

            // Accumulate token usage from gleaning iterations
            result.input_tokens += response.prompt_tokens;
            result.output_tokens += response.completion_tokens;

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
