# [D] Multi-Mode Query Engine: Matching Retrieval Strategy to Question Type

**TL;DR**: Vector similarity fails for relationship questions. We built a query engine with 5 modes (Naive, Local, Global, Hybrid, Mix) that matches retrieval strategy to question type. Hybrid mode is 35% better quality than vector-only on relationship queries.

---

## The Problem

Most RAG implementations use a single retrieval strategy: embed query → vector search → top-k chunks → LLM.

Works for: "What is our refund policy?"
Fails for: "How does Alice collaborate with Bob?"

Why? Relationship questions require traversing connections between entities, not just finding similar text chunks.

## The Modes

**Naive (~50ms)**

- Pure vector similarity on chunks
- Best for factual lookups
- 40% of queries can use this

**Local (~150ms)**

- Entity-centric graph traversal
- Find ALICE node → traverse to BOB → explore shared connections
- Best for "Who works with whom?"

**Global (~200ms)**

- Community-based retrieval
- High-level themes and patterns
- Best for "What are the main challenges?"

**Hybrid (~250ms)** - DEFAULT

- Runs Local + Global in parallel
- Merges results
- Best for complex/unknown query types

**Mix (configurable)**

- Weighted blend: α×Naive + β×Graph
- Full control for domain tuning

## LightRAG Implementation

We follow the LightRAG paper (arXiv:2410.05779):

1. **Keyword Extraction**
   - LLM extracts high-level (themes) and low-level (entities) keywords
   - High-level → Global mode retrieval
   - Low-level → Local mode retrieval

2. **Token Budgeting**
   - Graph context gets 70% priority (pre-summarized, higher signal)
   - Raw chunks get 30%
   - Never exceed LLM context window

3. **Caching**
   - Keyword extraction cached 24h
   - 70-90% hit rate on similar queries
   - Massive cost reduction

## Benchmarks

1,000 queries with human evaluation:

| Mode   | Latency (p50) | Quality |
| ------ | ------------- | ------- |
| Naive  | 48ms          | 6.2/10  |
| Local  | 142ms         | 7.8/10  |
| Global | 195ms         | 7.5/10  |
| Hybrid | 245ms         | 8.5/10  |

Hybrid is 5x slower but 35% better quality.

For relationship-heavy domains (org charts, project dependencies, knowledge bases), Hybrid is clearly worth the latency.

## Stack

- Rust + Tokio async
- PostgreSQL + Apache AGE (Cypher graph queries)
- pgvector (embeddings)
- Optional: BM25 or cross-encoder reranking

## Open Source

```bash
git clone https://github.com/your-org/edgequake
cd edgequake
make dev
```

Implements LightRAG (arXiv:2410.05779) with production-ready multi-mode query engine.

---

**Discussion questions:**

1. How do you currently select retrieval strategy for different query types?

2. Anyone tried adaptive mode selection (LLM classifies query intent)?

3. For those using graph-enhanced RAG: what's your experience with graph depth? We default to 2 hops.
