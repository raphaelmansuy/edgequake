//! Extraction result parsers.
//!
//! Provides parsers for both tuple-based (SOTA) and JSON-based extraction formats,
//! plus a hybrid parser for graceful migration.
//!
//! # WHY Tuple Format Over JSON
//!
//! The tuple-delimited format (`entity<|#|>Name<|#|>TYPE<|#|>Description`) is used
//! because it's significantly more robust than JSON for LLM outputs:
//!
//! 1. **Partial output recovery**: If LLM output is truncated or streaming is
//!    interrupted, valid lines up to that point can still be parsed. JSON requires
//!    complete, valid syntax to parse anything.
//!
//! 2. **No escaping issues**: JSON requires proper escaping of quotes, backslashes,
//!    and special characters. LLMs frequently produce malformed JSON with:
//!    - Unescaped quotes in descriptions
//!    - Missing closing braces
//!    - Invalid unicode escapes
//!
//! 3. **Line-by-line processing**: Each tuple is independent, allowing streaming
//!    extraction and early termination without buffering the full response.
//!
//! 4. **LightRAG proven**: This format is battle-tested in the LightRAG paper
//!    and implementation with millions of extractions.
//!
//! The hybrid parser falls back to JSON parsing for backward compatibility,
//! but tuple format is preferred for production reliability.

use super::normalizer::normalize_entity_name;
use super::{DEFAULT_COMPLETION_DELIMITER, DEFAULT_TUPLE_DELIMITER};
use crate::error::{PipelineError, Result};
use crate::extractor::{ExtractedEntity, ExtractedRelationship, ExtractionResult};

/// Parser for tuple-delimited extraction results (SOTA format).
///
/// Parses extraction output in the format:
/// ```text
/// entity<|#|>Name<|#|>TYPE<|#|>Description
/// relation<|#|>Source<|#|>Target<|#|>keywords<|#|>Description
/// <|COMPLETE|>
/// ```
#[derive(Debug, Clone)]
pub struct TupleParser {
    tuple_delimiter: String,
    completion_delimiter: String,
}

impl Default for TupleParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TupleParser {
    /// Create a new tuple parser with default delimiters.
    pub fn new() -> Self {
        Self {
            tuple_delimiter: DEFAULT_TUPLE_DELIMITER.to_string(),
            completion_delimiter: DEFAULT_COMPLETION_DELIMITER.to_string(),
        }
    }

    /// Create with custom delimiters.
    pub fn with_delimiters(tuple: &str, completion: &str) -> Self {
        Self {
            tuple_delimiter: tuple.to_string(),
            completion_delimiter: completion.to_string(),
        }
    }

    /// Parse extraction results from tuple format.
    pub fn parse(&self, response: &str, chunk_id: &str) -> Result<ExtractionResult> {
        let mut entities = Vec::new();
        let mut relationships = Vec::new();
        let mut is_complete = false;
        let mut parse_errors = 0;

        for line in response.lines() {
            let line = line.trim();

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Check for completion signal
            if line.contains(&self.completion_delimiter) {
                is_complete = true;
                continue;
            }

            // Skip lines that don't contain our delimiter
            if !line.contains(&self.tuple_delimiter) {
                continue;
            }

            let parts: Vec<&str> = line.split(&self.tuple_delimiter).collect();

            match parts.first().map(|s| s.trim().to_lowercase()).as_deref() {
                Some("entity") if parts.len() >= 4 => {
                    let name = parts[1].trim();
                    let entity_type = parts[2].trim().to_uppercase();
                    let description = parts[3].trim();

                    // Skip empty entities
                    if name.is_empty() {
                        parse_errors += 1;
                        continue;
                    }

                    let normalized_name = normalize_entity_name(name);
                    let entity = ExtractedEntity::new(normalized_name, entity_type, description);
                    entities.push(entity);
                }
                Some("relation") | Some("relationship") if parts.len() >= 5 => {
                    let source = parts[1].trim();
                    let target = parts[2].trim();
                    let keywords_str = parts[3].trim();
                    let description = parts[4].trim();

                    // Skip empty relationships
                    if source.is_empty() || target.is_empty() {
                        parse_errors += 1;
                        continue;
                    }

                    // Parse keywords
                    let keywords: Vec<String> = keywords_str
                        .split(',')
                        .map(|k| k.trim().to_string())
                        .filter(|k| !k.is_empty())
                        .collect();

                    // Determine relationship type from keywords
                    let relation_type = keywords
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "RELATED_TO".to_string());

                    let normalized_source = normalize_entity_name(source);
                    let normalized_target = normalize_entity_name(target);

                    let relationship = ExtractedRelationship::new(
                        normalized_source,
                        normalized_target,
                        relation_type,
                    )
                    .with_description(description)
                    .with_keywords(keywords);

                    relationships.push(relationship);
                }
                _ => {
                    // Skip unrecognized lines, log for debugging
                    tracing::debug!(line = %line, "Skipping unrecognized line in tuple extraction");
                    parse_errors += 1;
                }
            }
        }

        let mut result = ExtractionResult::new(chunk_id);
        result.entities = entities;
        result.relationships = relationships;
        result
            .metadata
            .insert("is_complete".to_string(), serde_json::json!(is_complete));
        result
            .metadata
            .insert("parser".to_string(), serde_json::json!("tuple"));
        result
            .metadata
            .insert("parse_errors".to_string(), serde_json::json!(parse_errors));

        Ok(result)
    }

    /// Check if the response appears complete.
    pub fn is_complete(&self, response: &str) -> bool {
        response.contains(&self.completion_delimiter)
    }
}

/// Parser for JSON-based extraction results (legacy format).
#[derive(Debug, Clone, Default)]
pub struct JsonExtractionParser;

impl JsonExtractionParser {
    /// Create a new JSON parser.
    pub fn new() -> Self {
        Self
    }

    /// Parse extraction results from JSON format.
    pub fn parse(&self, response: &str, chunk_id: &str) -> Result<ExtractionResult> {
        let mut result = ExtractionResult::new(chunk_id);

        // Try to extract JSON from the response
        let json_str = extract_json_from_response(response);

        // Sanitize JSON to fix common LLM mistakes
        let sanitized_json = sanitize_json(&json_str);

        let parsed: serde_json::Value = serde_json::from_str(&sanitized_json).map_err(|e| {
            // WHY: Truncate for logging using char boundaries to avoid UTF-8 panics
            // Direct byte slicing like &str[..300] can panic if byte 300 falls inside a multi-byte char
            let json_preview = sanitized_json.chars().take(300).collect::<String>();
            let json_short = sanitized_json.chars().take(200).collect::<String>();

            tracing::warn!(
                error = %e,
                json_preview = %json_preview,
                "JSON parsing failed - LLM returned malformed JSON"
            );
            PipelineError::ExtractionError(format!(
                "Invalid JSON: {} - First 200 chars: {}",
                e, json_short
            ))
        })?;

        // Extract entities
        if let Some(entities) = parsed.get("entities").and_then(|v| v.as_array()) {
            for entity_val in entities {
                if let (Some(name), Some(entity_type), Some(description)) = (
                    entity_val.get("name").and_then(|v| v.as_str()),
                    entity_val.get("type").and_then(|v| v.as_str()),
                    entity_val.get("description").and_then(|v| v.as_str()),
                ) {
                    let normalized_name = normalize_entity_name(name);
                    result.add_entity(ExtractedEntity::new(
                        normalized_name,
                        entity_type.to_uppercase(),
                        description,
                    ));
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

                    let normalized_source = normalize_entity_name(source);
                    let normalized_target = normalize_entity_name(target);

                    result.add_relationship(
                        ExtractedRelationship::new(normalized_source, normalized_target, rel_type)
                            .with_description(description),
                    );
                }
            }
        }

        result
            .metadata
            .insert("parser".to_string(), serde_json::json!("json"));

        Ok(result)
    }
}

/// Hybrid parser supporting both JSON and Tuple formats.
///
/// Provides a migration path from JSON to tuple-based extraction
/// with automatic format detection and fallback.
#[derive(Debug, Clone)]
pub struct HybridExtractionParser {
    json_parser: JsonExtractionParser,
    tuple_parser: TupleParser,
    prefer_tuple: bool,
}

impl Default for HybridExtractionParser {
    fn default() -> Self {
        Self::new(true)
    }
}

impl HybridExtractionParser {
    /// Create a new hybrid parser.
    ///
    /// # Arguments
    /// * `prefer_tuple` - If true, prefer tuple parsing when format is ambiguous
    pub fn new(prefer_tuple: bool) -> Self {
        Self {
            json_parser: JsonExtractionParser::new(),
            tuple_parser: TupleParser::new(),
            prefer_tuple,
        }
    }

    /// Create with custom tuple delimiters.
    pub fn with_tuple_delimiters(mut self, tuple: &str, completion: &str) -> Self {
        self.tuple_parser = TupleParser::with_delimiters(tuple, completion);
        self
    }

    /// Parse extraction result, auto-detecting format.
    pub fn parse(&self, response: &str, chunk_id: &str) -> Result<ExtractionResult> {
        // Detect format by content
        let has_tuple_markers = response.contains(DEFAULT_TUPLE_DELIMITER)
            || response.contains("entity<|")
            || response.contains("relation<|");
        let has_json_markers = response.trim_start().starts_with('{')
            || response.contains("```json")
            || response.contains("\"entities\"")
            || response.contains("\"relationships\"");

        tracing::debug!(
            has_tuple = has_tuple_markers,
            has_json = has_json_markers,
            prefer_tuple = self.prefer_tuple,
            response_len = response.len(),
            "Detecting extraction format"
        );

        // Determine which parser to use
        if has_tuple_markers && (!has_json_markers || self.prefer_tuple) {
            // Use tuple parser (more robust)
            match self.tuple_parser.parse(response, chunk_id) {
                Ok(result) if !result.entities.is_empty() || !result.relationships.is_empty() => {
                    tracing::debug!(
                        entities = result.entities.len(),
                        relationships = result.relationships.len(),
                        "Tuple parsing succeeded"
                    );
                    return Ok(result);
                }
                Ok(result) => {
                    // If tuple parsing returned empty but we have JSON markers, try JSON
                    if result.entities.is_empty()
                        && result.relationships.is_empty()
                        && has_json_markers
                    {
                        tracing::debug!("Tuple parsing returned empty, trying JSON fallback");
                    } else {
                        return Ok(result);
                    }
                }
                Err(e) => {
                    tracing::debug!(error = %e, "Tuple parsing failed, trying JSON fallback");
                }
            }
        }

        // Try JSON parser
        if has_json_markers {
            match self.json_parser.parse(response, chunk_id) {
                Ok(result) => {
                    tracing::debug!(
                        entities = result.entities.len(),
                        relationships = result.relationships.len(),
                        "JSON parsing succeeded"
                    );
                    return Ok(result);
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "JSON parsing failed - attempting tuple fallback"
                    );
                    // If we also have tuple markers, try that as last resort
                    if has_tuple_markers {
                        tracing::info!("Falling back to tuple parsing after JSON failure");
                        return self.tuple_parser.parse(response, chunk_id);
                    }
                    // If no tuple markers, try tuple anyway (it's more lenient)
                    tracing::info!(
                        "No tuple markers detected but trying tuple parsing as last resort"
                    );
                    match self.tuple_parser.parse(response, chunk_id) {
                        Ok(result)
                            if !result.entities.is_empty() || !result.relationships.is_empty() =>
                        {
                            tracing::info!(
                                entities = result.entities.len(),
                                relationships = result.relationships.len(),
                                "Tuple fallback succeeded despite no markers"
                            );
                            return Ok(result);
                        }
                        Ok(_) => {
                            // Return original JSON error if tuple also failed
                            return Err(e);
                        }
                        Err(_) => {
                            // Return original JSON error
                            return Err(e);
                        }
                    }
                }
            }
        }

        // Neither format detected, try tuple (more lenient)
        if has_tuple_markers {
            return self.tuple_parser.parse(response, chunk_id);
        }

        // Last resort: try JSON in case it's just not properly formatted
        self.json_parser.parse(response, chunk_id)
    }

    /// Get the underlying tuple parser.
    pub fn tuple_parser(&self) -> &TupleParser {
        &self.tuple_parser
    }

    /// Get the underlying JSON parser.
    pub fn json_parser(&self) -> &JsonExtractionParser {
        &self.json_parser
    }
}

/// Sanitize malformed JSON from LLM responses.
///
/// # WHY: LLMs Produce Malformed JSON
///
/// Common issues this fixes:
/// 1. Unquoted keys: `{name: "value"}` → `{"name": "value"}`
/// 2. Single quotes: `{'name': 'value'}` → `{"name": "value"}`
/// 3. Trailing commas: `{"a": 1,}` → `{"a": 1}`
/// 4. Comments: `{"a": 1 // comment}` → `{"a": 1}`
/// 5. Unescaped quotes in strings (best-effort)
///
/// This is a best-effort fix. If sanitization fails, the original
/// JSON error will be returned to the caller.
fn sanitize_json(json: &str) -> String {
    let mut sanitized = json.to_string();

    // Remove JavaScript-style comments
    // Single-line: // comment
    let re_single_comment = regex::Regex::new(r"//.*$").unwrap();
    sanitized = re_single_comment.replace_all(&sanitized, "").to_string();

    // Multi-line: /* comment */
    let re_multi_comment = regex::Regex::new(r"/\*.*?\*/").unwrap();
    sanitized = re_multi_comment.replace_all(&sanitized, "").to_string();

    // Remove trailing commas before } or ]
    let re_trailing_comma = regex::Regex::new(r",(\s*[}\]])").unwrap();
    sanitized = re_trailing_comma.replace_all(&sanitized, "$1").to_string();

    // Fix single quotes to double quotes (be careful with apostrophes in text)
    // This is a simple heuristic: replace ' with " only when it looks like a JSON delimiter
    // Pattern: '{key}' or ':{value}' at JSON structure positions
    let re_single_quote_key = regex::Regex::new(r"'([a-zA-Z_][a-zA-Z0-9_]*)'(\s*:)").unwrap();
    sanitized = re_single_quote_key
        .replace_all(&sanitized, "\"$1\"$2")
        .to_string();

    let re_single_quote_val = regex::Regex::new(r":\s*'([^']*)'").unwrap();
    sanitized = re_single_quote_val
        .replace_all(&sanitized, ": \"$1\"")
        .to_string();

    // Fix unquoted keys: {name: "value"} → {"name": "value"}
    // Match: word characters followed by colon
    let re_unquoted_key = regex::Regex::new(r#"([,{]\s*)([a-zA-Z_][a-zA-Z0-9_]*)(\s*:)"#).unwrap();
    sanitized = re_unquoted_key
        .replace_all(&sanitized, "$1\"$2\"$3")
        .to_string();

    sanitized
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

    // Try regular code block
    if let Some(start) = response.find("```") {
        if let Some(end) = response[start + 3..].find("```") {
            let content = response[start + 3..start + 3 + end].trim();
            // Check if it starts like JSON
            if content.starts_with('{') {
                return content.to_string();
            }
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
    fn test_tuple_parser_entities() {
        let parser = TupleParser::new();
        let response = r#"entity<|#|>John Doe<|#|>PERSON<|#|>A software developer
entity<|#|>Acme Corp<|#|>ORGANIZATION<|#|>A technology company
<|COMPLETE|>"#;

        let result = parser.parse(response, "chunk-1").unwrap();

        assert_eq!(result.entities.len(), 2);
        assert_eq!(result.entities[0].name, "JOHN_DOE");
        assert_eq!(result.entities[0].entity_type, "PERSON");
        assert_eq!(result.entities[1].name, "ACME_CORP");
        assert!(result
            .metadata
            .get("is_complete")
            .unwrap()
            .as_bool()
            .unwrap());
    }

    #[test]
    fn test_tuple_parser_relationships() {
        let parser = TupleParser::new();
        let response = r#"entity<|#|>Alice<|#|>PERSON<|#|>A researcher
entity<|#|>Bob<|#|>PERSON<|#|>Another researcher
relation<|#|>Alice<|#|>Bob<|#|>collaboration, research<|#|>Alice and Bob work together
<|COMPLETE|>"#;

        let result = parser.parse(response, "chunk-1").unwrap();

        assert_eq!(result.entities.len(), 2);
        assert_eq!(result.relationships.len(), 1);
        assert_eq!(result.relationships[0].source, "ALICE");
        assert_eq!(result.relationships[0].target, "BOB");
        assert_eq!(result.relationships[0].keywords.len(), 2);
    }

    #[test]
    fn test_tuple_parser_incomplete() {
        let parser = TupleParser::new();
        let response = r#"entity<|#|>John<|#|>PERSON<|#|>A person"#;

        let result = parser.parse(response, "chunk-1").unwrap();

        assert_eq!(result.entities.len(), 1);
        assert!(!result
            .metadata
            .get("is_complete")
            .unwrap()
            .as_bool()
            .unwrap());
    }

    #[test]
    fn test_tuple_parser_malformed_lines() {
        let parser = TupleParser::new();
        let response = r#"entity<|#|>Valid<|#|>PERSON<|#|>Valid entity
some random text here
entity<|#|><|#|>PERSON<|#|>Empty name should skip
entity<|#|>Also Valid<|#|>CONCEPT<|#|>Another valid
<|COMPLETE|>"#;

        let result = parser.parse(response, "chunk-1").unwrap();

        assert_eq!(result.entities.len(), 2); // Only valid entities
        assert!(
            result
                .metadata
                .get("parse_errors")
                .unwrap()
                .as_u64()
                .unwrap()
                > 0
        );
    }

    #[test]
    fn test_json_parser() {
        let parser = JsonExtractionParser::new();
        let response = r#"
```json
{
  "entities": [
    {"name": "John Doe", "type": "PERSON", "description": "A developer"}
  ],
  "relationships": [
    {"source": "John", "target": "Company", "type": "WORKS_AT", "description": "Employment"}
  ]
}
```
"#;

        let result = parser.parse(response, "chunk-1").unwrap();

        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].name, "JOHN_DOE");
        assert_eq!(result.relationships.len(), 1);
    }

    #[test]
    fn test_hybrid_parser_tuple() {
        let parser = HybridExtractionParser::new(true);
        let response = r#"entity<|#|>Test<|#|>CONCEPT<|#|>A test entity
<|COMPLETE|>"#;

        let result = parser.parse(response, "chunk-1").unwrap();

        assert_eq!(result.entities.len(), 1);
        assert_eq!(
            result.metadata.get("parser").unwrap().as_str().unwrap(),
            "tuple"
        );
    }

    #[test]
    fn test_hybrid_parser_json() {
        let parser = HybridExtractionParser::new(true);
        let response = r#"{"entities": [{"name": "Test", "type": "CONCEPT", "description": "A test"}], "relationships": []}"#;

        let result = parser.parse(response, "chunk-1").unwrap();

        assert_eq!(result.entities.len(), 1);
        assert_eq!(
            result.metadata.get("parser").unwrap().as_str().unwrap(),
            "json"
        );
    }

    #[test]
    fn test_extract_json_from_response() {
        // Test code block extraction
        let response = "Here's the result:\n```json\n{\"key\": \"value\"}\n```\nDone!";
        assert_eq!(extract_json_from_response(response), "{\"key\": \"value\"}");

        // Test raw JSON
        let response = "Response: {\"key\": \"value\"}";
        assert_eq!(extract_json_from_response(response), "{\"key\": \"value\"}");
    }

    #[test]
    fn test_tuple_parser_is_complete() {
        let parser = TupleParser::new();

        assert!(parser.is_complete("entity<|#|>X<|#|>Y<|#|>Z\n<|COMPLETE|>"));
        assert!(!parser.is_complete("entity<|#|>X<|#|>Y<|#|>Z"));
    }
}
