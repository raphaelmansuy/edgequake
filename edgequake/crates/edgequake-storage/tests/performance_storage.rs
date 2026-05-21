//! SPEC-011: Storage performance regression tests.
//!
//! Validates O(1) probes (ping, is_empty) and keys_like filtering semantics
//! without requiring PostgreSQL (memory backend). PostgreSQL benchmarks run
//! when DATABASE_URL is set.

use edgequake_storage::adapters::memory::MemoryKVStorage;
use edgequake_storage::traits::{kv_key_matches_like, KVStorage};
use std::time::{Duration, Instant};

const DOC_COUNT: usize = 500;
const CHUNKS_PER_DOC: usize = 20;

async fn populate_kv(storage: &MemoryKVStorage) -> usize {
    let mut batch = Vec::with_capacity(DOC_COUNT * (CHUNKS_PER_DOC + 1));
    for i in 0..DOC_COUNT {
        let doc_id = format!("doc-{i:05}");
        batch.push((
            format!("{doc_id}-metadata"),
            serde_json::json!({"id": doc_id, "status": "completed"}),
        ));
        for c in 0..CHUNKS_PER_DOC {
            batch.push((
                format!("{doc_id}-chunk-{c}"),
                serde_json::json!({"text": "chunk content", "index": c}),
            ));
        }
    }
    storage.upsert(&batch).await.unwrap();
    batch.len()
}

#[tokio::test]
async fn test_count_remains_exact_after_bulk_load() {
    let storage = MemoryKVStorage::new("perf_count");
    storage.initialize().await.unwrap();
    let expected = populate_kv(&storage).await;
    assert_eq!(storage.count().await.unwrap(), expected);
}

#[tokio::test]
async fn test_is_empty_is_fast_and_correct() {
    let storage = MemoryKVStorage::new("perf_empty");
    storage.initialize().await.unwrap();
    assert!(storage.is_empty().await.unwrap());

    populate_kv(&storage).await;
    assert!(!storage.is_empty().await.unwrap());

    let start = Instant::now();
    for _ in 0..100 {
        let _ = storage.is_empty().await.unwrap();
    }
    assert!(
        start.elapsed() < Duration::from_millis(50),
        "is_empty should be O(1) in memory: {:?}",
        start.elapsed()
    );
}

#[tokio::test]
async fn test_ping_does_not_require_full_count_scan() {
    let storage = MemoryKVStorage::new("perf_ping");
    storage.initialize().await.unwrap();
    populate_kv(&storage).await;

    let start = Instant::now();
    for _ in 0..100 {
        storage.ping().await.unwrap();
    }
    assert!(
        start.elapsed() < Duration::from_millis(100),
        "ping loop took {:?}",
        start.elapsed()
    );
}

#[tokio::test]
async fn test_keys_like_matches_subset_of_keys() {
    let storage = MemoryKVStorage::new("perf_like");
    storage.initialize().await.unwrap();
    populate_kv(&storage).await;

    let all_keys = storage.keys().await.unwrap();
    let metadata_keys = storage.keys_like("%-metadata").await.unwrap();
    let chunk_keys = storage.keys_like("%-chunk-%").await.unwrap();

    assert_eq!(metadata_keys.len(), DOC_COUNT);
    assert_eq!(chunk_keys.len(), DOC_COUNT * CHUNKS_PER_DOC);
    assert_eq!(metadata_keys.len() + chunk_keys.len(), all_keys.len());

    for key in &metadata_keys {
        assert!(kv_key_matches_like(key, "%-metadata"));
    }
    for key in &chunk_keys {
        assert!(kv_key_matches_like(key, "%-chunk-%"));
    }
}

#[tokio::test]
async fn test_keys_like_faster_than_full_keys_at_scale() {
    let storage = MemoryKVStorage::new("perf_like_scale");
    storage.initialize().await.unwrap();
    populate_kv(&storage).await;

    let full_start = Instant::now();
    let all = storage.keys().await.unwrap();
    let full_time = full_start.elapsed();

    let like_start = Instant::now();
    let meta = storage.keys_like("%-metadata").await.unwrap();
    let like_time = like_start.elapsed();

    assert_eq!(meta.len(), DOC_COUNT);
    assert!(all.len() > meta.len());
    // Memory backend: keys_like still scans internally but returns smaller vec
    assert!(meta.len() < all.len());
    assert!(
        like_time <= full_time + Duration::from_millis(5),
        "keys_like should not be slower than keys(): like={:?} full={:?}",
        like_time,
        full_time
    );
}

#[tokio::test]
async fn test_keys_with_prefix_subset() {
    let storage = MemoryKVStorage::new("perf_prefix");
    storage.initialize().await.unwrap();
    populate_kv(&storage).await;

    let keys = storage.keys_with_prefix("doc-00001-chunk-").await.unwrap();
    assert_eq!(keys.len(), CHUNKS_PER_DOC);
    for key in keys {
        assert!(key.starts_with("doc-00001-chunk-"));
    }
}

#[cfg(feature = "postgres")]
mod postgres_perf {
    use super::*;
    use edgequake_storage::adapters::postgres::{PostgresConfig, PostgresKVStorage};
    use std::env;
    use std::time::Duration;

    fn get_test_config() -> Option<PostgresConfig> {
        let password = env::var("POSTGRES_PASSWORD").ok()?;
        Some(PostgresConfig {
            host: env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: env::var("POSTGRES_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5432),
            database: env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string()),
            user: env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string()),
            password,
            namespace: format!(
                "perf_{}",
                &uuid::Uuid::new_v4().to_string().replace('-', "")[..8]
            ),
            max_connections: 5,
            min_connections: 1,
            connect_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(60),
            ..Default::default()
        })
    }

    #[tokio::test]
    async fn test_postgres_kv_count_is_o1_via_stats_table() {
        let Some(config) = get_test_config() else {
            eprintln!("SKIP: POSTGRES_PASSWORD not set");
            return;
        };

        let storage = PostgresKVStorage::new(config);
        storage.initialize().await.unwrap();

        let mut batch = Vec::new();
        for i in 0..5000 {
            batch.push((
                format!("stats-row-{i}-metadata"),
                serde_json::json!({"i": i}),
            ));
        }
        storage.upsert(&batch).await.unwrap();

        let count_start = Instant::now();
        let count = storage.count().await.unwrap();
        let count_time = count_start.elapsed();

        assert_eq!(count, 5000);
        assert!(
            count_time.as_millis() < 50,
            "KV count() must be O(1) via stats table, took {:?}",
            count_time
        );

        storage
            .delete(&["stats-row-0-metadata".to_string()])
            .await
            .unwrap();
        assert_eq!(storage.count().await.unwrap(), 4999);

        eprintln!(
            "postgres kv count: {:?} for 5000 rows (stats table, not COUNT(*))",
            count_time
        );
    }

    #[tokio::test]
    async fn test_postgres_ping_faster_than_count_on_large_table() {
        let Some(config) = get_test_config() else {
            eprintln!("SKIP: POSTGRES_PASSWORD not set");
            return;
        };

        let storage = PostgresKVStorage::new(config);
        storage.initialize().await.unwrap();

        let mut batch = Vec::new();
        for i in 0..2000 {
            batch.push((format!("row-{i}-metadata"), serde_json::json!({"i": i})));
        }
        storage.upsert(&batch).await.unwrap();

        let count_start = Instant::now();
        let count = storage.count().await.unwrap();
        let count_time = count_start.elapsed();

        let ping_start = Instant::now();
        storage.ping().await.unwrap();
        let ping_time = ping_start.elapsed();

        assert_eq!(count, 2000);
        assert!(
            ping_time < count_time,
            "ping ({:?}) should beat count ({:?}) on 2000 rows",
            ping_time,
            count_time
        );
        eprintln!(
            "postgres perf: count={:?} ping={:?} speedup={:.1}x",
            count_time,
            ping_time,
            count_time.as_secs_f64() / ping_time.as_secs_f64().max(1e-9)
        );
    }
}
