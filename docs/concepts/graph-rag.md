# Graph-RAG: The Foundation

> **Graph-RAG enhances Retrieval-Augmented Generation by using knowledge graphs
> to capture relationships that vector search alone misses.**

---

## What is Graph-RAG?

Graph-RAG is an architecture pattern that combines:

1. **Retrieval-Augmented Generation (RAG)**: Using external documents to ground LLM responses
2. **Knowledge Graphs**: Storing entities and their relationships as nodes and edges

The key insight: **relationships between entities are as important as the entities themselves**.

---

## The Problem with Traditional RAG

Traditional RAG uses vector similarity to find relevant text chunks:

```
┌─────────────────────────────────────────────────────────────────┐
│                    TRADITIONAL RAG                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│   Documents ──▶ Chunks ──▶ Embeddings ──▶ Vector Database        │
│                                                                   │
│   Query ──▶ Embedding ──▶ Similarity Search ──▶ Top-K Chunks    │
│                                                                   │
│   Problem: Chunks are ISOLATED. No relationships captured.       │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

**Example failure:**

Query: _"How did Sarah's research influence Bob's work?"_

Traditional RAG returns:

- Chunk 1: "Sarah published a paper on neural networks..."
- Chunk 2: "Bob's latest project uses deep learning..."

But it **cannot connect** that Bob's work was **based on** Sarah's research, because:

- The relationship isn't in either chunk
- Vector similarity doesn't understand causation

---

## How Graphs Solve It

Graphs explicitly model relationships:

```
┌─────────────────────────────────────────────────────────────────┐
│                    KNOWLEDGE GRAPH                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│          ┌─────────────┐         ┌─────────────┐                 │
│          │   SARAH     │         │     BOB     │                 │
│          │  (PERSON)   │         │   (PERSON)  │                 │
│          └──────┬──────┘         └──────┬──────┘                 │
│                 │                       │                        │
│      ┌──────────┴───────────────────────┴──────────┐            │
│      │                                              │            │
│      v                                              v            │
│  ┌───────────┐                              ┌───────────────┐   │
│  │ PUBLISHED │                              │   BASED_ON    │   │
│  └─────┬─────┘                              └───────┬───────┘   │
│        │                                            │            │
│        v                                            v            │
│  ┌───────────────────┐                    ┌─────────────────┐   │
│  │ NEURAL_NETWORKS   │◀───────────────────│  BOB'S_PROJECT  │   │
│  │    (CONCEPT)      │   uses_concepts    │    (PROJECT)    │   │
│  └───────────────────┘                    └─────────────────┘   │
│                                                                   │
│  Now we can TRAVERSE: Sarah → published → Neural Networks       │
│                        Neural Networks ← based_on ← Bob's work  │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

With this graph, querying "How did Sarah influence Bob?" becomes a **graph traversal** problem.

---

## EdgeQuake's Approach

EdgeQuake implements Graph-RAG with these components:

| Component                   | Purpose               | Implementation                |
| --------------------------- | --------------------- | ----------------------------- |
| **Entity Extraction**       | Find entities in text | LLM-based with tuple parsing  |
| **Relationship Extraction** | Find connections      | Same LLM call, explicit edges |
| **Knowledge Graph**         | Store structure       | PostgreSQL + Apache AGE       |
| **Vector Embeddings**       | Semantic search       | pgvector                      |
| **Hybrid Retrieval**        | Query both            | 6 query modes                 |

```
┌─────────────────────────────────────────────────────────────────┐
│                    EDGEQUAKE PIPELINE                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  Document ──▶ [Chunk] ──▶ [Extract] ──▶ [Store] ──▶ Query       │
│                             │            │                       │
│                             │            ├─▶ Entities (nodes)    │
│                             │            ├─▶ Relations (edges)   │
│                             │            └─▶ Embeddings (vectors)│
│                             │                                    │
│                             └─▶ LLM extracts entities +         │
│                                 relationships in one pass        │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Benefits

| Benefit                   | Description                             |
| ------------------------- | --------------------------------------- |
| **Multi-hop reasoning**   | Follow chains of relationships          |
| **Entity disambiguation** | "Apple" (company) vs "apple" (fruit)    |
| **Implicit connections**  | Discover relationships across documents |
| **Contextual answers**    | Include related entities in responses   |

---

## Learn More

- **How extraction works**: [Entity Extraction](entity-extraction.md)
- **Where data is stored**: [Knowledge Graph](knowledge-graph.md)
- **How queries work**: [Hybrid Retrieval](hybrid-retrieval.md)
- **Algorithm details**: [LightRAG Algorithm Deep-Dive](../deep-dives/lightrag-algorithm.md)

---

## Source Code

The Graph-RAG orchestration lives in:

- [edgequake-core/src/orchestrator.rs](../../edgequake/crates/edgequake-core/src/orchestrator.rs)
