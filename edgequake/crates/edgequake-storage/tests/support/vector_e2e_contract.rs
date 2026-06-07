//! Shared vector storage E2E contract (STORE-DRY-003 / P2-11).
#![allow(dead_code)]

use edgequake_storage::traits::VectorStorage;

pub const CONTRACT_DIMENSION: usize = 384;

fn embedding(seed: f32) -> Vec<f32> {
    (0..CONTRACT_DIMENSION)
        .map(|i| ((i as f32 + seed) / 1000.0).sin())
        .collect()
}

/// Upsert, get, update, delete a single vector.
pub async fn assert_vector_basic_crud<V: VectorStorage + ?Sized>(storage: &V) {
    assert_eq!(storage.dimension(), CONTRACT_DIMENSION);

    let vec = embedding(1.0);
    storage
        .upsert(&[(
            "vec-1".to_string(),
            vec.clone(),
            serde_json::json!({"label": "first"}),
        )])
        .await
        .unwrap();

    let fetched = storage.get_by_id("vec-1").await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().len(), CONTRACT_DIMENSION);

    storage
        .upsert(&[(
            "vec-1".to_string(),
            embedding(2.0),
            serde_json::json!({"label": "updated"}),
        )])
        .await
        .unwrap();

    storage.delete(&["vec-1".to_string()]).await.unwrap();
    assert!(storage.get_by_id("vec-1").await.unwrap().is_none());
}

/// Insert vectors and verify similarity query returns results.
pub async fn assert_vector_query<V: VectorStorage + ?Sized>(storage: &V) {
    let query_vec = embedding(0.0);
    storage
        .upsert(&[("vec-a".to_string(), embedding(0.1), serde_json::json!({}))])
        .await
        .unwrap();
    storage
        .upsert(&[("vec-b".to_string(), embedding(0.2), serde_json::json!({}))])
        .await
        .unwrap();

    let results = storage.query(&query_vec, 5, None).await.unwrap();
    assert!(!results.is_empty());
    assert!(results.len() <= 2);
}

/// Bulk upsert and count.
pub async fn assert_vector_bulk_count<V: VectorStorage + ?Sized>(storage: &V) {
    let data: Vec<(String, Vec<f32>, serde_json::Value)> = (0..20)
        .map(|i| {
            (
                format!("vec-{i}"),
                embedding(i as f32),
                serde_json::json!({"index": i}),
            )
        })
        .collect();
    storage.upsert(&data).await.unwrap();
    assert_eq!(storage.count().await.unwrap(), 20);
}

fn orthogonal_embedding(cluster: usize) -> Vec<f32> {
    (0..CONTRACT_DIMENSION)
        .map(|i| {
            if cluster == 0 {
                (i as f32 * 0.01).sin()
            } else {
                (i as f32 * 0.01).cos()
            }
        })
        .collect()
}

/// Two-cluster similarity: query embedding should rank same-cluster vectors first.
pub async fn assert_vector_cluster_similarity<V: VectorStorage + ?Sized>(storage: &V) {
    for i in 0..5 {
        let mut vec = orthogonal_embedding(0);
        for value in vec.iter_mut().take(CONTRACT_DIMENSION) {
            *value += i as f32 * 0.001;
        }
        storage
            .upsert(&[(
                format!("cluster0-{i}"),
                vec,
                serde_json::json!({"cluster": 0}),
            )])
            .await
            .unwrap();
    }
    for i in 0..5 {
        let mut vec = orthogonal_embedding(1);
        for value in vec.iter_mut().take(CONTRACT_DIMENSION) {
            *value += i as f32 * 0.001;
        }
        storage
            .upsert(&[(
                format!("cluster1-{i}"),
                vec,
                serde_json::json!({"cluster": 1}),
            )])
            .await
            .unwrap();
    }

    let results = storage
        .query(&orthogonal_embedding(0), 3, None)
        .await
        .unwrap();
    assert_eq!(results.len(), 3);
    for result in &results {
        assert!(
            result.id.starts_with("cluster0"),
            "expected cluster0, got {}",
            result.id
        );
    }

    let results = storage
        .query(&orthogonal_embedding(1), 3, None)
        .await
        .unwrap();
    assert_eq!(results.len(), 3);
    for result in &results {
        assert!(
            result.id.starts_with("cluster1"),
            "expected cluster1, got {}",
            result.id
        );
    }
}

/// Query with ID filter restricts result set.
pub async fn assert_vector_filtered_query<V: VectorStorage + ?Sized>(storage: &V) {
    for i in 0..10 {
        storage
            .upsert(&[(
                format!("vec-{i}"),
                embedding(i as f32),
                serde_json::json!({"index": i}),
            )])
            .await
            .unwrap();
    }

    let filter_ids = vec![
        "vec-0".to_string(),
        "vec-1".to_string(),
        "vec-2".to_string(),
    ];
    let results = storage
        .query(&embedding(0.0), 5, Some(&filter_ids))
        .await
        .unwrap();

    assert!(!results.is_empty());
    assert!(results.len() <= 3);
    for result in &results {
        assert!(filter_ids.contains(&result.id));
    }
}
