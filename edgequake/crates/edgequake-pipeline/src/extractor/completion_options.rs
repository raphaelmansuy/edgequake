//! Shared LLM completion options for extraction extractors (SPEC-017 DRY-008).

use edgequake_llm::traits::CompletionOptions;

use super::temperature::effective_temperature_for_model;
use super::types::ExtractionResult;

/// Build extraction [`CompletionOptions`] with reasoning disabled and temperature gating.
///
/// WHY reasoning_effort="none": Reasoning models exhaust completion budget on CoT,
/// producing empty structured output. Non-reasoning models ignore this field.
pub fn extraction_completion_options(model: &str, max_tokens: usize) -> CompletionOptions {
    CompletionOptions {
        max_tokens: Some(max_tokens),
        temperature: effective_temperature_for_model(model, 0.0),
        reasoning_effort: Some("none".to_string()),
        ..Default::default()
    }
}

/// Adaptive chunk size recommendation based on document size (bytes).
pub fn recommended_chunk_size_for_bytes(chunk_size_bytes: usize) -> usize {
    if chunk_size_bytes > 100_000 {
        600
    } else if chunk_size_bytes > 50_000 {
        800
    } else {
        1200
    }
}

/// Copy token usage from an LLM response into an extraction result.
pub fn assign_token_usage(
    result: &mut ExtractionResult,
    input_tokens: usize,
    output_tokens: usize,
) {
    result.input_tokens = input_tokens;
    result.output_tokens = output_tokens;
}
