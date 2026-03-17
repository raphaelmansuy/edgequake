//! Resolve DocumentFilter into a list of matching document IDs.
//!
//! @implements SPEC-005: Document date and pattern filters (Tier 1 — KV scan)
//!
//! Scans KV storage for document metadata keys (`{doc_id}-metadata`), extracts
//! `created_at` and `title` fields, and returns the subset of document IDs that
//! match the filter criteria (date range + title pattern).

use edgequake_storage::traits::KVStorage;
use tracing::{debug, warn};

use crate::handlers::query_types::DocumentFilter;

/// Resolve a `DocumentFilter` into a list of matching document IDs.
///
/// Returns `None` if no filter fields are set (all-pass), or `Some(vec)` with
/// the matching IDs. An empty `Some(vec![])` means nothing matched — the caller
/// should short-circuit with an empty result.
///
/// # Filter logic
///
/// - `date_from`: Documents with `created_at >= date_from` (ISO 8601 string compare)
/// - `date_to`: Documents with `created_at <= date_to` (ISO 8601 string compare)
/// - `document_pattern`: Case-insensitive substring match on `title`.
///   Comma-separated values are OR-ed (e.g., "report,summary" matches either).
///
/// All active criteria are AND-ed together.
pub async fn resolve_document_filter(
    kv_storage: &dyn KVStorage,
    filter: &DocumentFilter,
    tenant_id: &Option<String>,
    workspace_id: &Option<String>,
) -> Result<Option<Vec<String>>, crate::error::ApiError> {
    // If all filter fields are None, return None (no filtering)
    if filter.date_from.is_none() && filter.date_to.is_none() && filter.document_pattern.is_none() {
        return Ok(None);
    }

    // Fetch all KV keys and find metadata keys
    let keys = kv_storage
        .keys()
        .await
        .map_err(|e| crate::error::ApiError::Internal(format!("Failed to list KV keys: {}", e)))?;

    let metadata_keys: Vec<String> = keys
        .into_iter()
        .filter(|k| k.ends_with("-metadata"))
        .collect();

    if metadata_keys.is_empty() {
        debug!("No metadata keys found — filter returns empty set");
        return Ok(Some(Vec::new()));
    }

    // Batch-fetch all metadata values
    let metadata_values = kv_storage.get_by_ids(&metadata_keys).await.map_err(|e| {
        crate::error::ApiError::Internal(format!("Failed to fetch document metadata: {}", e))
    })?;

    // Pre-parse the pattern into lowercase substrings for matching
    let patterns: Vec<String> = filter
        .document_pattern
        .as_ref()
        .map(|p| {
            p.split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let mut matched_ids = Vec::new();

    for value in &metadata_values {
        let obj = match value.as_object() {
            Some(o) => o,
            None => continue,
        };

        let doc_id = match obj.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };

        // Tenant/workspace scoping — skip docs that don't belong to the caller
        if let Some(ref tid) = tenant_id {
            if let Some(doc_tid) = obj.get("tenant_id").and_then(|v| v.as_str()) {
                if doc_tid != tid {
                    continue;
                }
            }
        }
        if let Some(ref wid) = workspace_id {
            if let Some(doc_wid) = obj.get("workspace_id").and_then(|v| v.as_str()) {
                if doc_wid != wid {
                    continue;
                }
            }
        }

        // Date range filter (ISO 8601 string comparison)
        let created_at = obj.get("created_at").and_then(|v| v.as_str());

        if let Some(ref date_from) = filter.date_from {
            match created_at {
                Some(ca) if ca >= date_from.as_str() => {}
                Some(_) => continue, // created_at < date_from
                None => {
                    // No created_at — skip this document (can't prove it's in range)
                    warn!(document_id = %doc_id, "No created_at field — excluded by date_from filter");
                    continue;
                }
            }
        }

        if let Some(ref date_to) = filter.date_to {
            match created_at {
                Some(ca) if ca <= date_to.as_str() => {}
                Some(_) => continue, // created_at > date_to
                None => {
                    warn!(document_id = %doc_id, "No created_at field — excluded by date_to filter");
                    continue;
                }
            }
        }

        // Title pattern filter (case-insensitive, comma-separated OR)
        if !patterns.is_empty() {
            let title = obj
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let matches = patterns.iter().any(|p| title.contains(p.as_str()));
            if !matches {
                continue;
            }
        }

        matched_ids.push(doc_id.to_string());
    }

    debug!(
        filter = ?filter,
        matched_count = matched_ids.len(),
        "Document filter resolved"
    );

    Ok(Some(matched_ids))
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::adapters::memory::MemoryKVStorage;
    use serde_json::json;

    async fn setup_kv_with_docs(docs: Vec<serde_json::Value>) -> MemoryKVStorage {
        let kv = MemoryKVStorage::new("test");
        kv.initialize().await.unwrap();
        for doc in &docs {
            let id = doc.get("id").unwrap().as_str().unwrap();
            let key = format!("{}-metadata", id);
            kv.upsert(&[(key, doc.clone())]).await.unwrap();
        }
        kv
    }

    #[tokio::test]
    async fn test_all_none_returns_none() {
        let kv = setup_kv_with_docs(vec![]).await;
        let filter = DocumentFilter {
            date_from: None,
            date_to: None,
            document_pattern: None,
        };
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap();
        assert!(
            result.is_none(),
            "All-None filter should return None (no-op)"
        );
    }

    #[tokio::test]
    async fn test_date_from_filter() {
        let kv = setup_kv_with_docs(vec![
            json!({"id": "doc1", "title": "Alpha", "created_at": "2025-01-15T00:00:00Z"}),
            json!({"id": "doc2", "title": "Beta", "created_at": "2025-06-01T00:00:00Z"}),
            json!({"id": "doc3", "title": "Gamma", "created_at": "2024-12-01T00:00:00Z"}),
        ])
        .await;

        let filter = DocumentFilter {
            date_from: Some("2025-01-01T00:00:00Z".to_string()),
            date_to: None,
            document_pattern: None,
        };
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap()
            .unwrap();

        assert!(result.contains(&"doc1".to_string()));
        assert!(result.contains(&"doc2".to_string()));
        assert!(
            !result.contains(&"doc3".to_string()),
            "doc3 is before date_from"
        );
    }

    #[tokio::test]
    async fn test_date_range_filter() {
        let kv = setup_kv_with_docs(vec![
            json!({"id": "doc1", "title": "Alpha", "created_at": "2025-01-15T00:00:00Z"}),
            json!({"id": "doc2", "title": "Beta", "created_at": "2025-06-01T00:00:00Z"}),
            json!({"id": "doc3", "title": "Gamma", "created_at": "2025-03-01T00:00:00Z"}),
        ])
        .await;

        let filter = DocumentFilter {
            date_from: Some("2025-02-01T00:00:00Z".to_string()),
            date_to: Some("2025-04-30T23:59:59Z".to_string()),
            document_pattern: None,
        };
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(result, vec!["doc3".to_string()]);
    }

    #[tokio::test]
    async fn test_pattern_filter_case_insensitive() {
        let kv = setup_kv_with_docs(vec![
            json!({"id": "doc1", "title": "Annual Report 2025"}),
            json!({"id": "doc2", "title": "Technical Summary"}),
            json!({"id": "doc3", "title": "Budget Forecast"}),
        ])
        .await;

        let filter = DocumentFilter {
            date_from: None,
            date_to: None,
            document_pattern: Some("report".to_string()),
        };
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(result, vec!["doc1".to_string()]);
    }

    #[tokio::test]
    async fn test_pattern_filter_comma_separated_or() {
        let kv = setup_kv_with_docs(vec![
            json!({"id": "doc1", "title": "Annual Report 2025"}),
            json!({"id": "doc2", "title": "Technical Summary"}),
            json!({"id": "doc3", "title": "Budget Forecast"}),
        ])
        .await;

        let filter = DocumentFilter {
            date_from: None,
            date_to: None,
            document_pattern: Some("report, summary".to_string()),
        };
        let mut result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap()
            .unwrap();
        result.sort();

        assert_eq!(result, vec!["doc1".to_string(), "doc2".to_string()]);
    }

    #[tokio::test]
    async fn test_combined_date_and_pattern() {
        let kv = setup_kv_with_docs(vec![
            json!({"id": "doc1", "title": "Annual Report", "created_at": "2025-01-15T00:00:00Z"}),
            json!({"id": "doc2", "title": "Annual Report", "created_at": "2024-06-01T00:00:00Z"}),
            json!({"id": "doc3", "title": "Budget Forecast", "created_at": "2025-03-01T00:00:00Z"}),
        ])
        .await;

        let filter = DocumentFilter {
            date_from: Some("2025-01-01T00:00:00Z".to_string()),
            date_to: None,
            document_pattern: Some("report".to_string()),
        };
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            result,
            vec!["doc1".to_string()],
            "Only doc1 matches both date AND pattern"
        );
    }

    #[tokio::test]
    async fn test_tenant_scoping() {
        let kv = setup_kv_with_docs(vec![
            json!({"id": "doc1", "title": "Alpha", "tenant_id": "t1"}),
            json!({"id": "doc2", "title": "Beta", "tenant_id": "t2"}),
        ])
        .await;

        let filter = DocumentFilter {
            date_from: None,
            date_to: None,
            document_pattern: Some("alpha, beta".to_string()),
        };
        let result = resolve_document_filter(&kv, &filter, &Some("t1".to_string()), &None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            result,
            vec!["doc1".to_string()],
            "Only tenant t1 docs returned"
        );
    }

    #[tokio::test]
    async fn test_no_matching_documents() {
        let kv = setup_kv_with_docs(vec![
            json!({"id": "doc1", "title": "Alpha", "created_at": "2025-01-15T00:00:00Z"}),
        ])
        .await;

        let filter = DocumentFilter {
            date_from: Some("2026-01-01T00:00:00Z".to_string()),
            date_to: None,
            document_pattern: None,
        };
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap()
            .unwrap();

        assert!(result.is_empty(), "No documents should match future date");
    }
}
