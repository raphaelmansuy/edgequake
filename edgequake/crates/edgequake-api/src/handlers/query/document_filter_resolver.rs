//! Resolve DocumentFilter into a list of matching document IDs.
//!
//! @implements SPEC-005: Document date and pattern filters (Tier 1 — KV scan)
//! @implements SPEC-031: Explicit document scope selection (fast path — no KV scan)
//!
//! ## Resolution strategy
//!
//! 1. `filter.is_empty()` → `None` (all-pass, no work done)
//! 2. Only `document_ids` set, no date/pattern → return IDs directly (no KV scan)
//! 3. Date and/or pattern set → KV scan; union with any explicit `document_ids`
//!
//! Date filter is AND-combined with membership (ids OR pattern).

use edgequake_storage::traits::KVStorage;
use tracing::{debug, warn};

use crate::handlers::query_types::DocumentFilter;
use crate::middleware::TenantContext;
use crate::services::document_metadata_scan::load_scoped_document_metadata_entries;

/// Resolve a `DocumentFilter` into a list of matching document IDs.
///
/// Returns `None` if no filter fields are set (all-pass), or `Some(vec)` with
/// the matching IDs. An empty `Some(vec![])` means nothing matched — the caller
/// should short-circuit with an empty result.
///
/// @implements SPEC-005 @implements SPEC-031
pub async fn resolve_document_filter(
    kv_storage: &dyn KVStorage,
    filter: &DocumentFilter,
    tenant_id: &Option<String>,
    workspace_id: &Option<String>,
) -> Result<Option<Vec<String>>, crate::error::ApiError> {
    // Fast path: no filter criteria at all
    if filter.is_empty() {
        return Ok(None);
    }

    let has_explicit_ids = filter
        .document_ids
        .as_ref()
        .is_some_and(|ids| !ids.is_empty());
    let has_pattern = filter.document_pattern.is_some();
    let has_date_filter = filter.date_from.is_some() || filter.date_to.is_some();

    // SPEC-031 fast path: explicit IDs only, no date/pattern → skip KV scan entirely
    if has_explicit_ids && !has_pattern && !has_date_filter {
        let ids = deduplicate(filter.document_ids.as_ref().unwrap().clone());
        debug!(
            id_count = ids.len(),
            "Document filter: explicit IDs only — skipping KV scan"
        );
        return Ok(Some(ids));
    }

    // KV scan required (date filter, pattern, or date + explicit IDs combined)
    let tenant_ctx = TenantContext {
        tenant_id: tenant_id.clone(),
        workspace_id: workspace_id.clone(),
        user_id: None,
    };
    let metadata_values: Vec<serde_json::Value> =
        load_scoped_document_metadata_entries(kv_storage, &tenant_ctx)
            .await?
            .into_iter()
            .map(|(_, v)| v)
            .collect();

    if metadata_values.is_empty() {
        debug!("No metadata keys found — filter returns empty set");
        return Ok(Some(Vec::new()));
    }

    let patterns = parse_patterns(filter.document_pattern.as_deref());

    // Explicit ID set for O(1) membership checks
    let explicit_id_set: std::collections::HashSet<&str> = filter
        .document_ids
        .as_ref()
        .map(|ids| ids.iter().map(String::as_str).collect())
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

        // Date range is AND-applied across all candidates
        if !passes_date_filter(obj, &filter.date_from, &filter.date_to, doc_id) {
            continue;
        }

        // Membership: explicit ID OR pattern match
        // If neither IDs nor pattern → date-only filter (all pass date check)
        let passes_membership = if has_explicit_ids || has_pattern {
            let in_explicit = explicit_id_set.contains(doc_id);
            let in_pattern = !patterns.is_empty() && {
                let title = obj
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                patterns.iter().any(|p| title.contains(p.as_str()))
            };
            in_explicit || in_pattern
        } else {
            true
        };

        if passes_membership {
            matched_ids.push(doc_id.to_string());
        }
    }

    debug!(
        has_explicit_ids,
        has_pattern,
        has_date_filter,
        matched_count = matched_ids.len(),
        "Document filter resolved"
    );

    Ok(Some(matched_ids))
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Remove duplicate IDs while preserving order.
fn deduplicate(ids: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    ids.into_iter()
        .filter(|id| seen.insert(id.clone()))
        .collect()
}

/// Parse a comma-separated pattern string into lowercase substrings.
fn parse_patterns(pattern: Option<&str>) -> Vec<String> {
    pattern
        .map(|p| {
            p.split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Check whether a document passes the date range filter.
/// Returns `true` if both date boundaries are satisfied (or not set).
fn passes_date_filter(
    obj: &serde_json::Map<String, serde_json::Value>,
    date_from: &Option<String>,
    date_to: &Option<String>,
    doc_id: &str,
) -> bool {
    let created_at = obj.get("created_at").and_then(|v| v.as_str());

    if let Some(ref df) = date_from {
        match created_at {
            Some(ca) if ca >= df.as_str() => {}
            Some(_) => return false,
            None => {
                warn!(document_id = %doc_id, "No created_at field — excluded by date_from filter");
                return false;
            }
        }
    }

    if let Some(ref dt) = date_to {
        match created_at {
            Some(ca) if ca <= dt.as_str() => {}
            Some(_) => return false,
            None => {
                warn!(document_id = %doc_id, "No created_at field — excluded by date_to filter");
                return false;
            }
        }
    }

    true
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::adapters::memory::MemoryKVStorage;
    use edgequake_storage::kv_keys;
    use serde_json::json;

    async fn setup_kv_with_docs(docs: Vec<serde_json::Value>) -> MemoryKVStorage {
        let kv = MemoryKVStorage::new("test");
        kv.initialize().await.unwrap();
        for doc in &docs {
            let id = doc.get("id").unwrap().as_str().unwrap();
            let key = kv_keys::doc_metadata(id);
            kv.upsert(&[(key, doc.clone())]).await.unwrap();
        }
        kv
    }

    fn filter_ids_only(ids: Vec<&str>) -> DocumentFilter {
        DocumentFilter {
            document_ids: Some(ids.into_iter().map(String::from).collect()),
            ..Default::default()
        }
    }

    // ── SPEC-005 (existing) ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_all_none_returns_none() {
        let kv = setup_kv_with_docs(vec![]).await;
        let filter = DocumentFilter::default();
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap();
        assert!(result.is_none(), "Empty filter must return None");
    }

    #[tokio::test]
    async fn test_date_from_filter() {
        let kv = setup_kv_with_docs(vec![
            json!({"id": "doc1", "title": "Alpha", "created_at": "2025-01-15T00:00:00Z"}),
            json!({"id": "doc2", "title": "Beta",  "created_at": "2025-06-01T00:00:00Z"}),
            json!({"id": "doc3", "title": "Gamma", "created_at": "2024-12-01T00:00:00Z"}),
        ])
        .await;

        let filter = DocumentFilter {
            date_from: Some("2025-01-01T00:00:00Z".to_string()),
            ..Default::default()
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
            json!({"id": "doc2", "title": "Beta",  "created_at": "2025-06-01T00:00:00Z"}),
            json!({"id": "doc3", "title": "Gamma", "created_at": "2025-03-01T00:00:00Z"}),
        ])
        .await;

        let filter = DocumentFilter {
            date_from: Some("2025-02-01T00:00:00Z".to_string()),
            date_to: Some("2025-04-30T23:59:59Z".to_string()),
            ..Default::default()
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
            document_pattern: Some("report".to_string()),
            ..Default::default()
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
            document_pattern: Some("report, summary".to_string()),
            ..Default::default()
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
            json!({"id": "doc3", "title": "Budget Forecast","created_at": "2025-03-01T00:00:00Z"}),
        ])
        .await;

        let filter = DocumentFilter {
            date_from: Some("2025-01-01T00:00:00Z".to_string()),
            document_pattern: Some("report".to_string()),
            ..Default::default()
        };
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(result, vec!["doc1".to_string()]);
    }

    #[tokio::test]
    async fn test_no_matching_documents() {
        let kv = setup_kv_with_docs(vec![
            json!({"id": "doc1", "title": "Alpha", "created_at": "2025-01-15T00:00:00Z"}),
        ])
        .await;

        let filter = DocumentFilter {
            date_from: Some("2026-01-01T00:00:00Z".to_string()),
            ..Default::default()
        };
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap()
            .unwrap();

        assert!(result.is_empty(), "No documents should match future date");
    }

    // ── SPEC-031 (new) ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_empty_document_ids_treated_as_noop() {
        let kv = setup_kv_with_docs(vec![]).await;
        let filter = DocumentFilter {
            document_ids: Some(vec![]),
            ..Default::default()
        };
        // Empty document_ids → is_empty() → None (no filtering)
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap();
        assert!(result.is_none(), "Empty document_ids must be a no-op");
    }

    #[tokio::test]
    async fn test_explicit_ids_only_returns_directly_no_kv_scan() {
        // KV is empty — if KV is consulted the resolver would return Some(vec![])
        // but since we only have explicit IDs, it should return them directly
        // without scanning KV (even though KV has nothing).
        // We verify by checking the returned IDs match what was provided.
        let kv = setup_kv_with_docs(vec![]).await;
        let filter = filter_ids_only(vec!["abc", "def"]);
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap()
            .unwrap();
        let mut result = result;
        result.sort();
        assert_eq!(result, vec!["abc".to_string(), "def".to_string()]);
    }

    #[tokio::test]
    async fn test_explicit_ids_deduplicated() {
        let kv = setup_kv_with_docs(vec![]).await;
        let filter = DocumentFilter {
            document_ids: Some(vec!["a".to_string(), "b".to_string(), "a".to_string()]),
            ..Default::default()
        };
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.len(), 2, "Duplicates must be removed");
    }

    #[tokio::test]
    async fn test_explicit_ids_with_date_filter_scans_kv() {
        let kv = setup_kv_with_docs(vec![
            json!({"id": "doc1", "title": "A", "created_at": "2025-06-01T00:00:00Z"}),
            json!({"id": "doc2", "title": "B", "created_at": "2024-01-01T00:00:00Z"}),
        ])
        .await;

        // doc1 passes date, doc2 doesn't; both are in explicit IDs
        let filter = DocumentFilter {
            document_ids: Some(vec!["doc1".to_string(), "doc2".to_string()]),
            date_from: Some("2025-01-01T00:00:00Z".to_string()),
            ..Default::default()
        };
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result, vec!["doc1".to_string()]);
    }

    #[tokio::test]
    async fn test_explicit_ids_union_with_pattern() {
        let kv = setup_kv_with_docs(vec![
            json!({"id": "doc1", "title": "Alpha Report"}),
            json!({"id": "doc2", "title": "Beta Summary"}),
            json!({"id": "doc3", "title": "Gamma Forecast"}),
        ])
        .await;

        // doc1 in explicit IDs, doc2 matches pattern — both should be included
        let filter = DocumentFilter {
            document_ids: Some(vec!["doc1".to_string()]),
            document_pattern: Some("summary".to_string()),
            ..Default::default()
        };
        let mut result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap()
            .unwrap();
        result.sort();
        assert_eq!(result, vec!["doc1".to_string(), "doc2".to_string()]);
    }

    #[tokio::test]
    async fn test_nonexistent_explicit_ids_silently_ignored() {
        let kv = setup_kv_with_docs(vec![json!({"id": "real-doc", "title": "Real"})]).await;

        // "phantom-id" does not exist in KV — should be silently ignored
        let filter = DocumentFilter {
            document_ids: Some(vec!["phantom-id".to_string()]),
            document_pattern: Some("real".to_string()),
            ..Default::default()
        };
        let result = resolve_document_filter(&kv, &filter, &None, &None)
            .await
            .unwrap()
            .unwrap();
        // "real-doc" matches pattern; "phantom-id" is not in KV
        assert_eq!(result, vec!["real-doc".to_string()]);
    }

    #[tokio::test]
    async fn test_tenant_scoping() {
        let kv = setup_kv_with_docs(vec![
            json!({"id": "doc1", "title": "Alpha", "tenant_id": "t1"}),
            json!({"id": "doc2", "title": "Beta",  "tenant_id": "t2"}),
        ])
        .await;

        let filter = DocumentFilter {
            document_pattern: Some("alpha, beta".to_string()),
            ..Default::default()
        };
        let result = resolve_document_filter(&kv, &filter, &Some("t1".to_string()), &None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result, vec!["doc1".to_string()]);
    }
}
