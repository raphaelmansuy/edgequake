/// Performance benchmarks for SOTA graph query optimizations
///
/// These benchmarks validate that our SQL CTE optimizations achieve
/// the target performance goals:
/// - node_degree: <50ms
/// - node_degrees_batch: <100ms for 100 nodes
/// - get_popular_nodes_with_degree: <100ms for 1000 nodes
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use edgequake_storage::adapters::memory::MemoryGraphStorage;
use edgequake_storage::traits::GraphStorage;
use std::collections::HashMap;
use tokio::runtime::Runtime;

/// Create test graph with specified number of nodes
async fn setup_benchmark_graph(storage: &impl GraphStorage, node_count: usize) {
    // Create nodes
    for i in 0..node_count {
        let mut props = HashMap::new();
        props.insert(
            "node_id".to_string(),
            serde_json::json!(format!("NODE_{}", i)),
        );
        props.insert("entity_type".to_string(), serde_json::json!("test"));
        storage
            .upsert_node(&format!("NODE_{}", i), props)
            .await
            .unwrap();
    }

    // Create edges to form a connected graph
    // Each node connects to next 3 nodes (if they exist)
    for i in 0..node_count {
        for j in 1..=3 {
            let target = (i + j) % node_count;
            if target != i {
                storage
                    .upsert_edge(
                        &format!("NODE_{}", i),
                        &format!("NODE_{}", target),
                        HashMap::new(),
                    )
                    .await
                    .unwrap();
            }
        }
    }
}

fn bench_node_degree(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let storage = rt.block_on(async {
        let storage = MemoryGraphStorage::new("bench");
        storage.initialize().await.unwrap();
        setup_benchmark_graph(&storage, 1000).await;
        storage
    });

    c.bench_function("node_degree_single", |b| {
        b.to_async(&rt).iter(|| async {
            let degree = storage.node_degree(black_box("NODE_500")).await.unwrap();
            black_box(degree);
        });
    });
}

fn bench_node_degrees_batch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let storage = rt.block_on(async {
        let storage = MemoryGraphStorage::new("bench");
        storage.initialize().await.unwrap();
        setup_benchmark_graph(&storage, 1000).await;
        storage
    });

    let mut group = c.benchmark_group("node_degrees_batch");

    for size in [10, 50, 100, 200].iter() {
        let node_ids: Vec<String> = (0..*size).map(|i| format!("NODE_{}", i)).collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &node_ids,
            |b, node_ids| {
                b.to_async(&rt).iter(|| async {
                    let degrees = storage
                        .node_degrees_batch(black_box(node_ids))
                        .await
                        .unwrap();
                    black_box(degrees);
                });
            },
        );
    }

    group.finish();
}

fn bench_get_popular_nodes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let storage = rt.block_on(async {
        let storage = MemoryGraphStorage::new("bench");
        storage.initialize().await.unwrap();
        setup_benchmark_graph(&storage, 1000).await;
        storage
    });

    let mut group = c.benchmark_group("get_popular_nodes_with_degree");

    for limit in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(limit), limit, |b, &limit| {
            b.to_async(&rt).iter(|| async {
                let nodes = storage
                    .get_popular_nodes_with_degree(black_box(limit), None, None, None, None)
                    .await
                    .unwrap();
                black_box(nodes);
            });
        });
    }

    group.finish();
}

fn bench_search_labels(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let storage = rt.block_on(async {
        let storage = MemoryGraphStorage::new("bench");
        storage.initialize().await.unwrap();
        setup_benchmark_graph(&storage, 1000).await;
        storage
    });

    c.bench_function("search_labels", |b| {
        b.to_async(&rt).iter(|| async {
            let labels = storage.search_labels(black_box("NODE"), 10).await.unwrap();
            black_box(labels);
        });
    });
}

fn bench_comparison_batch_vs_individual(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let storage = rt.block_on(async {
        let storage = MemoryGraphStorage::new("bench");
        storage.initialize().await.unwrap();
        setup_benchmark_graph(&storage, 500).await;
        storage
    });

    let node_ids: Vec<String> = (0..100).map(|i| format!("NODE_{}", i)).collect();

    let mut group = c.benchmark_group("batch_vs_individual");

    // Individual queries (N separate calls)
    group.bench_function("individual_100_queries", |b| {
        b.to_async(&rt).iter(|| async {
            let mut degrees = Vec::new();
            for node_id in &node_ids {
                let degree = storage.node_degree(black_box(node_id)).await.unwrap();
                degrees.push(degree);
            }
            black_box(degrees);
        });
    });

    // Batch query (single call)
    group.bench_function("batch_100_queries", |b| {
        b.to_async(&rt).iter(|| async {
            let degrees = storage
                .node_degrees_batch(black_box(&node_ids))
                .await
                .unwrap();
            black_box(degrees);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_node_degree,
    bench_node_degrees_batch,
    bench_get_popular_nodes,
    bench_search_labels,
    bench_comparison_batch_vs_individual
);

criterion_main!(benches);
