# Building Resilient RAG Pipelines: A Map-Reduce Approach

_How EdgeQuake processes documents at scale without losing successful extractions to single failures_

---

## The 3am Production Incident

It was 3am when my phone buzzed. The overnight batch processing job—2,000 documents, 8 hours of work—had failed at document 1,847. The error? A single chunk timed out calling the LLM API.

The worst part? We lost everything. All 1,846 successfully processed documents. All the entities extracted. All the relationships discovered. Gone.

This wasn't a bug in our code. It was a fundamental architectural flaw: **fail-fast pipelines don't belong in production RAG systems**.

---

## Why Traditional Pipelines Fail at Scale

Most RAG document processing pipelines follow a simple pattern:

```
Document → Chunks → Extract Entities → Store → Done
```

Looks clean. Works great for demos. But here's what happens at scale:

```
╔═══════════════════════════════════════════════════════════════╗
║                   THE FAIL-FAST ANTI-PATTERN                 ║
╠═══════════════════════════════════════════════════════════════╣
║                                                               ║
║   Document (100 chunks)                                       ║
║        │                                                      ║
║        ▼                                                      ║
║   ┌────────────────────────────────────────────┐             ║
║   │ Process: 1...2...3...47...TIMEOUT!         │             ║
║   │                                            │             ║
║   │ Result: COMPLETE FAILURE                   │             ║
║   │ - 46 successful extractions: DISCARDED     │             ║
║   │ - Processing time: WASTED                  │             ║
║   │ - LLM costs: CHARGED ANYWAY                │             ║
║   └────────────────────────────────────────────┘             ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
```

The problems compound at scale:

- **Transient failures are inevitable**: Network blips, rate limits, API timeouts
- **Retry granularity is wrong**: Retrying the entire document wastes work
- **No visibility**: You don't know what succeeded before the failure
- **Cost blindness**: LLM tokens were consumed but results were discarded

---

## The Map-Reduce Solution

EdgeQuake reimagines document processing as a **map-reduce operation**:

```
╔═══════════════════════════════════════════════════════════════╗
║                        MAP PHASE                              ║
║    (Parallel chunk processing with per-chunk resilience)      ║
╠═══════════════════════════════════════════════════════════════╣
║                                                               ║
║   Document (N chunks)                                         ║
║        │                                                      ║
║        ▼                                                      ║
║   ┌────┬────┬────┬────┬────┐                                 ║
║   │ C1 │ C2 │ C3 │ C4 │ CN │   (chunks to workers)           ║
║   └─┬──┴─┬──┴─┬──┴─┬──┴─┬──┘                                 ║
║     │    │    │    │    │                                     ║
║     ▼    ▼    ▼    ▼    ▼      (parallel LLM calls)          ║
║   ┌───┐┌───┐┌───┐┌───┐┌───┐                                  ║
║   │ ✓ ││ ✗ ││ ✓ ││ ✓ ││ ✓ │    (each has own retry)         ║
║   └───┘└───┘└───┘└───┘└───┘                                  ║
║                                                               ║
╠═══════════════════════════════════════════════════════════════╣
║                       REDUCE PHASE                            ║
║    (Aggregate successes and failures with full reporting)     ║
╠═══════════════════════════════════════════════════════════════╣
║                                                               ║
║   All outcomes collected                                      ║
║        │                                                      ║
║        ▼                                                      ║
║   ┌────────────────────────────────────────────────────────┐ ║
║   │  Partition:                                            │ ║
║   │    - successes = [C1, C3, C4, CN]  → merge to graph   │ ║
║   │    - failures = [C2] → detailed error report          │ ║
║   │                                                        │ ║
║   │  Result: 4/5 = 80% success (NOT 0% failure!)          │ ║
║   └────────────────────────────────────────────────────────┘ ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
```

The key insight: **partial success is better than total failure**.

---

## Implementation Deep Dive

### 1. Semaphore-Controlled Concurrency

EdgeQuake uses Tokio's async semaphore for backpressure control:

```rust
pub struct Pipeline {
    config: PipelineConfig,
    chunker: Chunker,
    extractor: Option<Arc<dyn EntityExtractor>>,
}

impl Pipeline {
    async fn resilient_extract_parallel(
        &self,
        chunks: &[TextChunk],
        extractor: &Arc<dyn EntityExtractor>,
    ) -> ResilientExtractionResult {
        // Semaphore limits concurrent LLM calls
        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            self.config.max_concurrent_extractions,  // default: 16
        ));

        // Process all chunks in parallel, respecting limits
        let futures: Vec<_> = chunks.iter()
            .enumerate()
            .map(|(idx, chunk)| {
                let permit = semaphore.clone();
                async move {
                    let _guard = permit.acquire().await;
                    extract_with_retry(chunk, idx).await
                }
            })
            .collect();

        // Collect ALL results, don't short-circuit on errors
        stream::iter(futures)
            .buffer_unordered(self.config.max_concurrent_extractions)
            .collect()
            .await
    }
}
```

Why a semaphore?

- **Rate limit protection**: LLM APIs have concurrent request limits
- **Resource management**: Memory scales with concurrent extractions
- **Fair scheduling**: Every chunk gets equal opportunity

### 2. Per-Chunk Retry with Exponential Backoff

Each chunk has its own retry budget:

```rust
// Per-chunk retry loop
for attempt in 1..=max_retries {  // default: 3 attempts
    let timeout = Duration::from_secs(timeout_secs);  // default: 60s

    match tokio::time::timeout(timeout, extractor.extract(&chunk)).await {
        Ok(Ok(result)) => return ChunkOutcome::Success(result),
        Ok(Err(e)) => {
            // LLM error - maybe rate limit, maybe parse error
            last_error = e;
            was_timeout = false;
        }
        Err(_) => {
            // Timeout - LLM hung or network issue
            last_error = "Timeout after 60s";
            was_timeout = true;
        }
    }

    // Exponential backoff: 1s → 2s → 4s
    if attempt < max_retries {
        let delay = initial_delay_ms * 2u64.pow(attempt - 1);
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }
}

return ChunkOutcome::Failed(ChunkFailure {
    chunk_index: idx,
    error: last_error,
    retry_attempts: max_retries,
    was_timeout,
});
```

### 3. Real-Time Progress Tracking

No more black box processing:

```rust
// Callback invoked after each chunk completes
pub type ChunkProgressCallback =
    Arc<dyn Fn(ChunkProgressUpdate) + Send + Sync>;

pub struct ChunkProgressUpdate {
    pub chunk_index: usize,         // Which chunk just completed
    pub total_chunks: usize,        // Total in document
    pub processing_time_ms: u64,    // Time for this chunk
    pub input_tokens: usize,        // Tokens consumed
    pub output_tokens: usize,       // Tokens generated
    pub chunk_cost_usd: f64,        // Cost for this chunk
    pub cumulative_cost_usd: f64,   // Running total
    pub eta_seconds: u64,           // Time remaining estimate
}
```

This enables:

- **Live progress bars** in your UI
- **Cost monitoring** before the job completes
- **Early termination** if costs exceed budget
- **SLA alerting** if ETA exceeds expectations

---

## Chunking: The Unsung Hero

Before extraction, documents must be split into processable chunks. EdgeQuake provides four strategies:

```
╔═══════════════════════════════════════════════════════════════╗
║                    CHUNKING STRATEGIES                        ║
╠═══════════════════════════════════════════════════════════════╣
║                                                               ║
║  1. CharacterBasedChunking                                   ║
║     └─ Simple split by character count                        ║
║     └─ Fast, predictable sizes                               ║
║                                                               ║
║  2. TokenBasedChunking (default)                             ║
║     └─ Split respecting LLM token limits                     ║
║     └─ Prevents context window overflow                      ║
║                                                               ║
║  3. SentenceBoundaryChunking                                 ║
║     └─ Never splits mid-sentence                             ║
║     └─ Better extraction quality                             ║
║                                                               ║
║  4. ParagraphBoundaryChunking                                ║
║     └─ Respects document structure                           ║
║     └─ Best for well-formatted documents                     ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
```

Default configuration enforces key constraints:

- **1200 tokens** per chunk (fits in LLM context with headroom)
- **100 tokens overlap** (~8%) ensures entities at boundaries aren't missed
- **Preserve sentences** when possible for extraction quality

---

## The Merge Phase: Building Knowledge Over Time

When processing multiple documents, entities often appear across chunks:

```rust
/// Merge strategy: Combine, don't replace
///
/// When entity "SARAH_CHEN" appears in 3 documents:
///
/// Document 1: "Sarah Chen is a software engineer"
/// Document 2: "Dr. Chen leads the ML team"
/// Document 3: "Sarah Chen, PhD, Stanford '15"
///
/// Merged result:
/// {
///   name: "SARAH_CHEN",
///   description: "Software engineer and ML team lead.
///                 PhD from Stanford University, class of 2015.",
///   source_ids: ["doc1_chunk2", "doc2_chunk1", "doc3_chunk5"]
/// }
```

This strategy:

- **Builds richer descriptions** over time
- **Maintains full provenance** for citation
- **Enables cascade delete** via source_id filtering

---

## Production Patterns

### Multi-Tenant Isolation

```rust
impl Pipeline {
    pub fn with_tenant_context(
        mut self,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Self {
        self.tenant_id = tenant_id;
        self.workspace_id = workspace_id;
        self
    }
}
```

Every extraction includes tenant context, enabling:

- Row-level security in PostgreSQL
- Separate cost tracking per tenant
- Data isolation guarantees

### Lineage Tracking

Full traceability from document to entity:

```rust
DocumentLineage {
    document_id: "doc_123",
    chunks: [
        ChunkLineage {
            chunk_id: "chunk_456",
            line_range: (1, 50),
            entities: ["SARAH_CHEN", "EDGEQUAKE"],
        }
    ],
    extraction_metadata: {
        model: "gpt-4o-mini",
        timestamp: "2024-01-15T10:30:00Z",
        total_cost_usd: 0.0034,
    }
}
```

---

## Performance Benchmarks

On a 100-page technical document (200 chunks):

| Metric                 | Value            |
| ---------------------- | ---------------- |
| Concurrent extractions | 16               |
| Average chunk time     | 450ms            |
| Total processing time  | ~6 seconds       |
| Throughput             | 33 chunks/second |
| Success rate           | 98.5%            |
| Cost                   | $0.034           |

With fail-fast (old approach):

- **0% success** when any chunk fails
- **No visibility** into what completed
- **Same cost** but no value

---

## Try EdgeQuake

EdgeQuake is open source and production-ready:

```bash
# Clone and run
git clone https://github.com/your-org/edgequake
cd edgequake

# Start full stack (PostgreSQL + Backend + Frontend)
make dev

# Process your first document
curl -X POST http://localhost:3000/api/documents \
  -F "file=@your-document.pdf"
```

The pipeline architecture ensures your documents get processed reliably, with full visibility into progress and costs.

---

## Acknowledgments

EdgeQuake implements the LightRAG algorithm (arXiv:2410.05779) by Guo et al. We thank the research team for their groundbreaking work on graph-enhanced RAG systems.

---

_What document processing challenges have you faced? Share your experiences in the comments._

**GitHub**: [EdgeQuake Repository](https://github.com/your-org/edgequake)
**Paper**: [LightRAG: Simple and Fast Retrieval-Augmented Generation](https://arxiv.org/abs/2410.05779)
