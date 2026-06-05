//! Resolve query scope by matching query topic against document enrichment topics.
//!
//! When `enable_topic_scope` is set on a query, this module:
//! 1. Scans KV for all `{doc_id}-metadata` keys in the workspace
//! 2. Collects unique `enrichment_topic` values from completed enrichments
//! 3. Uses the LLM to classify which topics are relevant to the query
//! 4. Returns document IDs belonging to those matched topics
//!
//! Falls back gracefully (returns `None`) when:
//! - Fewer than 2 distinct topics exist (no useful filtering)
//! - No documents have completed enrichment
//! - LLM classification fails (logs warning, search proceeds unscoped)

use std::collections::HashMap;

use edgequake_llm::traits::LLMProvider;
use edgequake_storage::traits::KVStorage;
use tracing::{debug, warn};

pub struct TopicResolveResult {
    pub matched_topics: Vec<String>,
    pub document_ids: Vec<String>,
}

/// Resolve which document IDs are relevant to the query based on enrichment topics.
///
/// Returns `None` if topic scoping is not applicable (too few topics, no enrichment data).
/// Returns `Some(result)` with matched topics and their document IDs.
pub async fn resolve_topic_scope(
    kv_storage: &dyn KVStorage,
    llm_provider: &dyn LLMProvider,
    query: &str,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<Option<TopicResolveResult>, crate::error::ApiError> {
    // Collect topic → [doc_id] map from KV enrichment metadata
    let topic_map = build_topic_map(kv_storage, tenant_id, workspace_id).await?;

    if topic_map.len() < 2 {
        debug!(
            topic_count = topic_map.len(),
            "Too few topics for scoping — skipping topic filter"
        );
        return Ok(None);
    }

    let topics: Vec<&str> = topic_map.keys().map(|s| s.as_str()).collect();

    debug!(
        topic_count = topics.len(),
        topics = ?topics,
        "Classifying query against available topics"
    );

    let matched_topics = classify_query_topics(llm_provider, query, &topics).await;

    if matched_topics.is_empty() {
        debug!("LLM found no matching topics — query will search all documents");
        return Ok(None);
    }

    let document_ids: Vec<String> = matched_topics
        .iter()
        .filter_map(|t| topic_map.get(t.as_str()))
        .flat_map(|ids| ids.iter().cloned())
        .collect();

    if document_ids.is_empty() {
        return Ok(None);
    }

    debug!(
        matched_topics = ?matched_topics,
        document_count = document_ids.len(),
        "Topic scope resolved"
    );

    Ok(Some(TopicResolveResult {
        matched_topics,
        document_ids,
    }))
}

/// Resolve document IDs whose `enrichment_tags` contain ANY of the requested tags (case-insensitive).
///
/// Returns `None` when no documents match (caller should proceed unscoped).
pub async fn resolve_tags_filter(
    kv_storage: &dyn KVStorage,
    tags: &[String],
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<Option<Vec<String>>, crate::error::ApiError> {
    if tags.is_empty() {
        return Ok(None);
    }

    let tags_lower: Vec<String> = tags.iter().map(|t| t.to_lowercase()).collect();

    let keys = kv_storage
        .keys()
        .await
        .map_err(|e| crate::error::ApiError::Internal(format!("KV keys scan failed: {}", e)))?;

    let metadata_keys: Vec<String> = keys.into_iter().filter(|k| k.ends_with("-metadata")).collect();
    if metadata_keys.is_empty() {
        return Ok(None);
    }

    let metadata_values = kv_storage
        .get_by_ids(&metadata_keys)
        .await
        .map_err(|e| crate::error::ApiError::Internal(format!("KV batch fetch failed: {}", e)))?;

    let mut doc_ids: Vec<String> = Vec::new();

    for value in &metadata_values {
        let obj = match value.as_object() {
            Some(o) => o,
            None => continue,
        };

        if let Some(tid) = tenant_id {
            if obj.get("tenant_id").and_then(|v| v.as_str()) != Some(tid) {
                continue;
            }
        }
        if let Some(wid) = workspace_id {
            if obj.get("workspace_id").and_then(|v| v.as_str()) != Some(wid) {
                continue;
            }
        }

        if obj.get("enrichment_status").and_then(|v| v.as_str()) != Some("completed") {
            continue;
        }

        let doc_enrichment_tags: Vec<String> = obj
            .get("enrichment_tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                    .collect()
            })
            .unwrap_or_default();

        // Include document if it has ANY of the requested tags
        let matches = tags_lower.iter().any(|t| doc_enrichment_tags.contains(t));
        if matches {
            if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                doc_ids.push(id.to_string());
            }
        }
    }

    if doc_ids.is_empty() {
        debug!(tags = ?tags, "No documents found for tag filter");
        return Ok(None);
    }

    debug!(
        tags = ?tags,
        document_count = doc_ids.len(),
        "Tags filter resolved"
    );
    Ok(Some(doc_ids))
}

/// Resolve document IDs whose `enrichment_topic` matches `topic` (case-insensitive).
///
/// Returns `None` when no documents match (caller should proceed unscoped).
pub async fn resolve_topic_filter(
    kv_storage: &dyn KVStorage,
    topic: &str,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<Option<Vec<String>>, crate::error::ApiError> {
    let topic_map = build_topic_map(kv_storage, tenant_id, workspace_id).await?;
    let topic_lower = topic.to_lowercase();

    let doc_ids: Vec<String> = topic_map
        .iter()
        .filter(|(k, _)| k.to_lowercase() == topic_lower)
        .flat_map(|(_, ids)| ids.iter().cloned())
        .collect();

    if doc_ids.is_empty() {
        debug!(topic = %topic, "No documents found for explicit topic filter");
        return Ok(None);
    }

    debug!(
        topic = %topic,
        document_count = doc_ids.len(),
        "Explicit topic filter resolved"
    );
    Ok(Some(doc_ids))
}

/// Scan KV storage and build a `HashMap<topic, Vec<doc_id>>`.
async fn build_topic_map(
    kv_storage: &dyn KVStorage,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<HashMap<String, Vec<String>>, crate::error::ApiError> {
    let keys = kv_storage
        .keys()
        .await
        .map_err(|e| crate::error::ApiError::Internal(format!("KV keys scan failed: {}", e)))?;

    let metadata_keys: Vec<String> = keys
        .into_iter()
        .filter(|k| k.ends_with("-metadata"))
        .collect();

    if metadata_keys.is_empty() {
        return Ok(HashMap::new());
    }

    let metadata_values = kv_storage
        .get_by_ids(&metadata_keys)
        .await
        .map_err(|e| crate::error::ApiError::Internal(format!("KV batch fetch failed: {}", e)))?;

    let mut topic_map: HashMap<String, Vec<String>> = HashMap::new();

    for value in &metadata_values {
        let obj = match value.as_object() {
            Some(o) => o,
            None => continue,
        };

        // Scope to matching tenant/workspace if provided
        if let Some(tid) = tenant_id {
            if obj.get("tenant_id").and_then(|v| v.as_str()) != Some(tid) {
                continue;
            }
        }
        if let Some(wid) = workspace_id {
            if obj.get("workspace_id").and_then(|v| v.as_str()) != Some(wid) {
                continue;
            }
        }

        // Only include documents with completed enrichment
        if obj.get("enrichment_status").and_then(|v| v.as_str()) != Some("completed") {
            continue;
        }

        let topic = match obj
            .get("enrichment_topic")
            .and_then(|v| v.as_str())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            Some(t) => t.to_string(),
            None => continue,
        };

        let doc_id = match obj.get("id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };

        topic_map.entry(topic).or_default().push(doc_id);
    }

    Ok(topic_map)
}

const TOPIC_CLASSIFY_PROMPT: &str = r#"You are a document topic classifier.

Given a user query and a numbered list of available document topics, identify which topics are likely to contain relevant information to answer the query.

Available topics:
{topics}

User query: {query}

Return ONLY a JSON array of exact topic names from the list above. Example: ["Topic A", "Topic B"]
Rules:
- Include all topics that are likely relevant (cast wide when uncertain)
- If the query spans multiple topics, include all of them
- If no topic is relevant, return []
- Do NOT invent new topics, only use names from the list above
- Return raw JSON, no markdown, no explanation"#;

/// Ask the LLM which topics from the list match the query.
/// On any error returns an empty vec (fail-open: no scoping).
async fn classify_query_topics(
    llm_provider: &dyn LLMProvider,
    query: &str,
    topics: &[&str],
) -> Vec<String> {
    let topics_str = topics
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{}. {}", i + 1, t))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = TOPIC_CLASSIFY_PROMPT
        .replace("{topics}", &topics_str)
        .replace("{query}", query);

    let response = match llm_provider.complete(&prompt).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Topic classification LLM call failed — proceeding without scope");
            return Vec::new();
        }
    };

    parse_topics_response(&response.content, topics)
}

/// Parse the LLM JSON response, keeping only topics that exist in the original list.
fn parse_topics_response(content: &str, valid_topics: &[&str]) -> Vec<String> {
    // Strip optional ```json fences
    let trimmed = content.trim();
    let json_str = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: Vec<String> = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            warn!(
                raw = %content,
                error = %e,
                "Failed to parse topic classification response"
            );
            return Vec::new();
        }
    };

    // Only keep topics that exist in the original list (case-insensitive guard)
    let valid_lower: Vec<String> = valid_topics.iter().map(|t| t.to_lowercase()).collect();
    parsed
        .into_iter()
        .filter(|t| {
            let lower = t.to_lowercase();
            valid_lower.contains(&lower)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_topics_response_valid() {
        let valid = &["Machine Learning", "Financial Reports", "Legal"];
        let result = parse_topics_response(r#"["Machine Learning", "Legal"]"#, valid);
        assert_eq!(result, vec!["Machine Learning", "Legal"]);
    }

    #[test]
    fn test_parse_topics_response_strips_fences() {
        let valid = &["Finance", "Tech"];
        let result = parse_topics_response("```json\n[\"Finance\"]\n```", valid);
        assert_eq!(result, vec!["Finance"]);
    }

    #[test]
    fn test_parse_topics_response_rejects_unknown() {
        let valid = &["Finance", "Tech"];
        let result = parse_topics_response(r#"["Finance", "Unknown Topic"]"#, valid);
        assert_eq!(result, vec!["Finance"]);
    }

    #[test]
    fn test_parse_topics_response_empty() {
        let valid = &["Finance", "Tech"];
        let result = parse_topics_response("[]", valid);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_topics_response_invalid_json() {
        let valid = &["Finance"];
        let result = parse_topics_response("not json at all", valid);
        assert!(result.is_empty());
    }
}
