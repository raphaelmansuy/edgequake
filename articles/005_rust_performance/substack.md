# From Python to Rust: Why We Rewrote Our RAG System

_The story of a 3 AM phone call, 47 containers, and an 85% reduction in cloud costs_

---

Dear Reader,

Let me tell you about the phone call that changed how I think about building AI systems.

It was 3 AM. My friend Alex — CTO of an AI startup building document intelligence — sounded exhausted.

"We're running 47 Python containers," he said. "CPU is at 90%. We added autoscaling but now our AWS bill is $28,000 a month. We haven't even launched yet."

I'd heard variations of this story before. Python RAG systems work beautifully on a laptop with 10 documents. They scale to maybe 50 concurrent users. Then everything breaks.

Two months later, Alex's team had rewritten their core pipeline in Rust. Same functionality. Same throughput.

**New AWS bill: $4,200/month.**

This newsletter is about why that happened, and whether it makes sense for you.

---

## The Uncomfortable Truth About Python RAG

I've been writing Python for 15 years. I love the language. It's the best prototyping tool ever created.

But Python has fundamental limitations that become walls in production RAG:

**Wall 1: The Global Interpreter Lock (GIL)**

Only one thread executes Python bytecode at a time. `asyncio` helps with I/O-bound work (waiting for API responses), but the moment you do CPU-bound work (text processing, chunking, parsing), everything blocks.

RAG systems do both. Constantly.

**Wall 2: Memory Overhead**

Every Python object carries metadata. An integer is 28 bytes. A small dictionary is 232+ bytes. When you're processing millions of text chunks, this adds up fast.

We measured: Python RAG uses about **8MB per document**. That's RAM you're paying for.

**Wall 3: Garbage Collection**

Python's garbage collector runs periodically. For large heaps (common when you're holding embeddings in memory), GC pauses can hit **50-200 milliseconds**.

You can't control when this happens. Your P99 latency becomes unpredictable.

**Wall 4: Cold Start**

Every time Kubernetes scales up, every time a Lambda function starts, your users wait:

- Python interpreter startup: ~200ms
- Import torch: ~300ms
- Import LangChain: ~150ms

That's 500ms-2s before your code runs. Every. Single. Time.

---

## Why Rust Changes Everything

Rust eliminates all four walls. Not by being "faster Python" but by being fundamentally different.

**Tokio: True Async**

Rust's Tokio runtime is a work-stealing scheduler. Tasks run concurrently across all CPU cores. There's no GIL. CPU-bound and I/O-bound work execute simultaneously.

One Rust process handles what 10+ Python containers struggle with.

**Zero-Copy Memory**

Rust's ownership model means data gets copied only when explicitly requested. We use `Arc<T>` (atomic reference counting) to share data across concurrent tasks without copying.

EdgeQuake uses **2MB per document** — 4x less than Python.

**No Garbage Collector**

Memory is freed the moment a value goes out of scope. Deterministic. Predictable. Your P99 latency stays close to your P50.

**Single Binary**

Rust compiles to a single ~15MB executable. No interpreter. No dependencies. No virtual environment.

Cold start: **<10ms**.

---

## The Code That Made It Work

Here's the pattern that handles concurrent LLM extraction:

```rust
// Create semaphore for backpressure
let semaphore = Arc::new(tokio::sync::Semaphore::new(16));

// Map phase: process chunks concurrently
let futures = chunks.iter().map(|chunk| {
    let semaphore = semaphore.clone();
    let extractor = extractor.clone();
    async move {
        // Acquire permit (waits if 16 tasks already running)
        let _permit = semaphore.acquire().await?;
        extractor.extract(&chunk).await
    }
});

// Execute with controlled concurrency
let results = stream::iter(futures)
    .buffer_unordered(16)  // Max 16 in-flight
    .collect()
    .await;
```

The semaphore provides backpressure — we don't overwhelm LLM rate limits. The `buffer_unordered` maintains up to 16 concurrent tasks. Memory stays bounded. Latency stays predictable.

---

## The Numbers

After rewriting:

| Metric           | Python  | Rust   | Improvement |
| ---------------- | ------- | ------ | ----------- |
| Query Latency    | ~1000ms | <200ms | 5x faster   |
| Concurrent Users | ~100    | 1000+  | 10x more    |
| Memory per Doc   | 8MB     | 2MB    | 4x less     |
| Cold Start       | ~500ms  | <10ms  | 50x faster  |

At scale:

| Monthly Users | Python Instances | Rust Instances | Cost Savings |
| ------------- | ---------------- | -------------- | ------------ |
| 10,000        | 4                | 1              | 75%          |
| 100,000       | 40               | 4              | 90%          |
| 1,000,000     | 400+             | 40             | 90%          |

Alex's $28k/month dropped to $4k/month. Same throughput. Fewer support tickets about latency.

---

## The Honest Trade-offs

I'm not going to pretend Rust is always the answer.

**Learning Curve**: The ownership model takes 2-4 weeks to internalize. The compiler will fight you until you understand borrowing. It's frustrating but educational.

**Compile Times**: Large Rust projects have long compile times. EdgeQuake takes about 3 minutes for a clean build, 10 seconds for incremental changes.

**Smaller Ecosystem**: Python has more ML libraries. For specialized models, we call Python via FFI.

**Hiring**: There are fewer Rust developers than Python developers. You'll pay more, or train internally.

---

## When Python is Still Right

Don't rewrite in Rust if:

- You're prototyping (Python is unbeatable here)
- You have <100 concurrent users
- Latency requirements are relaxed (>2s P99 is fine)
- Your team can't invest 2-4 weeks in learning Rust

Do consider Rust if:

- You're hitting scaling walls
- Cloud costs are becoming a problem
- Latency predictability matters
- You're building something you'll maintain for years

---

## The Hybrid Approach

If you're curious but not ready for a full rewrite:

1. Use `maturin` + PyO3 to write performance-critical functions in Rust
2. Call them from Python like normal functions
3. Get 10-100x speedups on specific bottlenecks

```python
# Python code calling Rust
from edgequake_rust import process_chunk

result = process_chunk(text)  # 10x faster than pure Python
```

Best of both worlds. Prototype in Python. Optimize in Rust.

---

## What's Next

This was about the "why Rust" decision. Next week, I'll dive into **EdgeQuake's query engine** — how we combine graph traversal with vector similarity in a single query.

If you're building RAG systems and hitting scaling issues, I'd love to hear about your experience. Reply to this email.

Until next week,

_Raphael_

---

_EdgeQuake is open source: [github.com/raphaelmansuy/edgequake](https://github.com/raphaelmansuy/edgequake)_

_Thanks to the Tokio team for building async Rust, and to the broader Rust community for proving systems programming can be both safe and ergonomic._

_LightRAG paper: arXiv:2410.05779_
