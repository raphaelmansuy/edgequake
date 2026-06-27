//! LLM-based entity extractor using structured JSON prompts.

use async_trait::async_trait;

use super::{
    assign_token_usage, extraction_completion_options, ConfigurableEntitySchema, EntityExtractor,
    ExtractionResult,
};
use crate::chunker::TextChunk;
use crate::error::{PipelineError, Result};
use crate::prompts::{JsonExtractionParser, JsonParseOptions};

/// LLM-based entity extractor using structured prompts.
///
/// # WHY: LLM Extraction Strategy
///
/// LLM extraction is the core of knowledge graph construction:
///
/// 1. **Structured Prompt** - Uses a carefully designed prompt that:
///    - Lists valid entity types to constrain LLM output
///    - Requests JSON format for reliable parsing
///    - Asks for descriptions to enrich entity/relationship context
///    - WHY JSON: Tuples are faster but JSON is more reliable for complex relationships
///
/// 2. **Entity Type Constraints** - Pre-defined types (PERSON, ORG, LOCATION, etc.)
///    - WHY: Constraining types improves extraction consistency
///    - WHY custom types: Domain-specific extraction (e.g., PROTEIN for biomedical)
///
/// 3. **Relationship Extraction** - Source → Relationship → Target triples
///    - WHY tuples: Graph databases need explicit source/target
///    - WHY descriptions: Context for semantic search
///
/// 4. **Error-Tolerant Parsing** - Handles malformed LLM output
///    - WHY: LLMs occasionally produce invalid JSON; we extract what we can
pub struct LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    llm_provider: std::sync::Arc<L>,
    entity_schema: crate::prompts::EntityExtractionSchema,
}

impl<L> LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    /// Create a new LLM extractor.
    pub fn new(llm_provider: std::sync::Arc<L>) -> Self {
        Self {
            llm_provider,
            entity_schema: crate::prompts::EntityExtractionSchema::server_default(),
        }
    }
}

impl<L> ConfigurableEntitySchema for LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    fn entity_schema_field(&mut self) -> &mut crate::prompts::EntityExtractionSchema {
        &mut self.entity_schema
    }
}

impl<L> LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    /// Create with custom entity types (strict enforcement, backward compatible).
    pub fn with_entity_types(self, types: Vec<String>) -> Self {
        ConfigurableEntitySchema::with_entity_types(self, types)
    }

    /// Create with full schema (types + strict/permissive mode).
    pub fn with_entity_schema(self, schema: crate::prompts::EntityExtractionSchema) -> Self {
        ConfigurableEntitySchema::with_entity_schema(self, schema)
    }
}

impl<L> LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + ?Sized,
{
    /// Build the extraction prompt.
    fn build_prompt(&self, chunk: &TextChunk) -> String {
        let text =
            crate::prompts::text_with_section_context(&chunk.content, chunk.section.as_ref());
        crate::prompts::json_extraction_prompt(&text, &self.entity_schema)
    }

    /// Parse the LLM response via shared [`JsonExtractionParser`] (normalization + recovery).
    fn parse_response(&self, response: &str, chunk_id: &str) -> Result<ExtractionResult> {
        JsonExtractionParser::new().parse_with_options(
            response,
            chunk_id,
            JsonParseOptions {
                entity_schema: Some(&self.entity_schema),
                recover_truncated: true,
                empty_on_missing_json: true,
            },
        )
    }
}

#[async_trait]
impl<L> EntityExtractor for LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync + ?Sized,
{
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        let prompt = self.build_prompt(chunk);

        // WHY reasoning_effort="none" + explicit max_tokens:
        // Reasoning models (gpt-5-nano, gpt-5-mini, o-series) exhaust all completion_tokens
        // on chain-of-thought when no limit is set (reasoning_tokens = completion_tokens → 0
        // net output tokens → empty JSON → parse error). Setting reasoning_effort="none"
        // disables CoT for extraction tasks where structured JSON output is required.
        // Non-reasoning models silently ignore this field.
        let options = extraction_completion_options(self.llm_provider.model(), 16384);

        let response = self
            .llm_provider
            .complete_with_options(&prompt, &options)
            .await
            .map_err(|e| PipelineError::ExtractionError(format!("LLM error: {}", e)))?;

        let mut result = self.parse_response(&response.content, &chunk.id)?;

        assign_token_usage(
            &mut result,
            response.prompt_tokens,
            response.completion_tokens,
        );

        Ok(result)
    }

    fn name(&self) -> &str {
        "llm"
    }

    fn model_name(&self) -> &str {
        self.llm_provider.model()
    }

    /// @implements SPEC-032/OODA-226: Provider tracking in ProcessingStats
    fn provider_name(&self) -> &str {
        self.llm_provider.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_parse_response_recovers_partial_json() {
        let provider = Arc::new(edgequake_llm::MockProvider::default());
        let extractor = LLMExtractor::new(provider);

        let truncated_response = r#"{"entities":[{"name":"ALICE","type":"PERSON","description":"A scientist"},{"name":"BOB","type":"PERSON","description":"A colleague"}],"relationships":["#;

        let result = extractor.parse_response(truncated_response, "chunk_001");
        assert!(
            result.is_ok(),
            "parse_response should recover, got: {:?}",
            result
        );
        let extraction = result.unwrap();
        assert_eq!(extraction.entities.len(), 2);
        assert_eq!(extraction.entities[0].name, "ALICE");
        assert_eq!(extraction.entities[1].name, "BOB");
    }

    #[test]
    fn test_parse_response_empty_on_non_json() {
        let provider = Arc::new(edgequake_llm::MockProvider::default());
        let extractor = LLMExtractor::new(provider);

        let result = extractor.parse_response("this is not json", "chunk_bad");
        assert!(result.is_ok());
        let extraction = result.unwrap();
        assert_eq!(extraction.entities.len(), 0);
    }

    #[test]
    fn test_parse_response_normalizes_entity_names() {
        let provider = Arc::new(edgequake_llm::MockProvider::default());
        let extractor = LLMExtractor::new(provider);
        let response = r#"{"entities":[{"name":"The Company","type":"ORG","description":"x"}],"relationships":[]}"#;
        let extraction = extractor.parse_response(response, "c1").unwrap();
        assert_eq!(extraction.entities[0].name, "COMPANY");
    }
}
