# Show HN: EdgeQuake – Graph-RAG in Rust (5x faster than Python)

**HackerNews Post**

---

## Title

Show HN: EdgeQuake – Graph-RAG in Rust (5x faster than Python)

## URL

https://github.com/raphaelmansuy/edgequake

## Text

Hey HN,

I've been building EdgeQuake, a Rust implementation of Graph-RAG (LightRAG algorithm, arXiv:2410.05779). The decision to use Rust instead of Python was initially about performance, but it's turned out to be about much more than that.

**Why Rust for RAG?**

Most RAG systems use Python (LangChain, LlamaIndex). Python is great for prototyping, but we kept hitting walls in production:

1. **GIL limitations** — asyncio helps I/O, but CPU-bound work (text processing, parsing) blocks everything
2. **Memory overhead** — ~8MB per document in Python, ~2MB in Rust
3. **GC pauses** — 50-200ms garbage collection pauses kill P99 latency
4. **Cold start** — 500ms+ to import torch + framework

**The Architecture:**

Built on Tokio for async. The key pattern is concurrent extraction with backpressure:

```rust
let semaphore = Arc::new(Semaphore::new(16));

let results = stream::iter(chunks)
    .map(|chunk| {
        let sem = semaphore.clone();
        let ext = extractor.clone();
        async move {
            let _permit = sem.acquire().await?;
            ext.extract(&chunk).await
        }
    })
    .buffer_unordered(16)
    .collect()
    .await;
```

The semaphore provides backpressure for LLM rate limits. Lock-free atomic counters track progress across concurrent tasks without mutex overhead.

**Benchmarks (vs equivalent Python):**

| Metric           | Python  | Rust   | Improvement |
| ---------------- | ------- | ------ | ----------- |
| Query latency    | ~1000ms | <200ms | 5x          |
| Concurrent users | ~100    | 1000+  | 10x         |
| Memory per doc   | ~8MB    | 2MB    | 4x          |
| Cold start       | ~500ms  | <10ms  | 50x         |

A friend's startup went from 47 Python containers to 4 Rust instances. AWS bill dropped from $28k to $4k/month.

**Trade-offs (being honest):**

- Learning curve is real — 2-4 weeks to internalize ownership
- Compile times are long (~3 min clean build)
- Smaller ecosystem — we call Python via FFI for specialized ML models
- Hiring is harder — fewer Rust developers than Python

**When Python is still the right choice:**

- Prototyping and experimentation
- <100 concurrent users
- Team without Rust experience and no time to invest

**Stack:**

- Rust + Tokio for async
- PostgreSQL + Apache AGE for graph (Cypher queries)
- pgvector for embeddings
- React 19 frontend

**Getting Started:**

```bash
git clone https://github.com/raphaelmansuy/edgequake
make dev  # Starts PostgreSQL + Backend + Frontend
```

Would love feedback from anyone running RAG in production. What's your experience with Python at scale? Anyone else made the Rust jump?

---

## HN Comment Preparation

**Q: Why not just use multiprocessing in Python?**
A: Works for some workloads, but adds IPC overhead. Each process needs its own memory space, so you can't share model weights. For RAG with embeddings, this means loading the same 500MB model multiple times.

**Q: What about PyO3/maturin for hybrid approach?**
A: We do this for specialized models. But the core pipeline benefits from Rust's memory model throughout. PyO3 at boundaries adds some overhead.

**Q: How do you handle the Rust learning curve for the team?**
A: About 2-4 weeks of friction, then productivity returns. The compiler errors are frustrating but educational. We've had zero runtime null pointer exceptions or memory bugs since switching.

**Q: Why not Go instead of Rust?**
A: Go's GC, while better than Python's, still has pause times. Rust's zero-cost abstractions and no-GC model give more predictable latency. For RAG specifically, memory efficiency matters more than Go's simpler syntax.

**Q: Compile times are a real concern. How do you deal with it?**
A: Incremental builds are fast (~10s). We use cargo-watch for hot reload during development. Full clean builds (~3 min) only happen in CI or after major dependency updates.
