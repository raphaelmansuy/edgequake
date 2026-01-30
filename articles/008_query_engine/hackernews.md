# Beyond Vector Search: Multi-Mode Query Engine for RAG

**Show HN: EdgeQuake - Graph-enhanced RAG with 5 query modes for different question types**

---

Most RAG systems use a single retrieval strategy: vector similarity. Embed the query, find similar chunks, generate answer. Works well for factual lookups, fails spectacularly for relationship questions.

We built a multi-mode query engine that matches retrieval strategy to question type.

## The Problem

Query: "How do sales and engineering collaborate on customer issues?"

Vector search finds:

- Chunks about sales processes
- Chunks about engineering workflows

What's missing: the **relationship** between them. The answer requires traversing connections, not just finding similar text.

## The 5 Modes

| Mode   | Latency      | Use Case                                          |
| ------ | ------------ | ------------------------------------------------- |
| Naive  | ~50ms        | Factual lookups ("What is X?")                    |
| Local  | ~150ms       | Entity relationships ("What does Alice work on?") |
| Global | ~200ms       | Themes/patterns ("What are the main challenges?") |
| Hybrid | ~250ms       | Complex queries (default)                         |
| Mix    | configurable | Domain-specific tuning                            |

### Naive Mode

Pure vector similarity. Fast and effective for 40% of queries.

```
Query → Embed → Vector Search (chunks) → Top-K → LLM
```

### Local Mode

Entity-centric graph traversal. Finds an entity node, explores its neighborhood.

```
Query → Extract Entities → Graph Lookup → Traverse Neighbors → LLM
```

For "What has Sarah Chen worked on?", it finds SARAH_CHEN node, then follows edges to PROJECT_X, TEAM_Y, REPORT_Z.

### Global Mode

Community-based retrieval. Uses high-level keywords to find relationship clusters.

```
Query → Extract Themes → Community Search → Aggregate → LLM
```

### Hybrid Mode (Default)

Combines Local and Global. Runs both in parallel, merges results.

Best for unknown query types. Slightly slower but consistently highest quality.

### Mix Mode

Weighted combination with configurable α and β:

```
Context = α × NaiveResults + β × GraphResults
```

Full control for domain-specific optimization.

## LightRAG Algorithm

We implement the LightRAG paper (arXiv:2410.05779) approach:

1. **Keyword Extraction**: LLM extracts high-level (themes) and low-level (entities) keywords from query

2. **Level-Specific Retrieval**:
   - High-level → relationship embeddings
   - Low-level → entity embeddings
   - Raw query → chunk embeddings

3. **Token Budgeting**: Smart truncation with priority
   - Graph context (entities, relationships): 70%
   - Raw chunks: 30%

   Graph context is pre-summarized, higher signal per token.

## Performance

On 1,000 queries with human evaluation:

| Mode   | Quality Score |
| ------ | ------------- |
| Naive  | 6.2/10        |
| Local  | 7.8/10        |
| Global | 7.5/10        |
| Hybrid | 8.5/10        |

Hybrid is 5x slower than Naive, but 35% higher quality. For relationship-heavy domains, Hybrid is clearly worth it.

## Implementation Details

- Rust + Tokio for async concurrency
- PostgreSQL + Apache AGE (Cypher queries)
- pgvector for embeddings
- Keyword caching (24h TTL, 70-90% hit rate)
- Optional reranking (BM25 or cross-encoder)

Keyword caching is crucial for cost. Same query within 24h skips extraction. Similar queries often hit cache. Reduces LLM calls by 70-90%.

## Adaptive Mode Selection

Optionally, the engine can auto-select mode:

```rust
SOTAQueryConfig {
    use_adaptive_mode: true,
    default_mode: QueryMode::Hybrid,
}
```

LLM analyzes query intent and routes to appropriate mode.

## Try It

```bash
git clone https://github.com/your-org/edgequake
cd edgequake
make dev

curl -X POST http://localhost:3000/api/query \
  -d '{"query": "How do sales and engineering collaborate?", "mode": "hybrid"}'
```

Open source, production-ready. Implements LightRAG (arXiv:2410.05779).

---

Happy to discuss tradeoffs, alternative approaches, or how you handle retrieval strategy selection in your systems.
