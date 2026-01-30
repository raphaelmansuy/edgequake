# [D] Building Resilient RAG Pipelines: Why Partial Success Beats Total Failure

**TL;DR**: Traditional RAG pipelines fail completely when any single chunk fails. Map-reduce pattern with per-chunk retry gives you 99/100 success instead of 0/100. Open source implementation in Rust.

---

## The Problem

We've all been there. Long-running document processing job fails at 3am. Single chunk timeout. Entire batch discarded.

With fail-fast semantics:

- 100 chunks in document
- Chunk 47 times out (LLM API blip)
- 46 successful extractions: gone
- LLM costs: charged anyway
- Retry: re-process everything from scratch

This isn't an edge case at scale. It's the expected failure mode.

## The Architecture

Instead of sequential fail-fast, we use map-reduce:

```
MAP PHASE:
  - Process all chunks in parallel
  - Each chunk has own timeout (60s)
  - Each chunk has own retry budget (3 attempts)
  - Semaphore controls concurrency (16 max)

REDUCE PHASE:
  - Partition into successes and failures
  - Merge successes into knowledge graph
  - Report failures with full details
  - Calculate success rate: 98/100 = 98% (not 0%)
```

Key insight: chunks are independent. Why should chunk 47 failing destroy chunks 1-46?

## Implementation Details

**Rust with Tokio async:**

```rust
let semaphore = Arc::new(Semaphore::new(16));

let futures: Vec<_> = chunks.iter().enumerate()
    .map(|(idx, chunk)| {
        let sem = semaphore.clone();
        async move {
            let _permit = sem.acquire().await;

            for attempt in 1..=3 {
                match timeout(Duration::from_secs(60),
                              extract(chunk)).await {
                    Ok(Ok(result)) => return Success(idx, result),
                    _ => sleep_exponential(attempt).await,
                }
            }
            Failed(idx, error_details)
        }
    })
    .collect();

let results = join_all(futures).await;
let (successes, failures) = partition(results);
```

**Real-time progress streaming:**

```rust
struct ChunkProgressUpdate {
    chunk_index: usize,
    total_chunks: usize,
    processing_time_ms: u64,
    cumulative_cost_usd: f64,
    eta_seconds: u64,
}
```

UI gets live updates. Users see exactly where processing is. Cost limits can abort early.

**Lineage tracking:**

Every entity traces back to:

- Source document ID
- Source chunk ID
- Line numbers in original document
- Extraction timestamp and model

Enables cascade delete: remove document → remove all derived entities.

## Benchmarks

100-page technical document (200 chunks):

| Metric       | Value         |
| ------------ | ------------- |
| Throughput   | 33 chunks/sec |
| Success rate | 98.5%         |
| Cost         | $0.034        |
| Memory       | 45MB peak     |

Compare to fail-fast: 0% success on any single failure, same cost, no visibility.

## Stack

- Rust + Tokio (async runtime)
- PostgreSQL + Apache AGE (graph queries) + pgvector (embeddings)
- LightRAG algorithm (arXiv:2410.05779)

All in one database. No multi-DB sync nightmares.

## Try It

```bash
git clone https://github.com/your-org/edgequake
cd edgequake
make dev  # Full stack in 30 seconds
```

Open source, production-ready.

---

**Discussion questions:**

1. What chunk sizes work best for your domain? We default to 1200 tokens with 100 token overlap.

2. How do you handle multi-language documents? Current implementation assumes single language per document.

3. What's your retry strategy for rate limits? We use exponential backoff (1s → 2s → 4s) but interested in token bucket approaches.

Would love to hear what others are doing for document processing reliability.
