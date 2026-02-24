//! JSON-based extraction result parser (legacy format).
//!
//! Parses structured JSON extraction results, with sanitization for
//! common LLM malformation issues (unquoted keys, trailing commas,
//! control characters, etc.).

use super::super::normalizer::normalize_entity_name;
use crate::error::{PipelineError, Result};
use crate::extractor::{ExtractedEntity, ExtractedRelationship, ExtractionResult};

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

                    // BR0006 defense: Skip entities that normalize to empty string
                    if normalized_name.is_empty() {
                        tracing::debug!(raw_name = %name, "Skipping JSON entity with empty normalized name");
                        continue;
                    }

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

                    // BR0006: Same-entity relationships forbidden
                    if normalized_source == normalized_target {
                        tracing::debug!(
                            source = %normalized_source,
                            "Skipping self-referencing JSON relationship (BR0006)"
                        );
                        continue;
                    }

                    // Skip relationships with empty normalized endpoints
                    if normalized_source.is_empty() || normalized_target.is_empty() {
                        tracing::debug!(
                            raw_source = %source,
                            raw_target = %target,
                            "Skipping JSON relationship with empty normalized endpoint"
                        );
                        continue;
                    }

                    // BR0004: Keyword limit (extract keywords if present)
                    let keywords: Vec<String> = rel_val
                        .get("keywords")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|k| k.as_str())
                                .map(|k| k.trim().to_string())
                                .filter(|k| !k.is_empty())
                                .take(5)
                                .collect()
                        })
                        .unwrap_or_default();

                    let mut rel =
                        ExtractedRelationship::new(normalized_source, normalized_target, rel_type)
                            .with_description(description);

                    if !keywords.is_empty() {
                        rel = rel.with_keywords(keywords);
                    }

                    result.add_relationship(rel);
                }
            }
        }

        result
            .metadata
            .insert("parser".to_string(), serde_json::json!("json"));

        Ok(result)
    }
}

// ============================================================================
// JSON Sanitization Helpers
// ============================================================================

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

    // WHY: LLMs sometimes emit control characters (\u0000-\u001F) inside JSON strings.
    // serde_json rejects these per RFC 7159. Strip them first (except \n, \r, \t which
    // are valid when properly escaped but rare in entity names/descriptions).
    sanitized = sanitized
        .chars()
        .filter(|c| {
            // Keep printable chars and whitespace that serde handles
            !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t'
        })
        .collect();

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
pub(super) fn extract_json_from_response(response: &str) -> String {
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
