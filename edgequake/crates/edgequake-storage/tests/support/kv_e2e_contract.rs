//! Shared KV storage E2E contract (STORE-DRY-003 / P2-11).
#![allow(dead_code)]

use std::collections::HashSet;

use edgequake_storage::traits::KVStorage;

/// Basic create-read-update-delete cycle.
pub async fn assert_kv_basic_crud<K: KVStorage + ?Sized>(storage: &K) {
    let data = vec![(
        "key1".to_string(),
        serde_json::json!({"name": "test", "value": 42}),
    )];
    storage.upsert(&data).await.unwrap();

    let result = storage.get_by_id("key1").await.unwrap();
    assert!(result.is_some());
    let doc = result.unwrap();
    assert_eq!(doc["name"], "test");
    assert_eq!(doc["value"], 42);

    let updated = vec![(
        "key1".to_string(),
        serde_json::json!({"name": "updated", "value": 100}),
    )];
    storage.upsert(&updated).await.unwrap();
    assert_eq!(
        storage.get_by_id("key1").await.unwrap().unwrap()["name"],
        "updated"
    );

    storage.delete(&["key1".to_string()]).await.unwrap();
    assert!(storage.get_by_id("key1").await.unwrap().is_none());
}

/// Bulk upsert, bulk get, count, and keys listing.
pub async fn assert_kv_bulk_operations<K: KVStorage + ?Sized>(storage: &K) {
    let data: Vec<(String, serde_json::Value)> = (0..50)
        .map(|i| {
            (
                format!("doc-{i}"),
                serde_json::json!({"index": i, "content": format!("Document {i}")}),
            )
        })
        .collect();
    storage.upsert(&data).await.unwrap();

    let ids: Vec<String> = (0..25).map(|i| format!("doc-{i}")).collect();
    assert_eq!(storage.get_by_ids(&ids).await.unwrap().len(), 25);
    assert_eq!(storage.count().await.unwrap(), 50);

    let keys = storage.keys().await.unwrap();
    assert_eq!(keys.len(), 50);
    assert!(keys.contains(&"doc-0".to_string()));
    assert!(keys.contains(&"doc-49".to_string()));
}

/// filter_keys returns only missing keys from the input set.
pub async fn assert_kv_filter_keys<K: KVStorage + ?Sized>(storage: &K) {
    storage
        .upsert(&[
            ("exists1".to_string(), serde_json::json!({})),
            ("exists2".to_string(), serde_json::json!({})),
        ])
        .await
        .unwrap();

    let check: HashSet<String> = ["exists1", "exists2", "missing1", "missing2"]
        .into_iter()
        .map(String::from)
        .collect();
    let missing = storage.filter_keys(check).await.unwrap();
    assert_eq!(missing.len(), 2);
    assert!(missing.contains("missing1"));
    assert!(missing.contains("missing2"));
}
