//! Document metadata KV scan — SPEC-027 IMP-029 (DRY SSOT).
//!
//! Indexed suffix scan + workspace doc index (`wsdoc:` prefix) for tenant isolation.

use edgequake_storage::traits::KVStorage;

use crate::error::ApiResult;
use crate::middleware::TenantContext;
use crate::services::workspace_document_index::{
    list_workspace_metadata_keys, list_workspace_metadata_keys_limited,
};
use crate::workspace_scope::metadata_matches_tenant_context;

pub use edgequake_storage::document_metadata_integrity::{
    canonical_document_id, document_id_from_metadata_key, DOCUMENT_METADATA_SUFFIX,
};

/// Metadata KV key for a document (DRY — delegates to `kv_keys::doc_metadata`).
#[inline]
pub fn metadata_key_for_document(document_id: &str) -> String {
    edgequake_storage::kv_keys::doc_metadata(document_id)
}

/// Load metadata JSON values for a workspace via wsdoc index with suffix-scan fallback.
pub async fn load_workspace_metadata_values(
    kv_storage: &(dyn KVStorage + Send + Sync),
    workspace_id: &str,
) -> ApiResult<Vec<serde_json::Value>> {
    let indexed = load_workspace_metadata_entries_by_index(kv_storage, workspace_id).await?;
    if !indexed.is_empty() {
        return Ok(indexed.into_iter().map(|(_, v)| v).collect());
    }

    Ok(load_all_document_metadata(kv_storage)
        .await?
        .into_iter()
        .filter(|v| {
            v.get("workspace_id")
                .and_then(|w| w.as_str())
                .map(|ws| ws == workspace_id)
                .unwrap_or(workspace_id == "default")
        })
        .collect())
}

/// Load all `(key, metadata)` pairs via indexed suffix scan (unscoped).
pub async fn load_all_document_metadata_entries(
    kv_storage: &(dyn KVStorage + Send + Sync),
) -> ApiResult<Vec<(String, serde_json::Value)>> {
    let keys = kv_storage
        .keys_with_suffix(DOCUMENT_METADATA_SUFFIX)
        .await?;
    if keys.is_empty() {
        return Ok(vec![]);
    }
    let values = kv_storage.get_by_ids_ordered(&keys).await?;
    Ok(keys
        .into_iter()
        .zip(values)
        .filter_map(|(key, value)| value.map(|v| (key, v)))
        .collect())
}

/// Load all document metadata values via indexed suffix scan (unscoped).
pub async fn load_all_document_metadata(
    kv_storage: &(dyn KVStorage + Send + Sync),
) -> ApiResult<Vec<serde_json::Value>> {
    Ok(load_all_document_metadata_entries(kv_storage)
        .await?
        .into_iter()
        .map(|(_, value)| value)
        .collect())
}

/// Filter metadata JSON values to those visible under tenant context (legacy alias SSOT).
pub fn filter_metadata_for_tenant<'a>(
    values: impl IntoIterator<Item = &'a serde_json::Value>,
    tenant_ctx: &TenantContext,
) -> Vec<&'a serde_json::Value> {
    values
        .into_iter()
        .filter(|value| metadata_matches_tenant_context(value, tenant_ctx))
        .collect()
}

/// Result of a bounded interactive metadata load (list path).
#[derive(Debug, Clone)]
pub struct ScopedMetadataLoad {
    pub entries: Vec<(String, serde_json::Value)>,
    /// True when key enumeration exceeded `max_entries` before value fetch.
    pub truncated: bool,
}

/// Load scoped `(key, metadata)` pairs for tenant/workspace (unbounded).
///
/// Prefer [`load_scoped_document_metadata_entries_limited`] on interactive
/// HTTP paths — this unlimited variant is for internal/admin scans.
pub async fn load_scoped_document_metadata_entries(
    kv_storage: &(dyn KVStorage + Send + Sync),
    tenant_ctx: &TenantContext,
) -> ApiResult<Vec<(String, serde_json::Value)>> {
    if let Some(workspace_id) = tenant_ctx.workspace_id.as_deref() {
        let metadata_keys = list_workspace_metadata_keys(kv_storage, workspace_id).await?;
        if !metadata_keys.is_empty() {
            return Ok(
                fetch_scoped_entries(kv_storage, tenant_ctx, metadata_keys, false)
                    .await?
                    .entries,
            );
        }
    }
    let keys = kv_storage
        .keys_with_suffix(DOCUMENT_METADATA_SUFFIX)
        .await?;
    Ok(fetch_scoped_entries(kv_storage, tenant_ctx, keys, false)
        .await?
        .entries)
}

/// Load scoped metadata with a hard cap on keys **before** `get_by_ids_ordered`.
///
/// WHY: interactive list must not pay unbounded KV key/value materialization for
/// huge workspaces. Truncation happens at the storage key-list stage (Postgres
/// `LIMIT` on index/suffix scan), not after loading every JSON blob into memory.
pub async fn load_scoped_document_metadata_entries_limited(
    kv_storage: &(dyn KVStorage + Send + Sync),
    tenant_ctx: &TenantContext,
    max_entries: usize,
) -> ApiResult<ScopedMetadataLoad> {
    let max_entries = max_entries.max(1);

    if let Some(workspace_id) = tenant_ctx.workspace_id.as_deref() {
        let (metadata_keys, truncated) =
            list_workspace_metadata_keys_limited(kv_storage, workspace_id, max_entries).await?;
        if !metadata_keys.is_empty() {
            return fetch_scoped_entries(kv_storage, tenant_ctx, metadata_keys, truncated).await;
        }
    }

    let (keys, truncated) = kv_storage
        .keys_with_suffix_limited(DOCUMENT_METADATA_SUFFIX, max_entries)
        .await?;
    fetch_scoped_entries(kv_storage, tenant_ctx, keys, truncated).await
}

async fn fetch_scoped_entries(
    kv_storage: &(dyn KVStorage + Send + Sync),
    tenant_ctx: &TenantContext,
    metadata_keys: Vec<String>,
    truncated: bool,
) -> ApiResult<ScopedMetadataLoad> {
    if metadata_keys.is_empty() {
        return Ok(ScopedMetadataLoad {
            entries: vec![],
            truncated: false,
        });
    }
    let values = kv_storage.get_by_ids_ordered(&metadata_keys).await?;
    let entries = metadata_keys
        .into_iter()
        .zip(values)
        .filter_map(|(key, value)| value.map(|v| (key, v)))
        .filter(|(_, value)| metadata_matches_tenant_context(value, tenant_ctx))
        .collect();
    Ok(ScopedMetadataLoad { entries, truncated })
}

/// Load `(key, metadata)` for a workspace using `wsdoc:` index prefix scan.
pub async fn load_workspace_metadata_entries_by_index(
    kv_storage: &(dyn KVStorage + Send + Sync),
    workspace_id: &str,
) -> ApiResult<Vec<(String, serde_json::Value)>> {
    let metadata_keys = list_workspace_metadata_keys(kv_storage, workspace_id).await?;
    if metadata_keys.is_empty() {
        return Ok(vec![]);
    }
    let values = kv_storage.get_by_ids_ordered(&metadata_keys).await?;
    Ok(metadata_keys
        .into_iter()
        .zip(values)
        .filter_map(|(key, value)| value.map(|v| (key, v)))
        .collect())
}

/// Load document metadata scoped to tenant/workspace.
pub async fn load_scoped_document_metadata(
    kv_storage: &(dyn KVStorage + Send + Sync),
    tenant_ctx: &TenantContext,
) -> ApiResult<Vec<serde_json::Value>> {
    Ok(
        load_scoped_document_metadata_entries(kv_storage, tenant_ctx)
            .await?
            .into_iter()
            .map(|(_, value)| value)
            .collect(),
    )
}

/// Fetch tenant-scoped in-flight staging metadata keys/values (SPEC-086 / 068 SSOT).
///
/// Text/MD admits write `staging:{doc}-metadata` only until promote. The wsdoc index
/// skips staging keys, so interactive list/track/progress must merge these explicitly.
async fn load_staging_metadata_entries(
    kv_storage: &(dyn KVStorage + Send + Sync),
    tenant_ctx: &TenantContext,
) -> ApiResult<Vec<(String, serde_json::Value)>> {
    let staging_keys: Vec<String> = kv_storage
        .keys_with_prefix("staging:")
        .await?
        .into_iter()
        .filter(|k| k.ends_with(DOCUMENT_METADATA_SUFFIX) && !k.contains(":hash:"))
        .collect();
    if staging_keys.is_empty() {
        return Ok(Vec::new());
    }

    let staging_values = kv_storage.get_by_ids_ordered(&staging_keys).await?;
    let mut out = Vec::new();
    for (key, maybe_value) in staging_keys.into_iter().zip(staging_values) {
        let Some(value) = maybe_value else {
            continue;
        };
        if !metadata_matches_tenant_context(&value, tenant_ctx) {
            continue;
        }
        let Some(id) = value.get("id").and_then(|i| i.as_str()) else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        out.push((key, value));
    }
    Ok(out)
}

/// Merge staging metadata into final `(key, value)` entries (prefer final on id collision).
///
/// Used by documents list (after limited final load) so in-flight MD appears in ActiveRuns.
pub async fn merge_staging_metadata_entries(
    kv_storage: &(dyn KVStorage + Send + Sync),
    tenant_ctx: &TenantContext,
    mut entries: Vec<(String, serde_json::Value)>,
) -> ApiResult<Vec<(String, serde_json::Value)>> {
    let staging = load_staging_metadata_entries(kv_storage, tenant_ctx).await?;
    if staging.is_empty() {
        return Ok(entries);
    }

    let mut seen_ids: std::collections::HashSet<String> = entries
        .iter()
        .filter_map(|(_, v)| v.get("id").and_then(|i| i.as_str()).map(str::to_string))
        .collect();

    for (key, value) in staging {
        let Some(id) = value.get("id").and_then(|i| i.as_str()) else {
            continue;
        };
        if seen_ids.contains(id) {
            // Prefer final `{doc}-metadata` over staging when both exist.
            continue;
        }
        seen_ids.insert(id.to_string());
        entries.push((key, value));
    }
    Ok(entries)
}

/// Merge staging metadata into final value list (prefer final on id collision).
pub async fn merge_staging_metadata_values(
    kv_storage: &(dyn KVStorage + Send + Sync),
    tenant_ctx: &TenantContext,
    values: Vec<serde_json::Value>,
) -> ApiResult<Vec<serde_json::Value>> {
    let entries: Vec<(String, serde_json::Value)> = values
        .into_iter()
        .map(|v| {
            let key = v
                .get("id")
                .and_then(|i| i.as_str())
                .map(metadata_key_for_document)
                .unwrap_or_default();
            (key, v)
        })
        .collect();
    Ok(
        merge_staging_metadata_entries(kv_storage, tenant_ctx, entries)
            .await?
            .into_iter()
            .map(|(_, v)| v)
            .collect(),
    )
}

/// Progress / in-flight load (068 + 086): final workspace docs **plus** staging metadata.
///
/// Thin wrapper over [`merge_staging_metadata_values`] — one merge implementation (no third loader).
pub async fn load_scoped_document_metadata_for_progress(
    kv_storage: &(dyn KVStorage + Send + Sync),
    tenant_ctx: &TenantContext,
) -> ApiResult<Vec<serde_json::Value>> {
    let values = load_scoped_document_metadata(kv_storage, tenant_ctx).await?;
    merge_staging_metadata_values(kv_storage, tenant_ctx, values).await
}

/// KV keys to remove when cascade-deleting a workspace's documents.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceDocumentDeletePlan {
    pub keys: Vec<String>,
    pub documents: usize,
    pub chunks: usize,
}

/// Plan workspace document KV deletion using workspace index with suffix-scan fallback.
pub async fn plan_workspace_document_kv_deletion(
    kv_storage: &(dyn KVStorage + Send + Sync),
    workspace_id: &str,
) -> ApiResult<WorkspaceDocumentDeletePlan> {
    let doc_ids = crate::services::workspace_document_index::list_workspace_document_ids(
        kv_storage,
        workspace_id,
    )
    .await?;
    if !doc_ids.is_empty() {
        return build_delete_plan_for_doc_ids(kv_storage, workspace_id, doc_ids).await;
    }

    plan_workspace_document_kv_deletion_suffix_fallback(kv_storage, workspace_id).await
}

async fn build_delete_plan_for_doc_ids(
    kv_storage: &(dyn KVStorage + Send + Sync),
    workspace_id: &str,
    doc_ids: Vec<String>,
) -> ApiResult<WorkspaceDocumentDeletePlan> {
    let mut plan = WorkspaceDocumentDeletePlan::default();
    for doc_id in doc_ids {
        let metadata_key = metadata_key_for_document(&doc_id);
        plan.keys.push(metadata_key.clone());
        plan.keys.push(format!("{doc_id}-content"));
        plan.keys
            .push(edgequake_storage::kv_keys::workspace_doc_index(
                workspace_id,
                &doc_id,
            ));

        let chunk_prefix = format!("{doc_id}-chunk-");
        let chunk_keys = kv_storage.keys_with_prefix(&chunk_prefix).await?;
        plan.chunks += chunk_keys.len();
        plan.keys.extend(chunk_keys);
        plan.documents += 1;
    }
    Ok(plan)
}

/// Legacy suffix-scan delete planner (fallback when wsdoc index is empty).
async fn plan_workspace_document_kv_deletion_suffix_fallback(
    kv_storage: &(dyn KVStorage + Send + Sync),
    workspace_id: &str,
) -> ApiResult<WorkspaceDocumentDeletePlan> {
    let entries = load_all_document_metadata_entries(kv_storage).await?;
    let mut plan = WorkspaceDocumentDeletePlan::default();

    for (metadata_key, metadata) in entries {
        let doc_workspace = metadata
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        if doc_workspace != workspace_id {
            continue;
        }

        let Some(doc_id) = document_id_from_metadata_key(&metadata_key) else {
            continue;
        };

        plan.keys.push(metadata_key);
        plan.keys.push(format!("{doc_id}-content"));

        let chunk_prefix = format!("{doc_id}-chunk-");
        let chunk_keys = kv_storage.keys_with_prefix(&chunk_prefix).await?;
        plan.chunks += chunk_keys.len();
        plan.keys.extend(chunk_keys);

        plan.documents += 1;
    }

    Ok(plan)
}

/// Parsed document metadata for workspace-scoped bulk operations (IMP-017 DRY).
#[derive(Debug, Clone)]
pub struct WorkspaceDocumentRecord {
    pub doc_id: String,
    pub title: String,
    pub chunk_count: usize,
    pub source_type: Option<String>,
    pub pdf_id_str: Option<String>,
    pub status: Option<String>,
}

fn parse_workspace_document_record(value: &serde_json::Value) -> Option<WorkspaceDocumentRecord> {
    let obj = value.as_object()?;
    let doc_id = obj.get("id")?.as_str()?.to_string();
    Some(WorkspaceDocumentRecord {
        doc_id: doc_id.clone(),
        title: obj
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(&doc_id)
            .to_string(),
        chunk_count: obj.get("chunk_count").and_then(|v| v.as_u64()).unwrap_or(1) as usize,
        source_type: obj
            .get("source_type")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        pdf_id_str: obj
            .get("pdf_id")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        status: obj
            .get("status")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

/// Load documents belonging to a workspace via index prefix scan + slug-aware fallback.
pub async fn load_workspace_documents(
    kv_storage: &(dyn KVStorage + Send + Sync),
    workspace_id: &uuid::Uuid,
    workspace_slug: &str,
) -> ApiResult<Vec<WorkspaceDocumentRecord>> {
    use crate::handlers::isolation::doc_belongs_to_workspace;

    let workspace_id_str = workspace_id.to_string();

    // Index path: O(workspace docs) when wsdoc pointers exist (post migration 047 / write hooks).
    if workspace_slug != "default" {
        let entries =
            load_workspace_metadata_entries_by_index(kv_storage, &workspace_id_str).await?;
        if !entries.is_empty() {
            return Ok(entries
                .into_iter()
                .filter_map(|(_, value)| parse_workspace_document_record(&value))
                .collect());
        }
    }

    // Fallback: global suffix scan + slug-aware filter (legacy default alias, pre-backfill).
    let values = load_all_document_metadata(kv_storage).await?;
    let mut docs = Vec::new();

    for value in values {
        let doc_workspace = value
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        if !doc_belongs_to_workspace(doc_workspace, &workspace_id_str, workspace_slug) {
            continue;
        }

        if let Some(record) = parse_workspace_document_record(&value) {
            docs.push(record);
        }
    }

    Ok(docs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::{default_tenant_uuid, default_workspace_uuid};

    fn ctx(tenant: &str, workspace: &str) -> TenantContext {
        TenantContext {
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            user_id: None,
        }
    }

    #[test]
    fn document_id_from_metadata_key_strips_suffix() {
        assert_eq!(
            document_id_from_metadata_key("abc-123-metadata").as_deref(),
            Some("abc-123")
        );
        assert!(document_id_from_metadata_key("not-metadata-key").is_none());
    }

    #[test]
    fn canonical_document_id_prefers_metadata_key() {
        let key = "real-doc-id-metadata";
        let meta = serde_json::json!({ "id": "wrong-id" });
        assert_eq!(canonical_document_id(key, &meta), "real-doc-id");
    }

    #[test]
    fn filter_includes_legacy_default_alias() {
        let values = vec![serde_json::json!({
            "tenant_id": "default",
            "workspace_id": "default",
            "status": "completed",
        })];
        let filtered = filter_metadata_for_tenant(&values, &ctx("default", "default"));
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn filter_excludes_other_workspace_uuid() {
        let values = vec![serde_json::json!({
            "tenant_id": default_tenant_uuid().to_string(),
            "workspace_id": default_workspace_uuid().to_string(),
            "status": "completed",
        })];
        let other = uuid::Uuid::new_v4().to_string();
        let filtered = filter_metadata_for_tenant(&values, &ctx("default", &other));
        assert!(filtered.is_empty());
    }

    #[tokio::test]
    async fn plan_workspace_delete_uses_prefix_not_full_keys_scan() {
        use edgequake_storage::MemoryKVStorage;
        use std::sync::Arc;

        let kv = Arc::new(MemoryKVStorage::new("spec027-ws-delete"));
        kv.initialize().await.unwrap();

        let ws_target = uuid::Uuid::new_v4().to_string();
        let ws_other = uuid::Uuid::new_v4().to_string();

        kv.upsert(&[
            (
                "doc-target-metadata".to_string(),
                serde_json::json!({
                    "id": "doc-target",
                    "workspace_id": ws_target,
                }),
            ),
            (
                "doc-target-content".to_string(),
                serde_json::json!({ "content": "target" }),
            ),
            (
                "doc-target-chunk-0".to_string(),
                serde_json::json!({ "text": "chunk" }),
            ),
            (
                "doc-other-metadata".to_string(),
                serde_json::json!({
                    "id": "doc-other",
                    "workspace_id": ws_other,
                }),
            ),
            (
                "doc-other-content".to_string(),
                serde_json::json!({ "content": "other" }),
            ),
        ])
        .await
        .unwrap();

        let plan = plan_workspace_document_kv_deletion(kv.as_ref(), &ws_target)
            .await
            .unwrap();

        assert_eq!(plan.documents, 1);
        assert_eq!(plan.chunks, 1);
        assert!(plan.keys.contains(&"doc-target-metadata".to_string()));
        assert!(plan.keys.contains(&"doc-target-content".to_string()));
        assert!(plan.keys.contains(&"doc-target-chunk-0".to_string()));
        assert!(!plan.keys.iter().any(|k| k.starts_with("doc-other")));
    }

    #[tokio::test]
    async fn plan_workspace_delete_uses_wsdoc_index_when_present() {
        use crate::services::sync_workspace_document_index;
        use edgequake_storage::MemoryKVStorage;
        use std::sync::Arc;

        let kv = Arc::new(MemoryKVStorage::new("spec027-ws-index-delete"));
        kv.initialize().await.unwrap();

        let ws_target = uuid::Uuid::new_v4().to_string();
        let meta = serde_json::json!({
            "id": "doc-indexed",
            "workspace_id": ws_target,
        });
        kv.upsert(&[
            ("doc-indexed-metadata".to_string(), meta.clone()),
            (
                "doc-indexed-content".to_string(),
                serde_json::json!({ "content": "x" }),
            ),
            (
                "doc-indexed-chunk-0".to_string(),
                serde_json::json!({ "text": "c" }),
            ),
        ])
        .await
        .unwrap();
        sync_workspace_document_index(kv.as_ref(), "doc-indexed-metadata", &meta)
            .await
            .unwrap();

        let plan = plan_workspace_document_kv_deletion(kv.as_ref(), &ws_target)
            .await
            .unwrap();

        assert_eq!(plan.documents, 1);
        assert_eq!(plan.chunks, 1);
        assert!(plan
            .keys
            .iter()
            .any(|k| k.starts_with("wsdoc:") && k.contains("doc-indexed")));
    }

    #[tokio::test]
    async fn load_workspace_documents_uses_index_when_populated() {
        use crate::services::sync_workspace_document_index;
        use edgequake_storage::MemoryKVStorage;
        use std::sync::Arc;

        let kv = Arc::new(MemoryKVStorage::new("spec027-ws-index-load"));
        kv.initialize().await.unwrap();

        let ws_id = uuid::Uuid::new_v4();
        let meta = serde_json::json!({
            "id": "doc-via-index",
            "workspace_id": ws_id.to_string(),
            "title": "Indexed doc",
        });
        kv.upsert(&[("doc-via-index-metadata".to_string(), meta.clone())])
            .await
            .unwrap();
        sync_workspace_document_index(kv.as_ref(), "doc-via-index-metadata", &meta)
            .await
            .unwrap();

        let docs = load_workspace_documents(kv.as_ref(), &ws_id, "custom-slug")
            .await
            .unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].doc_id, "doc-via-index");
    }

    #[tokio::test]
    async fn load_workspace_documents_filters_by_slug_aware_membership() {
        use edgequake_storage::MemoryKVStorage;
        use std::sync::Arc;

        let kv = Arc::new(MemoryKVStorage::new("spec027-ws-load"));
        kv.initialize().await.unwrap();

        let ws_id = uuid::Uuid::new_v4();
        let other_id = uuid::Uuid::new_v4();

        kv.upsert(&[
            (
                "doc-in-ws-metadata".to_string(),
                serde_json::json!({
                    "id": "doc-in-ws",
                    "workspace_id": ws_id.to_string(),
                    "title": "In workspace",
                }),
            ),
            (
                "doc-legacy-default-metadata".to_string(),
                serde_json::json!({
                    "id": "doc-legacy-default",
                    "workspace_id": "default",
                    "title": "Legacy default",
                }),
            ),
            (
                "doc-other-ws-metadata".to_string(),
                serde_json::json!({
                    "id": "doc-other-ws",
                    "workspace_id": other_id.to_string(),
                }),
            ),
        ])
        .await
        .unwrap();

        let docs = load_workspace_documents(kv.as_ref(), &ws_id, "not-default")
            .await
            .unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].doc_id, "doc-in-ws");

        let default_docs = load_workspace_documents(kv.as_ref(), &uuid::Uuid::new_v4(), "default")
            .await
            .unwrap();
        assert_eq!(default_docs.len(), 1);
        assert_eq!(default_docs[0].doc_id, "doc-legacy-default");
    }

    #[tokio::test]
    async fn limited_load_truncates_keys_before_value_fetch() {
        use edgequake_storage::MemoryKVStorage;
        use std::sync::Arc;

        let kv = Arc::new(MemoryKVStorage::new("spec027-limited-load"));
        kv.initialize().await.unwrap();

        let ws = uuid::Uuid::new_v4().to_string();
        let tenant = uuid::Uuid::new_v4().to_string();
        let mut upserts = Vec::new();
        for i in 0..5 {
            let id = format!("doc-cap-{i}");
            upserts.push((
                format!("{id}-metadata"),
                serde_json::json!({
                    "id": id,
                    "tenant_id": tenant,
                    "workspace_id": ws,
                }),
            ));
        }
        kv.upsert(&upserts).await.unwrap();

        let loaded =
            load_scoped_document_metadata_entries_limited(kv.as_ref(), &ctx(&tenant, &ws), 2)
                .await
                .unwrap();
        assert!(loaded.truncated);
        assert!(loaded.entries.len() <= 2);
    }

    #[tokio::test]
    async fn limited_load_exact_cap_without_extra_is_not_truncated() {
        use edgequake_storage::MemoryKVStorage;
        use std::sync::Arc;

        let kv = Arc::new(MemoryKVStorage::new("spec027-exact-cap"));
        kv.initialize().await.unwrap();
        let ws = uuid::Uuid::new_v4().to_string();
        let tenant = uuid::Uuid::new_v4().to_string();
        let mut upserts = Vec::new();
        for i in 0..2 {
            let id = format!("doc-exact-{i}");
            upserts.push((
                format!("{id}-metadata"),
                serde_json::json!({
                    "id": id,
                    "tenant_id": tenant,
                    "workspace_id": ws,
                }),
            ));
        }
        kv.upsert(&upserts).await.unwrap();

        let loaded =
            load_scoped_document_metadata_entries_limited(kv.as_ref(), &ctx(&tenant, &ws), 2)
                .await
                .unwrap();
        assert!(!loaded.truncated, "exact fill must not report truncated");
        assert_eq!(loaded.entries.len(), 2);
    }

    #[tokio::test]
    async fn limited_load_max_entries_one_returns_single() {
        use edgequake_storage::MemoryKVStorage;
        use std::sync::Arc;

        let kv = Arc::new(MemoryKVStorage::new("spec027-cap-one"));
        kv.initialize().await.unwrap();
        let ws = uuid::Uuid::new_v4().to_string();
        let tenant = uuid::Uuid::new_v4().to_string();
        let mut upserts = Vec::new();
        for i in 0..3 {
            let id = format!("doc-one-{i}");
            upserts.push((
                format!("{id}-metadata"),
                serde_json::json!({
                    "id": id,
                    "tenant_id": tenant,
                    "workspace_id": ws,
                }),
            ));
        }
        kv.upsert(&upserts).await.unwrap();

        let loaded = load_scoped_document_metadata_entries_limited(
            kv.as_ref(),
            &ctx(&tenant, &ws),
            0, // clamped to 1
        )
        .await
        .unwrap();
        assert!(loaded.truncated);
        assert_eq!(loaded.entries.len(), 1);
    }
}
