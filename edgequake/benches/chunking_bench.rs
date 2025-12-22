//! Chunking performance benchmarks.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use edgequake_pipeline::chunker::{Chunker, ChunkerConfig};

fn create_test_text(size: usize) -> String {
    let sentence = "This is a test sentence for benchmarking text chunking performance. ";
    sentence.repeat(size / sentence.len() + 1)[..size].to_string()
}

fn bench_chunking_small(c: &mut Criterion) {
    let config = ChunkerConfig::default();
    let chunker = Chunker::new(config);
    let text = create_test_text(500); // ~500 bytes

    c.bench_function("chunk_small_text", |b| {
        b.iter(|| chunker.chunk(black_box(&text), black_box("doc-1")))
    });
}

fn bench_chunking_medium(c: &mut Criterion) {
    let config = ChunkerConfig::default();
    let chunker = Chunker::new(config);
    let text = create_test_text(10_000); // ~10KB

    let mut group = c.benchmark_group("chunking_medium");
    group.throughput(Throughput::Bytes(text.len() as u64));

    group.bench_function("chunk_10kb", |b| {
        b.iter(|| chunker.chunk(black_box(&text), black_box("doc-1")))
    });

    group.finish();
}

fn bench_chunking_large(c: &mut Criterion) {
    let config = ChunkerConfig::default();
    let chunker = Chunker::new(config);
    let text = create_test_text(100_000); // ~100KB

    let mut group = c.benchmark_group("chunking_large");
    group.throughput(Throughput::Bytes(text.len() as u64));
    group.sample_size(50); // Reduce sample size for large inputs

    group.bench_function("chunk_100kb", |b| {
        b.iter(|| chunker.chunk(black_box(&text), black_box("doc-1")))
    });

    group.finish();
}

fn bench_chunking_config_variations(c: &mut Criterion) {
    let text = create_test_text(20_000);

    let mut group = c.benchmark_group("chunking_configs");

    // Default config
    let default_chunker = Chunker::new(ChunkerConfig::default());
    group.bench_function("default_config", |b| {
        b.iter(|| default_chunker.chunk(black_box(&text), black_box("doc-1")))
    });

    // Small chunks
    let small_config = ChunkerConfig {
        chunk_size: 256,
        chunk_overlap: 32,
        ..Default::default()
    };
    let small_chunker = Chunker::new(small_config);
    group.bench_function("small_chunks", |b| {
        b.iter(|| small_chunker.chunk(black_box(&text), black_box("doc-1")))
    });

    // Large chunks
    let large_config = ChunkerConfig {
        chunk_size: 2048,
        chunk_overlap: 256,
        ..Default::default()
    };
    let large_chunker = Chunker::new(large_config);
    group.bench_function("large_chunks", |b| {
        b.iter(|| large_chunker.chunk(black_box(&text), black_box("doc-1")))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_chunking_small,
    bench_chunking_medium,
    bench_chunking_large,
    bench_chunking_config_variations
);
criterion_main!(benches);
