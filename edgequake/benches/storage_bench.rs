//! Storage operations performance benchmarks.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

use edgequake_storage::adapters::memory::{
    MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
};
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage};
use serde_json::json;

fn create_runtime() -> Runtime {
    Runtime::new().unwrap()
}

fn bench_vector_upsert(c: &mut Criterion) {
    let rt = create_runtime();

    let mut group = c.benchmark_group("vector_upsert");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single_vector", |b| {
        b.iter(|| {
            rt.block_on(async {
                let storage = MemoryVectorStorage::new("bench", 384);
                storage.initialize().await.unwrap();

                let embedding: Vec<f32> = (0..384).map(|i| i as f32 / 1000.0).collect();
                let data = vec![("vec-1".to_string(), embedding, json!({"content": "test"}))];
                storage.upsert(black_box(&data)).await.unwrap();
            })
        })
    });

    group.finish();
}

fn bench_vector_upsert_batch(c: &mut Criterion) {
    let rt = create_runtime();

    let mut group = c.benchmark_group("vector_batch_upsert");

    for batch_size in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));

        group.bench_function(format!("batch_{}", batch_size), |b| {
            b.iter(|| {
                rt.block_on(async {
                    let storage = MemoryVectorStorage::new("bench", 384);
                    storage.initialize().await.unwrap();

                    let data: Vec<_> = (0..*batch_size)
                        .map(|i| {
                            let embedding: Vec<f32> =
                                (0..384).map(|j| ((i + j) as f32 / 1000.0).sin()).collect();
                            (
                                format!("vec-{}", i),
                                embedding,
                                json!({"content": format!("test {}", i)}),
                            )
                        })
                        .collect();

                    storage.upsert(black_box(&data)).await.unwrap();
                })
            })
        });
    }

    group.finish();
}

fn bench_vector_query(c: &mut Criterion) {
    let rt = create_runtime();

    // Setup storage with data
    let storage = rt.block_on(async {
        let storage = Arc::new(MemoryVectorStorage::new("bench", 384));
        storage.initialize().await.unwrap();

        let data: Vec<_> = (0..500)
            .map(|i| {
                let embedding: Vec<f32> =
                    (0..384).map(|j| ((i + j) as f32 / 1000.0).sin()).collect();
                (
                    format!("vec-{}", i),
                    embedding,
                    json!({"content": format!("document {}", i)}),
                )
            })
            .collect();
        storage.upsert(&data).await.unwrap();
        storage
    });

    let query_embedding: Vec<f32> = (0..384).map(|i| (i as f32 / 500.0).cos()).collect();

    let mut group = c.benchmark_group("vector_query");

    for top_k in [5, 10, 20].iter() {
        group.bench_function(format!("top_{}", top_k), |b| {
            b.iter(|| {
                rt.block_on(async {
                    storage
                        .query(
                            black_box(&query_embedding),
                            black_box(*top_k),
                            black_box(None),
                        )
                        .await
                        .unwrap()
                })
            })
        });
    }

    group.finish();
}

fn bench_graph_operations(c: &mut Criterion) {
    let rt = create_runtime();

    let mut group = c.benchmark_group("graph_operations");

    group.bench_function("upsert_node", |b| {
        b.iter(|| {
            rt.block_on(async {
                let storage = MemoryGraphStorage::new("bench");
                storage.initialize().await.unwrap();

                let mut props: HashMap<String, serde_json::Value> = HashMap::new();
                props.insert("type".to_string(), json!("ENTITY"));
                props.insert("description".to_string(), json!("Test entity"));

                storage
                    .upsert_node(black_box("entity-1"), props)
                    .await
                    .unwrap();
            })
        })
    });

    group.bench_function("upsert_edge", |b| {
        b.iter(|| {
            rt.block_on(async {
                let storage = MemoryGraphStorage::new("bench");
                storage.initialize().await.unwrap();

                let mut node_props: HashMap<String, serde_json::Value> = HashMap::new();
                node_props.insert("type".to_string(), json!("ENTITY"));
                storage
                    .upsert_node("entity-1", node_props.clone())
                    .await
                    .unwrap();
                storage.upsert_node("entity-2", node_props).await.unwrap();

                let mut edge_props: HashMap<String, serde_json::Value> = HashMap::new();
                edge_props.insert("weight".to_string(), json!(1.0));

                storage
                    .upsert_edge(black_box("entity-1"), black_box("entity-2"), edge_props)
                    .await
                    .unwrap();
            })
        })
    });

    group.finish();
}

fn bench_graph_traversal(c: &mut Criterion) {
    let rt = create_runtime();

    // Setup graph
    let storage = rt.block_on(async {
        let storage = Arc::new(MemoryGraphStorage::new("bench"));
        storage.initialize().await.unwrap();

        // Create a graph with 100 nodes and 200 edges
        for i in 0..100 {
            let mut props: HashMap<String, serde_json::Value> = HashMap::new();
            props.insert("type".to_string(), json!("ENTITY"));
            storage
                .upsert_node(&format!("entity-{}", i), props)
                .await
                .unwrap();
        }

        for i in 0..200 {
            let src = format!("entity-{}", i % 100);
            let tgt = format!("entity-{}", (i * 7 + 13) % 100);
            let mut props: HashMap<String, serde_json::Value> = HashMap::new();
            props.insert("weight".to_string(), json!(1.0));
            storage.upsert_edge(&src, &tgt, props).await.unwrap();
        }

        storage
    });

    let mut group = c.benchmark_group("graph_traversal");

    group.bench_function("get_node", |b| {
        b.iter(|| rt.block_on(async { storage.get_node(black_box("entity-50")).await.unwrap() }))
    });

    group.bench_function("get_neighbors", |b| {
        b.iter(|| {
            rt.block_on(async {
                storage
                    .get_neighbors(black_box("entity-50"), 1)
                    .await
                    .unwrap()
            })
        })
    });

    group.bench_function("get_all_nodes", |b| {
        b.iter(|| rt.block_on(async { storage.get_all_nodes().await.unwrap() }))
    });

    group.finish();
}

fn bench_kv_operations(c: &mut Criterion) {
    let rt = create_runtime();

    let mut group = c.benchmark_group("kv_operations");

    group.bench_function("upsert_single", |b| {
        b.iter(|| {
            rt.block_on(async {
                let storage = MemoryKVStorage::new("bench");
                storage.initialize().await.unwrap();

                let data = vec![("key-1".to_string(), json!({"content": "test value"}))];
                storage.upsert(black_box(&data)).await.unwrap();
            })
        })
    });

    group.bench_function("get_single", |b| {
        let storage = rt.block_on(async {
            let storage = Arc::new(MemoryKVStorage::new("bench"));
            storage.initialize().await.unwrap();

            let data = vec![("key-1".to_string(), json!({"content": "test value"}))];
            storage.upsert(&data).await.unwrap();
            storage
        });

        b.iter(|| rt.block_on(async { storage.get_by_id(black_box("key-1")).await.unwrap() }))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_vector_upsert,
    bench_vector_upsert_batch,
    bench_vector_query,
    bench_graph_operations,
    bench_graph_traversal,
    bench_kv_operations
);
criterion_main!(benches);
