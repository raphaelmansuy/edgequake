# From Research Paper to Production System: The Story Behind EdgeQuake

_Why we built a Rust implementation of an algorithm we didn't invent_

---

## The Paper That Started Everything

In October 2024, I read a paper that changed how I thought about RAG systems.

"LightRAG: Simple and Fast Retrieval-Augmented Generation" by Guo, Xia, Yu, Ao, and Huang at the Hong Kong University of Data Science. The core insight was elegant: traditional RAG treats documents as bags of chunks, but relationships matter. If Sarah works at TechCorp and TechCorp is headquartered in Austin, a query about "where Sarah works" should find Austin—even if those facts are in different documents.

Their solution: build a knowledge graph during ingestion. Extract entities and relationships. Then search both the graph and the vectors.

I ran their Python implementation. It worked. Graph-enhanced answers were more coherent than flat RAG.

Then I tried to deploy it.

---

## The Production Gap

LightRAG is research software. That's not a criticism—it's designed for experimentation, for notebooks, for proving that graph-enhanced RAG works. It does that job brilliantly.

But production has different requirements.

**Storage**: LightRAG uses Neo4j for graphs, Pinecone for vectors, Redis for caching, and JSON files for metadata. Four systems. Four backup targets. Four monitoring dashboards. Four things to debug when something breaks at 3am.

**Operations**: No health endpoints. No connection pooling. No graceful shutdown. No cost tracking. No multi-tenancy. These aren't optional features—they're table stakes for anything running in Kubernetes.

**Deployment**: The Python runtime, with its dependencies and memory footprint, makes containerization... interesting. Package conflicts are a rite of passage.

I had a choice: spend 3-6 months building production patterns around LightRAG, or implement the algorithm in a language designed for production workloads.

---

## Why Rust?

I didn't choose Rust because "Rust is fast." (It is, but the LLM is the bottleneck—language speed barely matters.)

I chose Rust because:

**Single binary deployment**. No runtime, no dependencies. One Docker image that does one thing.

**Memory safety without garbage collection**. Predictable latency. No GC pauses during query processing.

**Async from the ground up**. Tokio handles concurrent connections naturally. No GIL to work around.

**Ecosystem maturity**. SQLx for database access. Axum for HTTP. Serde for serialization. The libraries exist and they're production-tested.

**PostgreSQL simplicity**. With Apache AGE for graphs and pgvector for embeddings, I could use one database instead of four. One connection string. One backup target. One system to understand.

---

## Building EdgeQuake

The algorithm implementation was straightforward. The paper is well-written, and the LightRAG codebase is readable. Entity extraction, relationship mapping, graph construction, dual-level retrieval—these translated cleanly to Rust.

The work was in what surrounded the algorithm.

**Health endpoints**: Three endpoints for Kubernetes probes. Liveness (is the process alive?). Readiness (is it ready for traffic?). Health (what's the component-level status?). Fifteen minutes to implement. Essential for zero-downtime deploys.

**Connection pooling**: SQLx provides this, but configuration matters. Max connections, acquire timeouts, minimum pool size. The defaults work; the configurability prevents edge-case failures.

**Multi-tenancy**: PostgreSQL Row-Level Security enforces tenant isolation at the database level. Not application logic that might have bugs—database enforcement that can't be bypassed.

**Cost tracking**: Per-document, per-operation cost breakdowns. When you're processing 10,000 documents, knowing that extraction costs 60% and embedding costs 10% tells you where to optimize.

**Graceful shutdown**: Drain connections, complete in-flight requests, then exit. Essential for rolling deployments.

None of this is innovative. It's infrastructure engineering. But it's the difference between "this demo works" and "this system runs in production."

---

## The Comparison

I want to be clear: LightRAG and EdgeQuake aren't competitors. They're for different stages.

| Aspect          | LightRAG              | EdgeQuake             |
| --------------- | --------------------- | --------------------- |
| **Purpose**     | Research, prototyping | Production deployment |
| **Language**    | Python                | Rust                  |
| **Storage**     | 4 systems             | 1 system              |
| **Query Modes** | 3                     | 6                     |
| **Operations**  | DIY                   | Built-in              |
| **Ideal User**  | ML researcher         | DevOps team           |

**Use LightRAG when** you're proving that graph-RAG works for your use case. When you're in a notebook. When you're iterating quickly in Python.

**Use EdgeQuake when** you're deploying to Kubernetes. When you need multi-tenancy. When your ops team wants one database to manage. When you need cost visibility day one.

---

## What I Learned

Building EdgeQuake taught me that algorithms are the easy part.

The LightRAG paper is ~15 pages. The algorithm implementation is maybe 2,000 lines of Rust. The surrounding infrastructure—health checks, pooling, tenancy, streaming, deployment patterns—is 10,000+ lines.

This ratio isn't unique to graph-RAG. It's true for most production systems. The core logic is a small fraction of the deployed code. The rest is everything that keeps it running.

I also learned that research and production need different tools. LightRAG isn't "bad" because it lacks health endpoints. It's research software doing research software things. EdgeQuake isn't "better" because it has Kubernetes probes. It's production software doing production software things.

The field needs both.

---

## Thank You

This project wouldn't exist without Guo, Xia, Yu, Ao, and Huang. Their paper provided the algorithm. Their codebase provided the reference implementation. EdgeQuake is a translation—from Python to Rust, from research to production—not an invention.

If you're evaluating graph-enhanced RAG, start with their paper. Run their implementation. Validate that it improves your queries. Then, if you need production patterns, EdgeQuake is there.

---

**Links**:

- LightRAG Paper: [arXiv:2410.05779](https://arxiv.org/abs/2410.05779)
- LightRAG: [github.com/HKUDS/LightRAG](https://github.com/HKUDS/LightRAG)
- EdgeQuake: [github.com/raphaelmansuy/edgequake](https://github.com/raphaelmansuy/edgequake)
