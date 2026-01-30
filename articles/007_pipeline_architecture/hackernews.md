# Building Resilient RAG Pipelines: A Map-Reduce Approach

**Show HN: EdgeQuake - Production-ready document processing with partial success handling**

---

Most RAG document processing pipelines use a fail-fast approach: one chunk fails, entire document fails. After losing 1,800+ successfully processed documents to a single timeout, I rebuilt the architecture using a map-reduce pattern.

## The Problem

Traditional flow:

```
Document → Chunk → Extract → Store
```

Sounds reasonable until:

- Chunk 47 of 100 times out (LLM API hiccup)
- All 46 successful extractions discarded
- LLM costs charged, zero value retained
- Retry means re-processing everything

At scale, transient failures are inevitable. Rate limits, network blips, context window errors. Fail-fast is incompatible with production reliability.

## The Solution: Map-Reduce for Documents

```
MAP PHASE:
┌────┬────┬────┬────┬────┐
│ C1 │ C2 │ C3 │ C4 │ C5 │
└─┬──┴─┬──┴─┬──┴─┬──┴─┬──┘
  ▼    ▼    ▼    ▼    ▼
┌───┐┌───┐┌───┐┌───┐┌───┐
│ ✓ ││ ✗ ││ ✓ ││ ✓ ││ ✓ │
└───┘└───┘└───┘└───┘└───┘

REDUCE PHASE:
  successes: [C1, C3, C4, C5] → merge to graph
  failures: [C2] → detailed error report
  result: 80% success (not 0%)
```

Key implementation decisions:

**1. Semaphore-controlled concurrency**

```rust
let semaphore = Semaphore::new(16);
let futures = chunks.iter().map(|chunk| {
    let permit = semaphore.clone();
    async move {
        let _guard = permit.acquire().await;
        extract_with_retry(chunk).await
    }
});
```

16 concurrent LLM calls by default. Prevents rate limiting without under-utilizing.

**2. Per-chunk retry with exponential backoff**

```rust
for attempt in 1..=3 {
    match timeout(60s, extract(chunk)).await {
        Ok(Ok(result)) => return Success(result),
        _ => sleep(1000ms * 2^(attempt-1)).await,
    }
}
return Failed(error_details)
```

Transient failures (rate limits) recover. Permanent failures (malformed content) fail fast after 3 attempts.

**3. Real-time progress callbacks**

```rust
struct ChunkProgressUpdate {
    chunk_index: usize,
    total_chunks: usize,
    eta_seconds: u64,
    cumulative_cost_usd: f64,
}
```

No more black box processing. Stream progress to UI, enable cost limits, alert on SLA violations.

## Tradeoffs and Decisions

**Why not retry at document level?**

A 100-chunk document failing at chunk 99 would waste 99 successful extractions. Chunk-level retry is more efficient and provides better visibility.

**Why 60-second timeout?**

Most LLM extractions complete in <2 seconds. 60 seconds catches hangs without aggressive timeouts killing slow-but-valid responses. Configurable per deployment.

**Why accumulate failures instead of discarding?**

Detailed failure reports enable:

- Targeted retry of failed chunks only
- Pattern detection (all failures from same document section?)
- Cost attribution (failed chunks still consumed tokens)

## Performance

100-page technical document (200 chunks):

- Throughput: 33 chunks/second
- Success rate: 98.5%
- Total cost: $0.034
- Full lineage tracking preserved

The fail-fast alternative: 0% success on any single failure, same cost.

## Stack

- Rust + Tokio for async concurrency
- PostgreSQL + Apache AGE (graph) + pgvector (embeddings)
- Implements LightRAG algorithm (arXiv:2410.05779)

Open source: https://github.com/your-org/edgequake

---

Happy to discuss implementation details, alternative approaches, or tradeoffs you'd make differently.
