//! SOTA Prompt Templates for Entity Extraction
//!
//! This module contains production-quality prompts ported from LightRAG,
//! implementing tuple-based extraction format for robustness.
//!
//! ## Key Features
//!
//! - **Tuple Format**: Uses `<|#|>` delimiter for robust parsing
//! - **Completion Signal**: `<|COMPLETE|>` for reliable extraction detection
//! - **Multi-Language**: Configurable `{language}` parameter
//! - **N-ary Decomposition**: Explicit instructions for complex relationships
//! - **Entity Naming**: Title case with consistent naming rules
//!
//! ## Usage
//!
//! ```rust,ignore
//! use edgequake_pipeline::prompts::{EntityExtractionPrompts, TupleParser};
//!
//! let prompts = EntityExtractionPrompts::default();
//! let system_prompt = prompts.system_prompt(&["PERSON", "ORGANIZATION"], "English");
//! let user_prompt = prompts.user_prompt("Some text...", &["PERSON"], "English");
//!
//! // Parse LLM response
//! let parser = TupleParser::new();
//! let result = parser.parse(&llm_response, "chunk-1")?;
//! ```

mod entity_extraction;
mod entity_type_policy;
mod json_extract;
mod json_prompts;
mod normalizer;
mod parser;
mod summarization;

pub use entity_extraction::EntityExtractionPrompts;
pub use entity_type_policy::{
    enforce_entity_type, json_entity_types_prompt_section, normalize_type_token,
    sota_entity_type_instruction, EntityExtractionSchema, METADATA_ENTITY_TYPES_STRICT,
};
pub use json_extract::extract_json_from_response;
pub use json_prompts::{json_extraction_prompt, json_gleaning_prompt, JSON_OUTPUT_FORMAT_SECTION};
pub use normalizer::normalize_entity_name;
pub use parser::{
    detect_format_markers, ExtractionResultParser, HybridExtractionParser, JsonExtractionParser,
    JsonParseOptions, TupleParser,
};
pub use summarization::SummarizationPrompts;

/// Default tuple delimiter for extraction output.
pub const DEFAULT_TUPLE_DELIMITER: &str = "<|#|>";

/// Completion signal to detect complete extractions.
pub const DEFAULT_COMPLETION_DELIMITER: &str = "<|COMPLETE|>";

/// Supported output languages for extraction.
pub const SUPPORTED_LANGUAGES: &[&str] = &[
    "English",
    "Chinese",
    "Japanese",
    "Korean",
    "Spanish",
    "French",
    "German",
    "Portuguese",
    "Italian",
    "Russian",
];

/// Default entity types for extraction.
pub fn default_entity_types() -> Vec<String> {
    vec![
        "PERSON".to_string(),
        "ORGANIZATION".to_string(),
        "LOCATION".to_string(),
        "EVENT".to_string(),
        "CONCEPT".to_string(),
        "TECHNOLOGY".to_string(),
        "PRODUCT".to_string(),
        "DATE".to_string(),
        "DOCUMENT".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_entity_types() {
        let types = default_entity_types();
        assert!(types.contains(&"PERSON".to_string()));
        assert!(types.contains(&"ORGANIZATION".to_string()));
        assert!(types.len() >= 7);
    }

    #[test]
    fn test_supported_languages() {
        assert!(SUPPORTED_LANGUAGES.contains(&"English"));
        assert!(SUPPORTED_LANGUAGES.contains(&"Chinese"));
    }
}
