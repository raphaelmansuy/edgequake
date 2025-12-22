# Performance Baselines

**Date Established**: 2025-12-21  
**System**: macOS (Apple Silicon)  
**Rust Version**: 1.78+

## Chunking Performance

| Benchmark      | Time    | Throughput  | Notes                     |
| -------------- | ------- | ----------- | ------------------------- |
| chunk_1kb      | ~680 ns | ~1.37 GiB/s | Small document chunking   |
| chunk_10kb     | ~2.6 µs | ~3.58 GiB/s | Medium document chunking  |
| chunk_100kb    | ~27 µs  | ~3.49 GiB/s | Large document chunking   |
| default_config | ~5.0 µs | -           | Default chunking settings |
| small_chunks   | ~8.1 µs | -           | 256-token chunks          |
| large_chunks   | ~4.0 µs | -           | 2048-token chunks         |

## Vector Storage Operations

| Benchmark               | Time    | Throughput   | Notes                    |
| ----------------------- | ------- | ------------ | ------------------------ |
| single_vector (384 dim) | ~930 ns | ~1.08M vec/s | Single vector upsert     |
| batch_10                | -       | ~1.5M vec/s  | 10 vectors per batch     |
| batch_50                | -       | ~2.0M vec/s  | 50 vectors per batch     |
| batch_100               | -       | ~2.0M vec/s  | 100 vectors per batch    |
| query_top5              | -       | -            | Top-5 similarity search  |
| query_top10             | -       | -            | Top-10 similarity search |
| query_top20             | -       | -            | Top-20 similarity search |

## Graph Storage Operations

| Benchmark     | Time     | Notes                          |
| ------------- | -------- | ------------------------------ |
| upsert_node   | ~322 ns  | Node insertion with properties |
| upsert_edge   | ~784 ns  | Edge insertion with properties |
| get_node      | ~136 ns  | Single node lookup             |
| get_neighbors | ~1.88 µs | Neighbor traversal (depth=1)   |
| get_all_nodes | ~8.2 µs  | Full graph scan (100 nodes)    |

## KV Storage Operations

| Benchmark     | Time    | Notes                   |
| ------------- | ------- | ----------------------- |
| upsert_single | ~305 ns | Single key-value upsert |
| get_single    | ~117 ns | Single key-value lookup |

## How to Run Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench --bench chunking_bench
cargo bench --bench storage_bench
cargo bench --bench query_bench

# Run with filtering
cargo bench -- "vector"
cargo bench -- "graph"
```

## Performance Goals

- **Chunking**: > 1 GiB/s throughput for all document sizes
- **Vector Upsert**: > 1M vectors/s for batch operations
- **Graph Operations**: < 1 µs for single-node operations
- **KV Operations**: < 500 ns for single-key operations

## Monitoring Regressions

Criterion automatically compares against previous runs and reports:

- **Change within noise threshold**: No significant change
- **Performance has improved**: Faster than baseline
- **Performance has regressed**: Slower than baseline

Reports are saved in `target/criterion/` for historical comparison.
