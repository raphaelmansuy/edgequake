//! LLM-based entity extractor using structured JSON prompts.

use async_trait::async_trait;

use super::{
    effective_temperature_for_model, extract_json_from_response, EntityExtractor, ExtractedEntity,
    ExtractedRelationship, ExtractionResult,
};
use crate::chunker::TextChunk;
use crate::error::{PipelineError, Result};

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
            // SPEC-085: Use shared default to align with SOTAExtractor (9 types).
            // WHY: Previously this used 7 hardcoded types (missing DATE, DOCUMENT).
            entity_types: crate::prompts::default_entity_types(),
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
    ///
    /// WHY PARTIAL RECOVERY: When the safety limit clamps `max_tokens` (e.g. to
    /// 16 384) and a very large entity list still exceeds that budget, the LLM
    /// output is truncated mid-JSON. Rather than discarding all work for this chunk
    /// we attempt to close the open JSON structure and salvage all complete
    /// entity/relationship objects that were emitted before truncation.
    fn parse_response(&self, response: &str, chunk_id: &str) -> Result<ExtractionResult> {
        let mut result = ExtractionResult::new(chunk_id);

        // Try to extract JSON from the response
        let json_str = extract_json_from_response(response);

        // WHY: LLMs sometimes emit control characters (\u0000-\u001F) in JSON strings.
        // Strip them to prevent serde_json parse failures.
        let sanitized: String = json_str
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
            .collect();

        let parsed: serde_json::Value = match serde_json::from_str(&sanitized) {
            Ok(v) => v,
            Err(primary_err) => {
                // Attempt partial JSON recovery: the output may be truncated mid-array
                // (e.g. "EOF while parsing a list"). Try closing every plausible open
                // structure and parse again. Use the first successful parse.
                tracing::warn!(
                    chunk_id = %chunk_id,
                    error = %primary_err,
                    "JSON parse failed — attempting partial recovery of truncated output"
                );
                let recovered = Self::try_recover_truncated_json(&sanitized);
                match recovered {
                    Some(v) => {
                        tracing::info!(
                            chunk_id = %chunk_id,
                            "Partial JSON recovery succeeded; some trailing entities may be missing"
                        );
                        v
                    }
                    None => {
                        return Err(PipelineError::ExtractionError(format!(
                            "Invalid JSON: {}",
                            primary_err
                        )));
                    }
                }
            }
        };

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

    /// Attempt to recover a valid JSON value from a truncated string.
    ///
    /// When the LLM output is cut off mid-array (e.g. after the last complete
    /// entity object), the raw string looks like:
    ///
    /// ```json
    /// {"entities":[{"name":"A","type":"B","description":"C"},{"name":"D",
    /// ```
    ///
    /// We try a series of closing suffixes in order of most-to-least aggressive.
    /// The first suffix that produces a valid `serde_json::Value` wins.  This
    /// salvages all complete entity/relationship objects that the model managed to
    /// emit before the token budget was exhausted.
    fn try_recover_truncated_json(s: &str) -> Option<serde_json::Value> {
        // Suffixes ordered from least invasive (just close the outer object)
        // to more invasive (close open inner object + arrays + outer object).
        // Each attempt closes a different level of nesting that the LLM might
        // have been cut off inside.
        let suffixes: &[&str] = &[
            "",        // already complete — fastest path, no allocation
            "}",       // already a complete object?
            "]}",      // truncated inside relationships array
            "}]}",     // truncated inside a relationship object
            "]}]}",    // truncated inside entities array, after a complete object
            "}]}]}",   // truncated inside an entity object
            "\"}]}]}", // truncated inside a string field of an entity
            "\"}]}",   // truncated inside a string in the relationships array
        ];
        for &suffix in suffixes {
            let candidate = format!("{}{}", s, suffix);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&candidate) {
                return Some(v);
            }
        }
        None
    }
}

#[async_trait]
impl<L> EntityExtractor for LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync + ?Sized,
{
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        let prompt = self.build_prompt(&chunk.content);

        // WHY reasoning_effort="none" + explicit max_tokens:
        // Reasoning models (gpt-5-nano, gpt-5-mini, o-series) exhaust all completion_tokens
        // on chain-of-thought when no limit is set (reasoning_tokens = completion_tokens → 0
        // net output tokens → empty JSON → parse error). Setting reasoning_effort="none"
        // disables CoT for extraction tasks where structured JSON output is required.
        // Non-reasoning models silently ignore this field.
        let options = edgequake_llm::traits::CompletionOptions {
            max_tokens: Some(16384),
            // WHY: GPT-5/o-series and some nano models reject explicit temperature values.
            // Omitting the field preserves compatibility while keeping deterministic prompts.
            temperature: effective_temperature_for_model(self.llm_provider.model(), 0.0),
            reasoning_effort: Some("none".to_string()),
            ..Default::default()
        };

        let response = self
            .llm_provider
            .complete_with_options(&prompt, &options)
            .await
            .map_err(|e| PipelineError::ExtractionError(format!("LLM error: {}", e)))?;

        let mut result = self.parse_response(&response.content, &chunk.id)?;

        // Set token usage from the LLM response
        result.input_tokens = response.prompt_tokens;
        result.output_tokens = response.completion_tokens;

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

    // ── try_recover_truncated_json ──────────────────────────────────────────

    /// A well-formed JSON value should be returned via the primary parse path;
    /// `try_recover_truncated_json` is only called on error, but we can still
    /// call it directly and confirm it handles complete input gracefully.
    #[test]
    fn test_recover_already_complete_json() {
        let complete = r#"{"entities":[],"relationships":[]}"#;
        let result =
            LLMExtractor::<edgequake_llm::MockProvider>::try_recover_truncated_json(complete);
        // A complete JSON value must be recoverable (empty suffix matches).
        assert!(result.is_some());
    }

    /// Simulates truncation after the last complete entity object — the most
    /// common failure mode from token-budget exhaustion.
    #[test]
    fn test_recover_truncated_after_complete_entity() {
        // Truncated mid-array: the closing `]}` for entities array and `}` for
        // the root object are missing.
        let truncated = r#"{"entities":[{"name":"A","type":"PERSON","description":"test"},"#;
        // After stripping the trailing comma the recovered JSON should be parseable.
        // Our strategy appends suffixes; one of them must close it cleanly.
        // (The implementation may or may not recover this exact form — what matters
        // is that it does NOT panic and either returns Some or None gracefully.)
        let result = LLMExtractor::<edgequake_llm::MockProvider>::try_recover_truncated_json(
            &truncated[..truncated.len() - 1], // remove trailing comma to allow suffix matching
        );
        // We expect at least one suffix to produce a valid JSON object.
        assert!(result.is_some());
        if let Some(v) = result {
            assert!(v.is_object());
        }
    }

    /// Simulates the exact production failure: entity array not closed.
    #[test]
    fn test_recover_truncated_entities_array_open() {
        let truncated = r#"{"entities":[{"name":"SEISMOLOGY","type":"CONCEPT","description":"Study of earthquakes"}],"relationships":["#;
        let result =
            LLMExtractor::<edgequake_llm::MockProvider>::try_recover_truncated_json(truncated);
        assert!(
            result.is_some(),
            "Should recover with open relationships array"
        );
        if let Some(v) = result {
            let entities = v["entities"].as_array().expect("entities must be an array");
            assert_eq!(entities.len(), 1);
            assert_eq!(entities[0]["name"], "SEISMOLOGY");
        }
    }

    /// Garbage input that cannot be recovered should return None, not panic.
    #[test]
    fn test_recover_returns_none_for_unrecoverable_input() {
        let garbage = "not json at all ///";
        let result =
            LLMExtractor::<edgequake_llm::MockProvider>::try_recover_truncated_json(garbage);
        assert!(result.is_none());
    }

    // ── parse_response partial recovery integration ─────────────────────────

    /// `parse_response` must succeed on truncated JSON by recovering entities
    /// that were fully emitted before the token budget was hit.
    #[test]
    fn test_parse_response_recovers_partial_json() {
        use std::sync::Arc;
        let provider = Arc::new(edgequake_llm::MockProvider::default());
        let extractor = LLMExtractor::new(provider);

        // Simulate a truncated response: relationships array was never closed.
        let truncated_response = r#"{"entities":[{"name":"ALICE","type":"PERSON","description":"A scientist"},{"name":"BOB","type":"PERSON","description":"A colleague"}],"relationships":["#;

        let result = extractor.parse_response(truncated_response, "chunk_001");
        assert!(
            result.is_ok(),
            "parse_response should recover, got: {:?}",
            result
        );
        let extraction = result.unwrap();
        assert_eq!(
            extraction.entities.len(),
            2,
            "Both complete entities should be salvaged"
        );
        assert_eq!(extraction.entities[0].name, "ALICE");
        assert_eq!(extraction.entities[1].name, "BOB");
    }

    /// `parse_response` must still fail hard when the JSON is truly unrecoverable.
    #[test]
    fn test_parse_response_fails_on_unrecoverable_json() {
        use std::sync::Arc;
        let provider = Arc::new(edgequake_llm::MockProvider::default());
        let extractor = LLMExtractor::new(provider);

        let result = extractor.parse_response("this is not json", "chunk_bad");
        assert!(result.is_err(), "Unrecoverable JSON must return an error");
    }
}
