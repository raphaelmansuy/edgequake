//! Paginated injection list — SSOT for list handler (SPEC-025 6.5).

use std::sync::Arc;

use edgequake_storage::traits::KVStorage;

use crate::handlers::injection_types::InjectionSummary;
use crate::services::injection_process::injection_list_prefix;

/// Default page size for injection list API.
pub const DEFAULT_INJECTION_LIST_LIMIT: usize = 50;

/// Maximum allowed page size.
pub const MAX_INJECTION_LIST_LIMIT: usize = 200;

/// Paginated injection list result.
#[derive(Debug, Clone)]
pub struct InjectionListPage {
    pub items: Vec<InjectionSummary>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

impl InjectionListPage {
    pub fn has_more(&self) -> bool {
        self.offset + self.items.len() < self.total
    }
}

/// List injection summaries for a workspace with offset/limit pagination.
///
/// Keys are prefix-scanned once; metadata is loaded for sort, then sliced.
pub async fn list_injections_paged(
    kv_storage: &Arc<dyn KVStorage>,
    workspace_id: &str,
    limit: usize,
    offset: usize,
) -> edgequake_storage::error::Result<InjectionListPage> {
    let limit = limit.clamp(1, MAX_INJECTION_LIST_LIMIT);
    let offset = offset.min(usize::MAX.saturating_sub(limit));

    let prefix = injection_list_prefix(workspace_id);
    let keys = kv_storage.keys_with_prefix(&prefix).await?;
    let meta_keys: Vec<String> = keys
        .into_iter()
        .filter(|k| k.ends_with("-metadata"))
        .collect();
    let total = meta_keys.len();

    // IMP-075-02: one RT for all metadata keys (not N× get_by_id) — O(K log N).
    let values = kv_storage.get_by_ids_ordered(&meta_keys).await?;
    let mut items: Vec<InjectionSummary> = values
        .into_iter()
        .flatten()
        .map(|val| summary_from_meta(&val))
        .collect();

    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let page_items: Vec<InjectionSummary> = items.into_iter().skip(offset).take(limit).collect();

    Ok(InjectionListPage {
        items: page_items,
        total,
        limit,
        offset,
    })
}

fn str_field(val: &serde_json::Value, key: &str) -> String {
    val.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn str_field_or(val: &serde_json::Value, key: &str, default: &str) -> String {
    val.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

pub fn summary_from_meta(val: &serde_json::Value) -> InjectionSummary {
    InjectionSummary {
        injection_id: str_field(val, "id"),
        name: str_field(val, "name"),
        status: str_field_or(val, "status", "unknown"),
        entity_count: val
            .get("entity_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        source_type: str_field_or(val, "source_type", "text"),
        error: val
            .get("error")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        created_at: str_field(val, "created_at"),
        updated_at: str_field(val, "updated_at"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::injection_process::injection_meta_key;
    use edgequake_storage::adapters::memory::MemoryKVStorage;
    use serde_json::json;

    #[tokio::test]
    async fn pagination_slices_sorted_results() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("inj-list"));
        let ws = "default";
        for (id, created) in [
            ("a", "2026-01-01"),
            ("b", "2026-06-01"),
            ("c", "2026-03-01"),
        ] {
            let key = injection_meta_key(ws, id);
            kv.upsert(&[(
                key,
                json!({
                    "id": id,
                    "name": id,
                    "status": "completed",
                    "entity_count": 1,
                    "source_type": "text",
                    "created_at": created,
                    "updated_at": created,
                }),
            )])
            .await
            .unwrap();
        }

        let page = list_injections_paged(&kv, ws, 2, 0).await.unwrap();
        assert_eq!(page.total, 3);
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].injection_id, "b");
        assert!(page.has_more());

        let page2 = list_injections_paged(&kv, ws, 2, 2).await.unwrap();
        assert_eq!(page2.items.len(), 1);
        assert!(!page2.has_more());
    }
}
