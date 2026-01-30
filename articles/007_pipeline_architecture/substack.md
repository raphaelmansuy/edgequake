# The 3am Production Incident That Changed How I Build RAG Pipelines

_Why partial success beats total failure, and how to implement it_

---

It was 3:17am when my phone buzzed with the alert that would change how I think about document processing forever.

The overnight batch job—2,000 documents, 8 hours of processing time, $340 in LLM costs—had failed. Not partially failed. Completely failed.

Document 1,847. Chunk 47. Timeout.

And with that single timeout, 1,846 successfully processed documents vanished. All the entities extracted. All the relationships discovered. All the embeddings generated. Gone.

I sat in bed, staring at the error log, trying to understand how a single chunk timeout could destroy eight hours of work.

That's when I realized: **our pipeline architecture was fundamentally broken**.

---

## The Fail-Fast Anti-Pattern

Like most RAG systems, our document processing pipeline was straightforward:

```
Document → Split into Chunks → Extract Entities → Store in Graph
```

Clean. Simple. Intuitive.

And catastrophically fragile.

The problem was our error handling philosophy: fail fast. The moment anything went wrong, we'd throw an exception and let it bubble up. The entire job would halt, "safely" discarding partial results.

"Safety" that cost us eight hours of work and $340 in API calls.

Here's the thing about LLM API calls at scale: **transient failures are inevitable**.

- Rate limits hit unexpectedly
- Network connections time out
- API responses occasionally malform
- Server-side errors happen

With fail-fast semantics, any of these—happening to any single chunk—destroys all progress. At 2,000 documents with an average of 50 chunks each, that's 100,000 opportunities for a transient failure to nuke your entire job.

The math doesn't work.

---

## The Map-Reduce Insight

The solution came from an unexpected place: Hadoop.

Not the technology itself (we're not processing petabytes here), but the conceptual model. Map-Reduce treats failures as normal:

- **Map phase**: Process each piece independently
- **Reduce phase**: Aggregate results, handle failures gracefully

Applied to document processing:

```
MAP PHASE:
  - Each chunk is processed independently
  - Each chunk has its own timeout
  - Each chunk has its own retry budget
  - Failures don't affect other chunks

REDUCE PHASE:
  - Collect all successful extractions
  - Collect all failures with details
  - Merge successes into knowledge graph
  - Report failures for investigation

Result: 99/100 chunks succeed = 99% success
        (not 0% because one failed)
```

This single architectural change transformed our reliability from "pray nothing fails" to "failures are expected and handled."

---

## Implementation: The Details That Matter

### Semaphore-Controlled Concurrency

The first challenge: how many chunks to process simultaneously?

Too few, and you waste time waiting. Too many, and you hit rate limits or exhaust memory.

We settled on a semaphore pattern:

```rust
let semaphore = Semaphore::new(16);

for chunk in chunks {
    let permit = semaphore.acquire().await;
    spawn(async move {
        // Process chunk
        // Permit automatically released on completion
    });
}
```

16 concurrent extractions by default. Enough to maximize throughput, few enough to stay under rate limits. Configurable per deployment based on your LLM provider's limits.

### Per-Chunk Retry with Exponential Backoff

When a chunk fails, we don't give up immediately:

```
Attempt 1: Try extraction (60s timeout)
           → Fail (rate limit)
Wait 1 second
Attempt 2: Try again
           → Fail (still rate limited)
Wait 2 seconds
Attempt 3: Final attempt
           → Fail
Mark as failed, continue processing other chunks
```

Three attempts with exponential backoff handles most transient failures. The 60-second timeout catches hung connections without prematurely killing slow-but-valid extractions.

### Real-Time Progress Tracking

No more black box processing. Every chunk completion triggers a callback:

```
Chunk 47/200 complete
  ├── Processing time: 450ms
  ├── Tokens used: 342 input, 156 output
  ├── Chunk cost: $0.00023
  ├── Cumulative cost: $0.0156
  └── ETA: 42 seconds remaining
```

This enables:

- Live progress bars in the UI
- Cost monitoring before job completes
- Early termination if costs exceed budget
- SLA alerting if ETA exceeds expectations

### Failure Reporting That Enables Action

When chunks fail, we don't just log "Error: timeout". We capture everything needed for investigation and retry:

```json
{
  "chunk_id": "doc_1847_chunk_47",
  "chunk_index": 47,
  "document_id": "doc_1847",
  "error_message": "Timeout after 60s",
  "was_timeout": true,
  "retry_attempts": 3,
  "processing_time_ms": 180042,
  "chunk_preview": "The quarterly results showed..."
}
```

With this detail, you can:

- Retry just the failed chunks (not the entire document)
- Identify patterns (all failures from one document section?)
- Debug root causes (always the same LLM model?)

---

## The Results

After rebuilding with this architecture, we reran our 2,000 document batch:

| Metric          | Before (Fail-Fast)         | After (Resilient)   |
| --------------- | -------------------------- | ------------------- |
| Success rate    | 0% (any failure kills all) | 98.7%               |
| Retry scope     | Entire job                 | Failed chunks only  |
| Visibility      | None until completion      | Real-time per-chunk |
| Cost tracking   | Post-hoc invoice           | Live monitoring     |
| Processing time | 8 hours (when successful)  | 7.5 hours           |

The 1.3% failed chunks? We investigated. Most were truly malformed content (OCR errors, corrupted text). A few were persistent API issues that resolved on manual retry the next morning.

The key: **99% of our work wasn't held hostage by 1% of failures**.

---

## Building This Today

We open-sourced this architecture as EdgeQuake:

```bash
git clone https://github.com/your-org/edgequake
cd edgequake
make dev  # Full stack running in 30 seconds
```

The pipeline implements the LightRAG algorithm (arXiv:2410.05779), with resilient extraction as a first-class concern.

Key components:

- Rust + Tokio for async concurrency
- PostgreSQL + Apache AGE for graph storage
- pgvector for embeddings
- Real-time progress streaming to UI

All in one database. No multi-system synchronization nightmares.

---

## Lessons Learned

**Fail-fast is a lie at scale.** What feels safe (stop immediately on error) becomes a liability when failures are inevitable.

**Granularity matters.** Retry at the smallest independent unit. Chunks are independent; documents are not.

**Visibility enables reliability.** You can't improve what you can't see. Real-time progress tracking transformed how we debug and operate.

**Partial success is success.** 99% of a document is infinitely more valuable than 0% of a document.

That 3am incident sucked. But it forced us to build something better.

Now when I see "98.7% success rate" on a batch job, I know exactly what that means: thousands of entities extracted, relationships discovered, knowledge captured. And a small, manageable list of failures to investigate—without losing any of the successes.

---

_Have you dealt with similar document processing challenges? I'd love to hear your approaches. Reply or reach out directly._

**GitHub**: [EdgeQuake Repository](https://github.com/your-org/edgequake)
**Paper**: [LightRAG: Simple and Fast Retrieval-Augmented Generation](https://arxiv.org/abs/2410.05779)
