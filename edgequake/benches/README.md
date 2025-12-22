# EdgeQuake Benchmarks

This directory contains performance benchmarks for EdgeQuake components.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench chunking_bench
cargo bench --bench query_bench
cargo bench --bench storage_bench
```

## Benchmarks

### Chunking Benchmark (`chunking_bench.rs`)

Measures text chunking performance:
- Small text (< 1KB)
- Medium text (~10KB)
- Large text (~100KB)

### Query Benchmark (`query_bench.rs`)

Measures query strategy performance:
- Naive (vector-only) strategy
- Local strategy with graph lookup
- Hybrid strategy combining vector and graph

### Storage Benchmark (`storage_bench.rs`)

Measures storage operations:
- Vector upsert and query
- Graph node and edge operations
- KV storage operations

## Interpreting Results

Criterion outputs statistics including:
- Mean execution time
- Standard deviation
- Throughput (for iterative benchmarks)

Results are saved in `target/criterion/` with HTML reports.

## Environment

Benchmarks use in-memory storage to measure pure algorithm performance.
For production-like benchmarks, configure external storage backends.
