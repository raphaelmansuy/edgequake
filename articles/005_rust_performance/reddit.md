# Reddit Posts for Article 005

## r/rust Post

**Title:** Built a Graph-RAG system in Rust — 5x faster than our Python prototype, here's what we learned

**Body:**

Hey rustaceans!

I spent the last 6 months building EdgeQuake, a Graph-RAG framework in Rust. We started with a Python prototype (like everyone does), then rewrote it when we hit production scaling issues.

**The Pain Points with Python RAG:**

1. GIL blocking CPU-bound text processing
2. 8MB memory per document (millions of chunks = $$)
3. 50-200ms GC pauses killing P99 latency
4. 500ms+ cold starts on every Kubernetes scale-up

**The Architecture:**

Tokio for async, with concurrent extraction using semaphore backpressure:

```rust
// Concurrent chunk extraction with rate limiting
let semaphore = Arc::new(tokio::sync::Semaphore::new(16));

let futures = chunks.iter().map(|chunk| {
    let sem = semaphore.clone();
    let extractor = extractor.clone();
    async move {
        let _permit = sem.acquire().await?;
        extractor.extract(&chunk).await
    }
});

let results: Vec<Result<ExtractionResult>> = stream::iter(futures)
    .buffer_unordered(16)
    .collect()
    .await;
```

Lock-free progress tracking:

```rust
let cumulative_tokens = Arc::new(AtomicU64::new(0));

// In each parallel task:
cumulative_tokens.fetch_add(tokens, Ordering::Relaxed);
```

**Results:**

| Metric           | Python  | Rust   | Δ   |
| ---------------- | ------- | ------ | --- |
| Query latency    | ~1000ms | <200ms | 5x  |
| Concurrent users | ~100    | 1000+  | 10x |
| Memory/doc       | 8MB     | 2MB    | 4x  |
| Cold start       | 500ms   | <10ms  | 50x |

A startup we advised went from 47 Python containers to 4 Rust instances. AWS bill: $28k → $4k/month.

**What I Learned:**

1. `Arc<T>` is your friend for sharing across tasks
2. Atomic counters beat mutexes for simple metrics
3. `buffer_unordered` is perfect for rate-limited APIs
4. Criterion for benchmarks — don't guess at performance
5. The borrow checker hurts for ~2 weeks, then becomes helpful

**Trade-offs:**

- Compile times are real (~3 min clean build)
- Smaller ML ecosystem (we FFI to Python for some models)
- Hiring is harder

**Code:** https://github.com/raphaelmansuy/edgequake

Curious what patterns others use for high-concurrency Rust services!

---

## r/Python Post

**Title:** Why we rewrote our Python RAG system in Rust (and when you shouldn't)

**Body:**

Hey r/Python — I come in peace! 🕊️

I love Python. I've been writing it for 15 years. But last year we hit a wall with our RAG system and ended up rewriting the core in Rust.

**NOT here to bash Python** — I want to share what we learned so you can make informed decisions.

**When Python is absolutely fine:**

- Prototyping and experimentation (nothing beats it)
- <100 concurrent users
- P99 latency requirements >2s
- Team doesn't have time to learn Rust

**When we hit problems:**

Our RAG system processes documents → chunks → LLM extraction → embeddings → storage. At ~50 concurrent users, things got ugly:

1. **GIL**: asyncio helped I/O, but text processing blocked everything
2. **Memory**: 8MB per document × thousands of docs = expensive RAM
3. **GC pauses**: 50-200ms unpredictable latency spikes
4. **Cold start**: 500ms+ every time Kubernetes scaled up

**What we did:**

Rewrote the core pipeline in Rust. Kept Python for experimentation and specialized models (via PyO3).

**Results:**

| Metric           | Python  | Rust   |
| ---------------- | ------- | ------ |
| Query latency    | ~1000ms | <200ms |
| Concurrent users | ~100    | 1000+  |
| Memory/doc       | 8MB     | 2MB    |
| Cold start       | 500ms   | <10ms  |

**Would I do it again?**

For this use case, yes. But it's not always the right call. The learning curve is 2-4 weeks of pain. Compile times are long. The ecosystem is smaller.

**Hybrid approach:**

If you're curious about Rust but don't want to rewrite everything:

1. Use `maturin` + PyO3 to write performance-critical functions in Rust
2. Call them from Python like normal functions
3. Get 10-100x speedups on specific bottlenecks

**Code:** https://github.com/raphaelmansuy/edgequake

Happy to answer questions about when this makes sense (or doesn't)!

---

## r/MachineLearning Post

**Title:** [P] Graph-RAG in Rust: 5x faster than Python with 4x less memory

**Body:**

**TL;DR:** Rewrote a Graph-RAG system from Python to Rust. Query latency: 5x faster. Memory: 4x less. Concurrent users: 10x more.

**Motivation:**

We implemented the LightRAG algorithm (arXiv:2410.05779) for production use. Python prototype worked great until ~50 concurrent users. Then:

- GIL blocked CPU-bound text processing
- 8MB memory per document
- GC pauses (50-200ms) killed P99 latency
- 500ms+ cold starts on scale-up

**Architecture:**

- Tokio async runtime (work-stealing, no GIL)
- Concurrent extraction with semaphore backpressure
- Lock-free atomic counters (no mutex overhead)
- Zero-copy data sharing with Arc<T>

**Key code pattern:**

```rust
let semaphore = Arc::new(Semaphore::new(16));

let results = stream::iter(chunks)
    .map(|chunk| async {
        let _permit = semaphore.acquire().await?;
        extractor.extract(&chunk).await
    })
    .buffer_unordered(16)
    .collect()
    .await;
```

**Benchmarks:**

| Metric           | Python  | Rust   | Improvement |
| ---------------- | ------- | ------ | ----------- |
| Query latency    | ~1000ms | <200ms | 5x          |
| Concurrent users | ~100    | 1000+  | 10x         |
| Memory per doc   | 8MB     | 2MB    | 4x          |
| Cold start       | ~500ms  | <10ms  | 50x         |

**Production impact:**

Friend's startup: 47 Python containers → 4 Rust instances
AWS bill: $28k/month → $4k/month (85% reduction)

**Trade-offs:**

- Learning curve: 2-4 weeks
- Longer compile times
- Smaller ML ecosystem (we use PyO3/FFI for some models)

**Code:** https://github.com/raphaelmansuy/edgequake

Paper: LightRAG (arXiv:2410.05779) — thanks to the authors for the algorithm.
