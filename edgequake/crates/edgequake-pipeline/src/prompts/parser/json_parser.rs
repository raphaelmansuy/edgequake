//! JSON-based extraction result parser (legacy format).
//!
//! Parses structured JSON extraction results, with sanitization for
//! common LLM malformation issues (unquoted keys, trailing commas,
//! control characters, etc.).

use super::super::entity_type_policy::{enforce_entity_type, EntityExtractionSchema};
use super::super::normalizer::normalize_entity_name;
use crate::error::{PipelineError, Result};
use crate::extractor::{ExtractedEntity, ExtractedRelationship, ExtractionResult};
use crate::prompts::extract_json_from_response;

/// Parser for JSON-based extraction results (legacy format).
#[derive(Debug, Clone, Default)]
pub struct JsonExtractionParser;

/// Tunable JSON parse behavior (SPEC-017: unify LLM + gleaning paths).
#[derive(Debug, Clone, Default)]
pub struct JsonParseOptions<'a> {
    /// When set, entity types are enforced/remapped via workspace schema.
    pub entity_schema: Option<&'a EntityExtractionSchema>,
    /// Attempt suffix-based recovery when JSON is truncated mid-stream.
    pub recover_truncated: bool,
    /// Return empty extraction when no JSON is found (LLM flaky output).
    pub empty_on_missing_json: bool,
}

impl JsonExtractionParser {
    /// Create a new JSON parser.
    pub fn new() -> Self {
        Self
    }

    /// Parse extraction results from JSON format (strict gleaning/default path).
    pub fn parse(&self, response: &str, chunk_id: &str) -> Result<ExtractionResult> {
        self.parse_with_options(response, chunk_id, JsonParseOptions::default())
    }

    /// Parse with explicit options (LLM path: schema enforcement + truncated recovery).
    pub fn parse_with_options(
        &self,
        response: &str,
        chunk_id: &str,
        options: JsonParseOptions<'_>,
    ) -> Result<ExtractionResult> {
        let mut result = ExtractionResult::new(chunk_id);

        let json_str = extract_json_from_response(response);
        let sanitized_json = sanitize_json(&json_str);

        if sanitized_json.trim().is_empty() {
            if options.empty_on_missing_json {
                tracing::warn!(chunk_id = %chunk_id, "LLM returned empty extraction JSON; treating as empty result");
                return Ok(result);
            }
            return Err(PipelineError::ExtractionError(
                "Invalid JSON: empty response".to_string(),
            ));
        }

        let parsed = match serde_json::from_str::<serde_json::Value>(&sanitized_json) {
            Ok(v) => v,
            Err(primary_err) => {
                if options.recover_truncated {
                    tracing::warn!(
                        chunk_id = %chunk_id,
                        error = %primary_err,
                        "JSON parse failed — attempting partial recovery of truncated output"
                    );
                    match try_recover_truncated_json(&sanitized_json) {
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
                } else {
                    let json_preview = sanitized_json.chars().take(300).collect::<String>();
                    let json_short = sanitized_json.chars().take(200).collect::<String>();

                    tracing::warn!(
                        error = %primary_err,
                        json_preview = %json_preview,
                        "JSON parsing failed - LLM returned malformed JSON"
                    );
                    return Err(PipelineError::ExtractionError(format!(
                        "Invalid JSON: {} - First 200 chars: {}",
                        primary_err, json_short
                    )));
                }
            }
        };

        populate_from_value(&mut result, &parsed, options.entity_schema);

        result
            .metadata
            .insert("parser".to_string(), serde_json::json!("json"));

        Ok(result)
    }
}

fn populate_from_value(
    result: &mut ExtractionResult,
    parsed: &serde_json::Value,
    entity_schema: Option<&EntityExtractionSchema>,
) {
    if let Some(entities) = parsed.get("entities").and_then(|v| v.as_array()) {
        for entity_val in entities {
            if let (Some(name), Some(entity_type), Some(description)) = (
                entity_val.get("name").and_then(|v| v.as_str()),
                entity_val.get("type").and_then(|v| v.as_str()),
                entity_val.get("description").and_then(|v| v.as_str()),
            ) {
                let normalized_name = normalize_entity_name(name);

                if normalized_name.is_empty() {
                    tracing::debug!(raw_name = %name, "Skipping JSON entity with empty normalized name");
                    continue;
                }

                let enforced_type = if let Some(schema) = entity_schema {
                    let (enforced, remapped) = enforce_entity_type(entity_type, schema);
                    if remapped {
                        tracing::debug!(
                            raw_type = %entity_type,
                            enforced = %enforced,
                            "Remapped entity type to workspace schema"
                        );
                    }
                    enforced
                } else {
                    entity_type.to_uppercase()
                };

                result.add_entity(ExtractedEntity::new(
                    normalized_name,
                    &enforced_type,
                    description,
                ));
            }
        }
    }

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

                if normalized_source == normalized_target {
                    tracing::debug!(
                        source = %normalized_source,
                        "Skipping self-referencing JSON relationship (BR0006)"
                    );
                    continue;
                }

                if normalized_source.is_empty() || normalized_target.is_empty() {
                    tracing::debug!(
                        raw_source = %source,
                        raw_target = %target,
                        "Skipping JSON relationship with empty normalized endpoint"
                    );
                    continue;
                }

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
}

/// Attempt to recover a valid JSON value from a truncated string.
fn try_recover_truncated_json(s: &str) -> Option<serde_json::Value> {
    let suffixes: &[&str] = &["", "}", "]}", "}]}", "]}]}", "}]}]}", "\"}]}]}", "\"}]}"];
    for &suffix in suffixes {
        let candidate = format!("{}{}", s, suffix);
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&candidate) {
            return Some(v);
        }
    }
    None
}

/// Sanitize malformed JSON from LLM responses.
fn sanitize_json(json: &str) -> String {
    let mut sanitized = json.to_string();

    sanitized = sanitized
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
        .collect();

    let re_single_comment = regex::Regex::new(r"//.*$").unwrap();
    sanitized = re_single_comment.replace_all(&sanitized, "").to_string();

    let re_multi_comment = regex::Regex::new(r"/\*.*?\*/").unwrap();
    sanitized = re_multi_comment.replace_all(&sanitized, "").to_string();

    let re_trailing_comma = regex::Regex::new(r",(\s*[}\]])").unwrap();
    sanitized = re_trailing_comma.replace_all(&sanitized, "$1").to_string();

    let re_single_quote_key = regex::Regex::new(r"'([a-zA-Z_][a-zA-Z0-9_]*)'(\s*:)").unwrap();
    sanitized = re_single_quote_key
        .replace_all(&sanitized, "\"$1\"$2")
        .to_string();

    let re_single_quote_val = regex::Regex::new(r":\s*'([^']*)'").unwrap();
    sanitized = re_single_quote_val
        .replace_all(&sanitized, ": \"$1\"")
        .to_string();

    let re_unquoted_key = regex::Regex::new(r#"([,{]\s*)([a-zA-Z_][a-zA-Z0-9_]*)(\s*:)"#).unwrap();
    sanitized = re_unquoted_key
        .replace_all(&sanitized, "$1\"$2\"$3")
        .to_string();

    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recover_already_complete_json() {
        let complete = r#"{"entities":[],"relationships":[]}"#;
        assert!(try_recover_truncated_json(complete).is_some());
    }

    #[test]
    fn test_recover_truncated_after_complete_entity() {
        let truncated = r#"{"entities":[{"name":"A","type":"PERSON","description":"test"}"#;
        assert!(try_recover_truncated_json(truncated).is_some());
    }

    #[test]
    fn test_recover_truncated_entities_array_open() {
        let truncated = r#"{"entities":[{"name":"SEISMOLOGY","type":"CONCEPT","description":"Study of earthquakes"}],"relationships":["#;
        let result = try_recover_truncated_json(truncated).expect("should recover");
        let entities = result["entities"].as_array().expect("entities array");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0]["name"], "SEISMOLOGY");
    }

    #[test]
    fn test_recover_returns_none_for_unrecoverable_input() {
        assert!(try_recover_truncated_json("not json at all ///").is_none());
    }

    #[test]
    fn test_empty_on_missing_json_returns_empty_result() {
        let parser = JsonExtractionParser::new();
        let result = parser
            .parse_with_options(
                "no json here",
                "chunk-1",
                JsonParseOptions {
                    empty_on_missing_json: true,
                    ..Default::default()
                },
            )
            .expect("tolerant parse");
        assert!(result.entities.is_empty());
    }

    #[test]
    fn test_llm_options_normalizes_entity_names() {
        let parser = JsonExtractionParser::new();
        let schema = EntityExtractionSchema::server_default();
        let response = r#"{"entities":[{"name":"The Company","type":"ORG","description":"x"}],"relationships":[]}"#;
        let result = parser
            .parse_with_options(
                response,
                "c1",
                JsonParseOptions {
                    entity_schema: Some(&schema),
                    recover_truncated: true,
                    empty_on_missing_json: true,
                },
            )
            .expect("parse");
        assert_eq!(result.entities[0].name, "COMPANY");
    }
}
