//! Gleaning (re-extraction) extractor for finding missed entities.
//!
//! @implements FEAT0305

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{EntityExtractor, ExtractedEntity, ExtractedRelationship, ExtractionResult};
use crate::chunker::TextChunk;
use crate::error::{PipelineError, Result};
use crate::prompts::{
    enforce_entity_type, EntityExtractionSchema, JsonExtractionParser, JsonParseOptions,
};

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
    /// Workspace entity type allow-list (must match base extractor).
    entity_schema: EntityExtractionSchema,
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
            entity_schema: EntityExtractionSchema::server_default(),
            config: GleaningConfig::default(),
        }
    }

    /// Set the entity type schema (strict/permissive allow-list).
    pub fn with_entity_schema(mut self, schema: EntityExtractionSchema) -> Self {
        self.entity_schema = schema;
        self
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
    fn build_gleaning_prompt(&self, chunk: &TextChunk, previous_entities: &[String]) -> String {
        let text =
            crate::prompts::text_with_section_context(&chunk.content, chunk.section.as_ref());
        crate::prompts::json_gleaning_prompt(&text, previous_entities, &self.entity_schema)
    }

    /// Parse gleaning response via shared JSON parser (normalization + BR0006 filters).
    fn parse_gleaning_response(
        &self,
        response: &str,
        chunk_id: &str,
    ) -> Result<(Vec<ExtractedEntity>, Vec<ExtractedRelationship>)> {
        let parsed = JsonExtractionParser::new().parse_with_options(
            response,
            chunk_id,
            JsonParseOptions {
                entity_schema: Some(&self.entity_schema),
                ..Default::default()
            },
        )?;
        Ok((parsed.entities, parsed.relationships))
    }

    fn apply_entity_schema_to_entities(&self, entities: &mut [ExtractedEntity]) {
        for entity in entities.iter_mut() {
            let (enforced, _) = enforce_entity_type(&entity.entity_type, &self.entity_schema);
            entity.entity_type = enforced;
        }
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
            let glean_key = crate::prompts::normalize_entity_name(&glean_entity.name);
            let existing = original
                .entities
                .iter_mut()
                .find(|e| crate::prompts::normalize_entity_name(&e.name) == glean_key);

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
            let source_key = crate::prompts::normalize_entity_name(&glean_rel.source);
            let target_key = crate::prompts::normalize_entity_name(&glean_rel.target);
            let existing = original.relationships.iter_mut().find(|r| {
                crate::prompts::normalize_entity_name(&r.source) == source_key
                    && crate::prompts::normalize_entity_name(&r.target) == target_key
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
            let gleaning_prompt = self.build_gleaning_prompt(chunk, &entity_names);

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
            match self.parse_gleaning_response(&response.content, &chunk.id) {
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

        // Defense in depth: re-apply schema after merge (base pass + gleaning).
        self.apply_entity_schema_to_entities(&mut result.entities);

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

    fn api_strict_schema() -> EntityExtractionSchema {
        EntityExtractionSchema {
            types: vec![
                "API_OR_INTERFACE".into(),
                "CODE_ELEMENT".into(),
                "SOFTWARE_COMPONENT".into(),
                "OTHER".into(),
            ],
            strict: true,
        }
    }

    #[test]
    fn gleaning_parse_enforces_strict_types() {
        let extractor = GleaningExtractor::new(
            std::sync::Arc::new(edgequake_llm::MockProvider::new()),
            std::sync::Arc::new(crate::extractor::LLMExtractor::new(std::sync::Arc::new(
                edgequake_llm::MockProvider::new(),
            ))),
        )
        .with_entity_schema(api_strict_schema());

        let response = r#"{
            "entities": [
                {"name": "Redis Client", "type": "Library", "description": "x"},
                {"name": "Vector DB", "type": "Concept", "description": "y"},
                {"name": "REST API", "type": "API_OR_INTERFACE", "description": "z"}
            ],
            "relationships": []
        }"#;

        let (entities, _) = extractor
            .parse_gleaning_response(response, "chunk-1")
            .expect("parse");

        assert_eq!(entities[0].entity_type, "OTHER");
        assert_eq!(entities[1].entity_type, "OTHER");
        assert_eq!(entities[2].entity_type, "API_OR_INTERFACE");
    }

    #[test]
    fn gleaning_post_merge_reapplies_schema() {
        let extractor = GleaningExtractor::new(
            std::sync::Arc::new(edgequake_llm::MockProvider::new()),
            std::sync::Arc::new(crate::extractor::LLMExtractor::new(std::sync::Arc::new(
                edgequake_llm::MockProvider::new(),
            ))),
        )
        .with_entity_schema(api_strict_schema());

        let mut entities = vec![ExtractedEntity::new("FOO", "Unknown", "d")];

        extractor.apply_entity_schema_to_entities(&mut entities);
        assert_eq!(entities[0].entity_type, "OTHER");
    }
}
