# X.com Thread: The EdgeQuake Approach

## Tweet 1 (Hook)

Graph-RAG is the future.

But every implementation I've seen is:
• A research prototype
• Python with GIL limitations
• Complex to deploy

So we built EdgeQuake.

Production-ready. Rust. Open source.

Here's how it works 🧵

---

## Tweet 2

EdgeQuake transforms documents into knowledge graphs in 3 stages:

1. INGEST (chunk + extract)
2. STORE (graph + vectors)
3. QUERY (5 modes)

Let's break each one down.

---

## Tweet 3

STAGE 1: INGEST

Document enters the system:
→ Preprocessing (text extraction, encoding)
→ Adaptive chunking (600-1200 tokens)
→ LLM extraction (entities + relationships)

The secret sauce? Tuple-delimited output.

---

## Tweet 4

Why tuples instead of JSON?

JSON parsing with LLMs is fragile.
One missing bracket = total failure.

Tuples are line-by-line:

```
entity<|#|>SARAH_CHEN<|#|>PERSON<|#|>Researcher
relation<|#|>SARAH<|#|>MIT<|#|>works_at<|#|>...
<|COMPLETE|>
```

Partial recovery. No escaping nightmares.

---

## Tweet 5

STAGE 2: STORE

Extracted entities + relationships go to PostgreSQL.

But not just any Postgres:
• Apache AGE for graph (nodes + edges)
• pgvector for embeddings

One database. Graph + Vector. No sync issues.

---

## Tweet 6

Why PostgreSQL instead of Neo4j + Pinecone?

1. Single database = no sync problems
2. ACID guarantees
3. Battle-tested at scale
4. Your team already knows SQL

Apache AGE gives you Cypher queries.
pgvector gives you similarity search.

Best of both worlds.

---

## Tweet 7

STAGE 3: QUERY

This is where EdgeQuake shines.

5 query modes:
• Naive (~50ms) - pure vector
• Local (~150ms) - entity + neighbors
• Global (~200ms) - community summaries
• Hybrid (~250ms) - local + global
• Mix (~300ms) - weighted fusion

---

## Tweet 8

When to use each mode:

Naive: "Find documents about X" (fastest)
Local: "Who is Sarah Chen?" (entity-centric)
Global: "Main themes across 50 docs?" (holistic)
Hybrid: "How does Sarah's work relate to Y?" (complex)
Mix: Custom balance for your use case

---

## Tweet 9

Performance numbers:

| Metric            | EdgeQuake |
| ----------------- | --------- |
| Query latency     | <200ms    |
| Concurrent users  | 1000+     |
| Memory per doc    | 2MB       |
| Entity extraction | 2-3x more |

Built in Rust for a reason.

---

## Tweet 10

The architecture:

```
┌─────────────────┐
│ edgequake-api   │ REST (Axum)
├─────────────────┤
│ edgequake-core  │ Orchestration
├─────────────────┤
│ edgequake-pipeline │ Extraction
├─────────────────┤
│ edgequake-query │ 5 Modes
├─────────────────┤
│ edgequake-storage │ Graph+Vector
└─────────────────┘
```

Modular. Testable. Maintainable.

---

## Tweet 11

Why Rust?

• Zero-cost abstractions
• Memory safety without GC
• True parallelism (no GIL)
• Async/await with Tokio

Python Graph-RAG: ~500ms queries
EdgeQuake: ~200ms queries

2.5x faster. Same accuracy.

---

## Tweet 12

Getting started is 3 commands:

```
git clone github.com/raphaelmansuy/edgequake
make install
make dev
```

Opens at localhost:3000.

Upload a document. See it become a graph.

---

## Tweet 13

What's included:

✅ REST API (OpenAPI 3.0)
✅ SSE streaming responses
✅ React 19 frontend
✅ Graph visualization
✅ Multi-tenant support
✅ Docker-ready

Production features from day one.

---

## Tweet 14

EdgeQuake is open source.

→ Star: github.com/raphaelmansuy/edgequake
→ Docs: Full deep-dive documentation
→ Discuss: GitHub Discussions open

We're building the fastest Graph-RAG framework.

Join us.

---

## Tweet 15 (Repost Hook)

Graph-RAG is the future.

EdgeQuake is how you get there—today.

🔄 Repost if you're building with knowledge graphs
