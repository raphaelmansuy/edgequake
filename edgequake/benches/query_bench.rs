//! Query strategy performance benchmarks.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::runtime::Runtime;

use edgequake_query::modes::QueryMode;
use edgequake_query::strategies::{create_strategy, NaiveStrategy, QueryStrategy, StrategyConfig};
use edgequake_storage::adapters::memory::{MemoryGraphStorage, MemoryVectorStorage};
use edgequake_storage::traits::{GraphStorage, VectorStorage};
use serde_json::json;
use std::collections::HashMap;

fn create_runtime() -> Runtime {
    Runtime::new().unwrap()
}

async fn setup_vector_storage(num_vectors: usize) -> Arc<MemoryVectorStorage> {
    let storage = Arc::new(MemoryVectorStorage::new("bench", 384));
    storage.initialize().await.unwrap();

    // Insert test vectors
    let mut data = Vec::with_capacity(num_vectors);
    for i in 0..num_vectors {
        let embedding: Vec<f32> = (0..384).map(|j| ((i + j) as f32 / 1000.0).sin()).collect();
        let metadata = json!({
            "content": format!("This is test document number {} with some content for benchmarking.", i),
            "source": format!("doc-{}", i)
        });
        data.push((format!("vec-{}", i), embedding, metadata));
    }
    storage.upsert(&data).await.unwrap();

    storage
}

async fn setup_graph_storage(num_nodes: usize, num_edges: usize) -> Arc<MemoryGraphStorage> {
    let storage = Arc::new(MemoryGraphStorage::new("bench"));
    storage.initialize().await.unwrap();

    // Insert test nodes
    for i in 0..num_nodes {
        let mut props: HashMap<String, serde_json::Value> = HashMap::new();
        props.insert("entity_type".to_string(), json!("ENTITY"));
        props.insert(
            "description".to_string(),
            json!(format!("Entity {} description", i)),
        );
        storage
            .upsert_node(&format!("entity-{}", i), props)
            .await
            .unwrap();
    }

    // Insert test edges (random connections)
    for i in 0..num_edges {
        let src = format!("entity-{}", i % num_nodes);
        let tgt = format!("entity-{}", (i * 7 + 3) % num_nodes);
        let mut props: HashMap<String, serde_json::Value> = HashMap::new();
        props.insert("weight".to_string(), json!(1.0));
        props.insert("description".to_string(), json!("related to"));
        storage.upsert_edge(&src, &tgt, props).await.unwrap();
    }

    storage
}

fn bench_naive_strategy(c: &mut Criterion) {
    let rt = create_runtime();
    let vector_storage = rt.block_on(setup_vector_storage(100));

    let strategy = NaiveStrategy::new(vector_storage);
    let config = StrategyConfig::default();
    let query_embedding: Vec<f32> = (0..384).map(|i| (i as f32 / 500.0).cos()).collect();

    c.bench_function("naive_strategy_100_vectors", |b| {
        b.iter(|| {
            rt.block_on(async {
                strategy
                    .execute(
                        black_box("test query"),
                        black_box(&query_embedding),
                        black_box(&config),
                    )
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_naive_strategy_large(c: &mut Criterion) {
    let rt = create_runtime();
    let vector_storage = rt.block_on(setup_vector_storage(1000));

    let strategy = NaiveStrategy::new(vector_storage);
    let config = StrategyConfig::default();
    let query_embedding: Vec<f32> = (0..384).map(|i| (i as f32 / 500.0).cos()).collect();

    let mut group = c.benchmark_group("naive_strategy_scaling");
    group.sample_size(50);

    group.bench_function("1000_vectors", |b| {
        b.iter(|| {
            rt.block_on(async {
                strategy
                    .execute(
                        black_box("test query"),
                        black_box(&query_embedding),
                        black_box(&config),
                    )
                    .await
                    .unwrap()
            })
        })
    });

    group.finish();
}

fn bench_strategy_factory(c: &mut Criterion) {
    let rt = create_runtime();
    let vector_storage = rt.block_on(setup_vector_storage(100));
    let graph_storage = rt.block_on(setup_graph_storage(50, 100));

    let config = StrategyConfig::default();
    let query_embedding: Vec<f32> = (0..384).map(|i| (i as f32 / 500.0).cos()).collect();

    let mut group = c.benchmark_group("query_strategies");

    // Naive strategy
    let naive = create_strategy(
        QueryMode::Naive,
        vector_storage.clone(),
        graph_storage.clone(),
    );
    group.bench_function("naive", |b| {
        b.iter(|| {
            rt.block_on(async {
                naive
                    .execute(
                        black_box("test query"),
                        black_box(&query_embedding),
                        black_box(&config),
                    )
                    .await
                    .unwrap()
            })
        })
    });

    // Local strategy
    let local = create_strategy(
        QueryMode::Local,
        vector_storage.clone(),
        graph_storage.clone(),
    );
    group.bench_function("local", |b| {
        b.iter(|| {
            rt.block_on(async {
                local
                    .execute(
                        black_box("test query"),
                        black_box(&query_embedding),
                        black_box(&config),
                    )
                    .await
                    .unwrap()
            })
        })
    });

    // Global strategy
    let global = create_strategy(
        QueryMode::Global,
        vector_storage.clone(),
        graph_storage.clone(),
    );
    group.bench_function("global", |b| {
        b.iter(|| {
            rt.block_on(async {
                global
                    .execute(
                        black_box("test query"),
                        black_box(&query_embedding),
                        black_box(&config),
                    )
                    .await
                    .unwrap()
            })
        })
    });

    // Hybrid strategy
    let hybrid = create_strategy(
        QueryMode::Hybrid,
        vector_storage.clone(),
        graph_storage.clone(),
    );
    group.bench_function("hybrid", |b| {
        b.iter(|| {
            rt.block_on(async {
                hybrid
                    .execute(
                        black_box("test query"),
                        black_box(&query_embedding),
                        black_box(&config),
                    )
                    .await
                    .unwrap()
            })
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_naive_strategy,
    bench_naive_strategy_large,
    bench_strategy_factory
);
criterion_main!(benches);
