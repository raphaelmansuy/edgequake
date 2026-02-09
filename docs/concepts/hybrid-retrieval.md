# Hybrid Retrieval

> **Hybrid retrieval combines vector similarity search with knowledge graph
> traversal to provide comprehensive context for LLM responses.**

---

## What is Hybrid Retrieval?

Hybrid retrieval uses **both** approaches together:

1. **Vector Search**: Find semantically similar content
2. **Graph Traversal**: Follow entity relationships

```
┌─────────────────────────────────────────────────────────────────┐
│                    HYBRID RETRIEVAL                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│                       USER QUERY                                  │
│                           │                                       │
│              ┌────────────┴────────────┐                         │
│              │                         │                          │
│              v                         v                          │
│     ┌─────────────────┐       ┌─────────────────┐                │
│     │  VECTOR SEARCH  │       │ GRAPH TRAVERSAL │                │
│     │                 │       │                 │                │
│     │ • Embeddings    │       │ • Entity match  │                │
│     │ • Cosine sim    │       │ • 1-hop neighbors│               │
│     │ • Top-K chunks  │       │ • Relationship  │                │
│     └────────┬────────┘       └────────┬────────┘                │
│              │                         │                          │
│              └────────────┬────────────┘                         │
│                           │                                       │
│                           v                                       │
│              ┌─────────────────────────┐                         │
│              │    CONTEXT FUSION       │                         │
│              │  • Deduplicate          │                         │
│              │  • Rank by relevance    │                         │
│              │  • Truncate to limit    │                         │
│              └────────────┬────────────┘                         │
│                           │                                       │
│                           v                                       │
│              ┌─────────────────────────┐                         │
│              │     LLM GENERATION      │                         │
│              └─────────────────────────┘                         │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Vector Search vs Graph Traversal

Each approach has strengths:

| Aspect       | Vector Search        | Graph Traversal       |
| ------------ | -------------------- | --------------------- |
| **Best for** | Semantic similarity  | Entity relationships  |
| **Finds**    | Similar text chunks  | Connected entities    |
| **Misses**   | Indirect connections | Semantic nuance       |
| **Speed**    | Fast (HNSW index)    | Medium (path queries) |

**Example:**

Query: _"What did Sarah Chen work on?"_

- **Vector search** finds: Chunks mentioning "Sarah Chen"
- **Graph traversal** finds: `SARAH_CHEN --[researches]--> NEURAL_NETWORKS`

**Combined**: More complete context than either alone.

---

## Dual-Level Approach

EdgeQuake implements LightRAG's dual-level retrieval:

### Low-Level Retrieval

Focuses on **specific entities** and their immediate neighbors:

```
Query: "Who is Sarah Chen?"

Low-Level Results:
┌─────────────────────────────────────────────┐
│  SARAH_CHEN (direct match)                  │
│  ├── Description: "Lead researcher at..."  │
│  │                                           │
│  ├── QUANTUM_LAB (1-hop neighbor)           │
│  │   └── via: WORKS_AT                      │
│  │                                           │
│  ├── NEURAL_NETWORKS (1-hop neighbor)       │
│  │   └── via: RESEARCHES                    │
│  │                                           │
│  └── BOB_SMITH (1-hop neighbor)             │
│      └── via: COLLABORATES_WITH             │
└─────────────────────────────────────────────┘
```

### High-Level Retrieval

Focuses on **broad topics** and theme summaries:

```
Query: "What are the main AI research themes?"

High-Level Results:
┌─────────────────────────────────────────────┐
│  Topic Cluster: "AI RESEARCH"               │
│  ├── Key themes:                            │
│  │   • Neural network architectures        │
│  │   • Deep learning optimization          │
│  │   • Computer vision applications        │
│  │                                           │
│  └── Related entities: 45                   │
└─────────────────────────────────────────────┘
```

---

## Query Modes

EdgeQuake offers 6 query modes for different use cases:

```
┌─────────────────────────────────────────────────────────────────┐
│                    QUERY MODE SPECTRUM                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  Speed ────────────────────────────────────────▶ Comprehensiveness│
│                                                                   │
│  ┌───────┐  ┌───────┐  ┌───────┐  ┌───────┐  ┌───────┐         │
│  │ Naive │  │ Local │  │ Global│  │ Hybrid│  │  Mix  │         │
│  │       │  │       │  │       │  │       │  │       │         │
│  │ Vector│  │Entity │  │Topics │  │ Both  │  │Weighted│        │
│  │ only  │  │+1-hop │  │only   │  │       │  │ blend │         │
│  └───────┘  └───────┘  └───────┘  └───────┘  └───────┘         │
│                                                                   │
│  FASTEST ◄─────────────────────────────────► MOST COMPLETE      │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

| Mode       | Vector   | Graph       | Best For                        |
| ---------- | -------- | ----------- | ------------------------------- |
| **Naive**  | ✅       | ❌          | Simple factual queries          |
| **Local**  | ✅       | ✅ Entities | "Who/What is X?"                |
| **Global** | ❌       | ✅ Topics   | "What are the themes?"          |
| **Hybrid** | ✅       | ✅ Both     | Complex multi-faceted (DEFAULT) |
| **Mix**    | Weighted | Weighted    | Custom blending                 |
| **Bypass** | ❌       | ❌          | Testing/debugging               |

---

## Context Fusion

After retrieval, results are fused into a coherent context:

```
┌─────────────────────────────────────────────────────────────────┐
│                    CONTEXT FUSION                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  Step 1: COLLECT                                                 │
│  ├── Chunks from vector search (10)                             │
│  ├── Entities from graph (20)                                   │
│  └── Relationships from graph (15)                              │
│                                                                   │
│  Step 2: DEDUPLICATE                                             │
│  └── Remove overlapping content                                  │
│                                                                   │
│  Step 3: RANK                                                    │
│  └── Score by relevance to query                                 │
│                                                                   │
│  Step 4: TRUNCATE                                                │
│  └── Fit within context window (4000 tokens default)            │
│                                                                   │
│  Step 5: FORMAT                                                  │
│  └── Structure for LLM consumption                               │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

### Balanced Truncation

EdgeQuake uses intelligent truncation to preserve diversity:

```rust
// From truncation.rs
pub struct TruncationConfig {
    pub max_context_tokens: usize,  // 4000 default
    pub chunk_weight: f32,          // 0.4
    pub entity_weight: f32,         // 0.4
    pub relationship_weight: f32,   // 0.2
}
```

Rather than just taking "top N" of each, it balances across categories.

---

## Choosing a Mode

**Decision guide:**

```
Is this a test/debug? ───▶ Use BYPASS
                │
                No
                │
                v
Is it about specific entities? ───▶ Use LOCAL
("Who is X?", "What is Y?")
                │
                No
                │
                v
Is it about broad themes? ───▶ Use GLOBAL
("What are the main topics?")
                │
                No
                │
                v
Is it complex/multi-faceted? ───▶ Use HYBRID (default)
("How does X relate to Y?")
                │
                │
                v
Need custom control? ───▶ Use MIX with weights
```

---

## API Usage

```bash
# Query with specific mode
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Who is Sarah Chen?",
    "mode": "local"
  }'

# Query with hybrid (default)
curl -X POST http://localhost:8080/api/v1/query \
  -d '{"query": "Tell me about the research"}'
```

---

## Learn More

- **Foundation concept**: [Graph-RAG](graph-rag.md)
- **How entities are extracted**: [Entity Extraction](entity-extraction.md)
- **Storage details**: [Knowledge Graph](knowledge-graph.md)
- **Deep dive**: [LightRAG Algorithm](../deep-dives/lightrag-algorithm.md)

---

## Source Code

- **Query engine**: [engine.rs](../../edgequake/crates/edgequake-query/src/engine.rs)
- **Query modes**: [modes.rs](../../edgequake/crates/edgequake-query/src/modes.rs)
- **Context building**: [context.rs](../../edgequake/crates/edgequake-query/src/context.rs)
