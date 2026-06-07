//! Parser registry trait for OCP-friendly format extension (SPEC-017 SOLID-O-001).

use super::super::DEFAULT_TUPLE_DELIMITER;
use super::{JsonExtractionParser, TupleParser};
use crate::error::Result;
use crate::extractor::ExtractionResult;

/// Parses LLM extraction output into structured entities and relationships.
pub trait ExtractionResultParser: Send + Sync {
    /// Stable parser identifier (`"tuple"` | `"json"`).
    fn id(&self) -> &'static str;

    /// Parse a response for the given chunk.
    fn parse(&self, response: &str, chunk_id: &str) -> Result<ExtractionResult>;
}

impl ExtractionResultParser for TupleParser {
    fn id(&self) -> &'static str {
        "tuple"
    }

    fn parse(&self, response: &str, chunk_id: &str) -> Result<ExtractionResult> {
        TupleParser::parse(self, response, chunk_id)
    }
}

impl ExtractionResultParser for JsonExtractionParser {
    fn id(&self) -> &'static str {
        "json"
    }

    fn parse(&self, response: &str, chunk_id: &str) -> Result<ExtractionResult> {
        JsonExtractionParser::parse(self, response, chunk_id)
    }
}

/// Detect tuple vs JSON markers in raw LLM output (shared by hybrid fallback chain).
pub fn detect_format_markers(response: &str) -> (bool, bool) {
    let has_tuple_markers = response.contains(DEFAULT_TUPLE_DELIMITER)
        || response.contains("entity<|")
        || response.contains("relation<|");
    let has_json_markers = response.trim_start().starts_with('{')
        || response.contains("```json")
        || response.contains("\"entities\"")
        || response.contains("\"relationships\"");
    (has_tuple_markers, has_json_markers)
}
