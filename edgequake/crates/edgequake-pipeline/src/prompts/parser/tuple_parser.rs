//! Tuple-delimited extraction result parser (SOTA format).
//!
//! Parses extraction output in the format used by LightRAG:
//! ```text
//! entity<|#|>Name<|#|>TYPE<|#|>Description
//! relation<|#|>Source<|#|>Target<|#|>keywords<|#|>Description
//! <|COMPLETE|>
//! ```
//!
//! # WHY Tuple Format Over JSON
//!
//! The tuple-delimited format is significantly more robust for LLM outputs:
//!
//! 1. **Partial output recovery**: Valid lines parse independently even if
//!    the response is truncated.
//! 2. **No escaping issues**: No quotes, backslashes, or unicode escaping.
//! 3. **Line-by-line processing**: Enables streaming extraction.
//! 4. **Battle-tested**: Proven in the LightRAG paper with millions of extractions.

use super::super::normalizer::normalize_entity_name;
use super::super::{DEFAULT_COMPLETION_DELIMITER, DEFAULT_TUPLE_DELIMITER};
use crate::error::Result;
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

    /// Create a parser with custom delimiters.
    pub fn with_delimiters(tuple: &str, completion: &str) -> Self {
        Self {
            tuple_delimiter: tuple.to_string(),
            completion_delimiter: completion.to_string(),
        }
    }

    /// Parse a response into an extraction result.
    pub fn parse(&self, response: &str, chunk_id: &str) -> Result<ExtractionResult> {
        let mut result = ExtractionResult::new(chunk_id);
        let mut parse_errors = 0u64;

        // Check if the response is complete
        let is_complete = response.contains(&self.completion_delimiter);
        result
            .metadata
            .insert("is_complete".to_string(), serde_json::json!(is_complete));

        for line in response.lines() {
            let line = line.trim();
            if line.is_empty() || line == self.completion_delimiter {
                continue;
            }

            let parts: Vec<&str> = line.split(&self.tuple_delimiter).collect();

            if parts.is_empty() {
                continue;
            }

            match parts[0].trim().to_lowercase().as_str() {
                "entity" if parts.len() >= 4 => {
                    let raw_name = parts[1].trim();
                    let entity_type = parts[2].trim().to_uppercase();
                    let description = parts[3].trim();

                    // Skip entities with empty or whitespace-only names
                    if raw_name.is_empty() {
                        parse_errors += 1;
                        continue;
                    }

                    let normalized_name = normalize_entity_name(raw_name);

                    // BR0006 defense: Skip entities that normalize to empty string
                    if normalized_name.is_empty() {
                        tracing::debug!(raw_name = %raw_name, "Skipping entity with empty normalized name");
                        continue;
                    }

                    result.add_entity(ExtractedEntity::new(
                        normalized_name,
                        entity_type,
                        description,
                    ));
                }
                "relation" | "relationship" if parts.len() >= 5 => {
                    let source = parts[1].trim();
                    let target = parts[2].trim();
                    let keywords_str = parts[3].trim();
                    let description = parts[4].trim();

                    let normalized_source = normalize_entity_name(source);
                    let normalized_target = normalize_entity_name(target);

                    // BR0006: Self-referencing relationships forbidden
                    if normalized_source == normalized_target {
                        tracing::debug!(
                            source = %normalized_source,
                            "Skipping self-referencing relationship (BR0006)"
                        );
                        continue;
                    }

                    // Skip relationships with empty normalized endpoints
                    if normalized_source.is_empty() || normalized_target.is_empty() {
                        tracing::debug!(
                            raw_source = %source,
                            raw_target = %target,
                            "Skipping relationship with empty normalized endpoint"
                        );
                        continue;
                    }

                    // BR0004: Parse and limit keywords (max 5 per edge)
                    let keywords: Vec<String> = keywords_str
                        .split(',')
                        .map(|k| k.trim().to_string())
                        .filter(|k| !k.is_empty())
                        .take(5)
                        .collect();

                    let mut rel = ExtractedRelationship::new(
                        normalized_source,
                        normalized_target,
                        keywords_str,
                    )
                    .with_description(description);

                    if !keywords.is_empty() {
                        rel = rel.with_keywords(keywords);
                    }

                    result.add_relationship(rel);
                }
                _ => {
                    // Unknown line type, count as parse error
                    if line.contains(&self.tuple_delimiter) {
                        parse_errors += 1;
                    }
                }
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn delim() -> &'static str {
        DEFAULT_TUPLE_DELIMITER
    }

    fn comp() -> &'static str {
        DEFAULT_COMPLETION_DELIMITER
    }

    #[test]
    fn test_parse_single_entity() {
        let parser = TupleParser::new();
        let input = format!("entity{}Alice{}PERSON{}A researcher", delim(), delim(), delim());
        let result = parser.parse(&input, "c1").unwrap();
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].entity_type, "PERSON");
        assert!(result.entities[0].description.contains("A researcher"));
    }

    #[test]
    fn test_parse_single_relationship() {
        let parser = TupleParser::new();
        let input = format!(
            "relation{}Alice{}Bob{}collaboration,research{}They work together",
            delim(), delim(), delim(), delim()
        );
        let result = parser.parse(&input, "c1").unwrap();
        assert_eq!(result.relationships.len(), 1);
        assert!(result.relationships[0].description.contains("work together"));
    }

    #[test]
    fn test_parse_mixed_entities_and_relationships() {
        let parser = TupleParser::new();
        let input = format!(
            "entity{}Alice{}PERSON{}Researcher\n\
             entity{}Bob{}PERSON{}Engineer\n\
             relation{}Alice{}Bob{}collab{}They collaborate",
            delim(), delim(), delim(),
            delim(), delim(), delim(),
            delim(), delim(), delim(), delim()
        );
        let result = parser.parse(&input, "c1").unwrap();
        assert_eq!(result.entities.len(), 2);
        assert_eq!(result.relationships.len(), 1);
    }

    #[test]
    fn test_parse_empty_input() {
        let parser = TupleParser::new();
        let result = parser.parse("", "c1").unwrap();
        assert_eq!(result.entities.len(), 0);
        assert_eq!(result.relationships.len(), 0);
    }

    #[test]
    fn test_br0006_self_referencing_relationship_skipped() {
        let parser = TupleParser::new();
        let input = format!(
            "relation{}Alice{}Alice{}self{}Talks to herself",
            delim(), delim(), delim(), delim()
        );
        let result = parser.parse(&input, "c1").unwrap();
        assert_eq!(result.relationships.len(), 0, "Self-referencing relationship must be skipped");
    }

    #[test]
    fn test_empty_entity_name_skipped() {
        let parser = TupleParser::new();
        let input = format!("entity{}{}PERSON{}Description", delim(), delim(), delim());
        let result = parser.parse(&input, "c1").unwrap();
        assert_eq!(result.entities.len(), 0);
        let errors = result.metadata.get("parse_errors").unwrap().as_u64().unwrap();
        assert_eq!(errors, 1, "Empty name should count as parse error");
    }

    #[test]
    fn test_unknown_line_type_counts_parse_error() {
        let parser = TupleParser::new();
        let input = format!("foobar{}x{}y{}z", delim(), delim(), delim());
        let result = parser.parse(&input, "c1").unwrap();
        assert_eq!(result.entities.len(), 0);
        assert_eq!(result.relationships.len(), 0);
        let errors = result.metadata.get("parse_errors").unwrap().as_u64().unwrap();
        assert!(errors >= 1, "Unknown line with delimiters should be a parse error");
    }

    #[test]
    fn test_keyword_limit_five() {
        let parser = TupleParser::new();
        let input = format!(
            "relation{}Alice{}Bob{}a,b,c,d,e,f,g{}Desc",
            delim(), delim(), delim(), delim()
        );
        let result = parser.parse(&input, "c1").unwrap();
        assert_eq!(result.relationships.len(), 1);
        let kw = &result.relationships[0].keywords;
        assert!(kw.len() <= 5, "Keywords must be limited to 5, got {}", kw.len());
    }

    #[test]
    fn test_is_complete_true() {
        let parser = TupleParser::new();
        let input = format!("entity{}X{}TYPE{}Desc\n{}", delim(), delim(), delim(), comp());
        assert!(parser.is_complete(&input));
    }

    #[test]
    fn test_is_complete_false() {
        let parser = TupleParser::new();
        let input = format!("entity{}X{}TYPE{}Desc", delim(), delim(), delim());
        assert!(!parser.is_complete(&input));
    }

    #[test]
    fn test_is_complete_metadata() {
        let parser = TupleParser::new();
        let input = format!("entity{}X{}TYPE{}Desc\n{}", delim(), delim(), delim(), comp());
        let result = parser.parse(&input, "c1").unwrap();
        let is_complete = result.metadata.get("is_complete").unwrap().as_bool().unwrap();
        assert!(is_complete);
    }

    #[test]
    fn test_with_delimiters_custom() {
        let parser = TupleParser::with_delimiters("||", "<<DONE>>");
        let input = "entity||Alice||PERSON||Researcher\n<<DONE>>";
        let result = parser.parse(input, "c1").unwrap();
        assert_eq!(result.entities.len(), 1);
        assert!(parser.is_complete(input));
    }

    #[test]
    fn test_relationship_keyword_also_accepted() {
        // "relationship" (not just "relation") should parse
        let parser = TupleParser::new();
        let input = format!(
            "relationship{}Alice{}Bob{}kw{}Desc",
            delim(), delim(), delim(), delim()
        );
        let result = parser.parse(&input, "c1").unwrap();
        assert_eq!(result.relationships.len(), 1);
    }

    #[test]
    fn test_parser_metadata_tuple() {
        let parser = TupleParser::new();
        let result = parser.parse("", "c1").unwrap();
        let parser_name = result.metadata.get("parser").unwrap().as_str().unwrap();
        assert_eq!(parser_name, "tuple");
    }
}
