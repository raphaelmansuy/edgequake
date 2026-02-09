# EdgeQuake vs Traditional RAG

> **Why Knowledge Graphs Transform Retrieval Quality**

Traditional RAG (Retrieval-Augmented Generation) uses vector similarity search to find relevant document chunks. EdgeQuake adds knowledge graph construction, enabling semantic understanding of entity relationships that pure vector search misses.

---

## Quick Comparison

| Aspect            | Traditional RAG        | Graph-Enhanced RAG (EdgeQuake) |
| ----------------- | ---------------------- | ------------------------------ |
| **Retrieval**     | Vector similarity only | Vector + Graph traversal       |
| **Understanding** | Semantic similarity    | Entity relationships           |
| **Multi-hop**     | ❌ Single-hop          | ✅ Multi-hop reasoning         |
| **Themes**        | ❌ Local only          | ✅ Global themes               |
| **Indexing**      | Fast (~1s/doc)         | Slower (~5-30s/doc)            |
| **Query Latency** | ~100-300ms             | ~200-500ms                     |

---

## The Problem with Vector-Only Search

Traditional RAG has fundamental limitations:

### 1. Lost Relationships

Consider this document:

> "Sarah Chen works at MIT. She authored the climate paper with Dr. James Wilson."

**Question**: "What is the connection between Sarah Chen and James Wilson?"

```
┌─────────────────────────────────────────────────────────────────┐
│                   TRADITIONAL RAG PROBLEM                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Document chunks:                                               │
│  ┌────────────────────────────────────────┐                     │
│  │ Chunk 1: "Sarah Chen works at MIT..."  │ → embedding_1       │
│  └────────────────────────────────────────┘                     │
│  ┌────────────────────────────────────────┐                     │
│  │ Chunk 2: "She authored the climate..." │ → embedding_2       │
│  └────────────────────────────────────────┘                     │
│                                                                 │
│  Query: "connection between Sarah and James"                    │
│                                                                 │
│  Vector search: May find Chunk 1 (Sarah mentioned)              │
│                 May miss Chunk 2 (if "connection" not similar)  │
│                                                                 │
│  PROBLEM: No explicit link between Sarah and James!             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 2. No Global Understanding

**Question**: "What are the main themes in this 50-page document?"

Traditional RAG retrieves the most semantically similar chunks to "themes", but this misses the document's structure and organization.

### 3. No Multi-Hop Reasoning

**Question**: "Who are Sarah Chen's collaborators' organizations?"

This requires:

1. Find Sarah Chen
2. Find her collaborators
3. Find their organizations

Vector search cannot chain these lookups.

---

## How Graph-Enhanced RAG Solves This

EdgeQuake constructs a knowledge graph during indexing:

```
┌─────────────────────────────────────────────────────────────────┐
│                   GRAPH-ENHANCED RAG                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Document → LLM Extraction → Knowledge Graph                    │
│                                                                 │
│         ┌───────────────┐                                       │
│         │  SARAH_CHEN   │                                       │
│         │  (PERSON)     │                                       │
│         └───────┬───────┘                                       │
│                 │                                               │
│    ┌───────────┼───────────┐                                    │
│    │ WORKS_AT  │ CO_AUTHORED                                    │
│    ▼           ▼                                                │
│  ┌─────┐    ┌──────────────┐                                    │
│  │ MIT │    │ CLIMATE_PAPER│                                    │
│  └─────┘    └──────┬───────┘                                    │
│                    │ AUTHORED_BY                                │
│                    ▼                                            │
│             ┌──────────────┐                                    │
│             │ JAMES_WILSON │                                    │
│             │  (PERSON)    │                                    │
│             └──────────────┘                                    │
│                                                                 │
│  Query: "connection between Sarah and James"                    │
│                                                                 │
│  Graph traversal: SARAH_CHEN → CLIMATE_PAPER → JAMES_WILSON     │
│                   Relationship: CO_AUTHORED                     │
│                                                                 │
│  ANSWER: "Sarah and James co-authored the climate paper"        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Feature Comparison

| Feature                 | Traditional RAG | EdgeQuake |
| ----------------------- | :-------------: | :-------: |
| Chunk embedding         |       ✅        |    ✅     |
| Entity extraction       |       ❌        |    ✅     |
| Relationship extraction |       ❌        |    ✅     |
| Knowledge graph         |       ❌        |    ✅     |
| Multi-hop queries       |       ❌        |    ✅     |
| Theme detection         |       ❌        |    ✅     |
| Entity-centric search   |       ❌        |    ✅     |
| Global context          |       ❌        |    ✅     |
| Source lineage          |    ⚠️ Basic     |  ✅ Full  |

---

## Query Quality Comparison

Research from the LightRAG paper (arxiv:2410.05779) shows significant improvements:

| Dataset     | Traditional RAG | Graph-RAG | Improvement |
| ----------- | --------------- | --------- | ----------- |
| Agriculture | 32.4%           | 67.6%     | **+35%**    |
| CS          | 38.4%           | 61.6%     | **+23%**    |
| Legal       | 16.4%           | 83.6%     | **+67%**    |
| Mix         | 38.8%           | 61.2%     | **+22%**    |

_Metrics: Comprehensiveness, measured by LLM-as-judge evaluation_

---

## Indexing Cost Trade-off

Graph-enhanced RAG requires more processing at index time:

```
┌─────────────────────────────────────────────────────────────────┐
│                   INDEXING COMPARISON                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Traditional RAG:                                               │
│  ┌────────┐    ┌───────────┐    ┌─────────────┐                 │
│  │ Doc    │ ─▶ │ Chunk     │ ─▶ │ Embed       │ ─▶ Done         │
│  │        │    │ (~10ms)   │    │ (~100ms)    │                 │
│  └────────┘    └───────────┘    └─────────────┘                 │
│                                                                 │
│  Total: ~200ms per document                                     │
│                                                                 │
│  ────────────────────────────────────────────────────────────── │
│                                                                 │
│  EdgeQuake:                                                     │
│  ┌────────┐    ┌───────────┐    ┌─────────────┐                 │
│  │ Doc    │ ─▶ │ Chunk     │ ─▶ │ LLM Extract │ ─▶ ─┐           │
│  │        │    │ (~10ms)   │    │ (~2-10s)    │    │            │
│  └────────┘    └───────────┘    └─────────────┘    │            │
│                                                      ▼          │
│                                            ┌─────────────┐      │
│                                            │ Graph Merge │      │
│                                            │ (~100ms)    │      │
│                                            └──────┬──────┘      │
│                                                   ▼             │
│                                            ┌─────────────┐      │
│                                            │ Embed       │      │
│                                            │ (~200ms)    │      │
│                                            └─────────────┘      │
│                                                                 │
│  Total: ~5-30s per document                                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Cost-Benefit Analysis

| Scenario                       | Traditional RAG        | EdgeQuake                    | Recommendation |
| ------------------------------ | ---------------------- | ---------------------------- | -------------- |
| 100 docs, simple queries       | ✅ Fast, cheap         | Overkill                     | Traditional    |
| 100 docs, relationship queries | ❌ Poor quality        | ✅ Good                      | EdgeQuake      |
| 10K docs, mixed queries        | Fast, moderate quality | Slower index, better quality | EdgeQuake      |
| Real-time indexing needed      | ✅ Works               | ⚠️ Latency                   | Traditional    |

---

## When to Choose Each Approach

### Choose Traditional RAG When:

- ✅ Documents have simple, factual content
- ✅ Queries are keyword-based lookups
- ✅ Real-time indexing is required
- ✅ LLM costs are a primary concern
- ✅ You need minimal infrastructure

### Choose EdgeQuake When:

- ✅ Documents describe entities and relationships
- ✅ Users ask about connections and themes
- ✅ Multi-hop reasoning is needed
- ✅ Answer quality is more important than indexing speed
- ✅ Global document understanding is required

---

## Hybrid Approach

EdgeQuake's query modes let you blend both approaches:

| Mode     | Strategy              | Use Case                  |
| -------- | --------------------- | ------------------------- |
| `naive`  | Vector only           | Simple factual queries    |
| `local`  | Vector + Entity graph | Entity-specific questions |
| `global` | Graph communities     | Theme/overview questions  |
| `hybrid` | All approaches        | Complex queries (default) |

This means you get the best of both worlds:

- Fast vector search for simple queries
- Graph traversal for complex reasoning
- Combined context for comprehensive answers

---

## Implementation Effort

| Aspect            | Traditional RAG | EdgeQuake                     |
| ----------------- | --------------- | ----------------------------- |
| Setup complexity  | Low             | Medium                        |
| LLM calls per doc | 1 (embedding)   | 3-10 (extraction + embedding) |
| Infrastructure    | Vector DB only  | Vector + Graph DB             |
| Maintenance       | Simple          | Moderate                      |
| Query tuning      | Limited         | 6 modes to optimize           |

---

## Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                    DECISION MATRIX                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Question Type              │ Traditional │ EdgeQuake           │
│  ─────────────────────────────────────────────────────────────  │
│  "What is X?"               │    ⭐⭐⭐         ⭐⭐⭐           
│  "How does X work?"         │    ⭐⭐           ⭐⭐⭐             
│  "What connects X and Y?"   │    ⭐           ⭐⭐⭐⭐             
│  "Main themes in doc?"      │    ⭐           ⭐⭐⭐⭐             
│  "X's collaborators' orgs?" │    ❌           ⭐⭐⭐⭐             
│                                                                  
│  If most queries are multi-hop or relationship-based,           │
│  EdgeQuake provides significantly better results.               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## See Also

- [LightRAG Algorithm](../deep-dives/lightrag-algorithm.md) - How the algorithm works
- [Graph-RAG Concepts](../concepts/graph-rag.md) - Understanding graph-enhanced RAG
- [Query Modes](../deep-dives/query-modes.md) - Choosing the right mode
- [vs GraphRAG](vs-graphrag.md) - Microsoft's approach comparison
