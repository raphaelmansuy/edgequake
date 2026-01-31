# Iteration 08: Observe - Query Engine Deep Dive

## Topic: Multi-Mode Query Engine Architecture

### Codebase Research Results

#### Query Modes (`modes.rs`)

```rust
pub enum QueryMode {
    Naive,   // Vector similarity only - FEAT0101
    Local,   // Entity-centric graph - FEAT0102
    Global,  // Community summaries - FEAT0103
    Hybrid,  // Local + Global combined - FEAT0104 (DEFAULT)
    Mix,     // Weighted naive + graph - FEAT0105
}
```

#### Mode Selection Guidelines

| Question Type         | Best Mode | Why                          |
| --------------------- | --------- | ---------------------------- |
| Factual/specific      | Naive     | Direct vector match, fast    |
| Entity relationships  | Local     | Explores entity neighborhood |
| Broad/thematic        | Global    | Uses community detection     |
| Complex/multi-faceted | Hybrid    | Both approaches combined     |
| Custom weights needed | Mix       | Configurable blend           |

#### Performance vs Accuracy Trade-offs

```
Mode    | Speed | Accuracy | Context Size
--------|-------|----------|-------------
Naive   | Fast  | Good     | Small (chunks only)
Local   | Med   | High     | Medium (entity + neighbors)
Global  | Slow  | High     | Large (community summaries)
Hybrid  | Slow  | Best     | Large (both approaches)
```

#### SOTA Query Engine Architecture (`sota_engine.rs`)

```
Query → Keyword Extraction → Mode Router
                                ↓
        ┌───────────────────────┼───────────────────────┐
        ↓                       ↓                       ↓
    Local Mode             Global Mode             Naive Mode
  (Entity VDB +          (Relationship VDB +      (Chunk VDB)
   low-level kw)          high-level kw)
        ↓                       ↓                       ↓
        └───────────────────────┼───────────────────────┘
                                ↓
                        Context Building
                                ↓
                        Token Budgeting
                                ↓
                        LLM Generation
```

#### Key Components

1. **Keyword Extraction** (LLM-based with caching)
   - High-level keywords: themes, concepts → Global mode
   - Low-level keywords: entities, specifics → Local mode
   - Cache TTL: 24 hours by default

2. **Vector Storage Filtering**
   - Entity embeddings (type="entity")
   - Relationship embeddings (type="relationship")
   - Chunk embeddings (type="chunk")

3. **Query Embeddings**
   - query: Original query embedding
   - high_level: Keywords embedding for Global mode
   - low_level: Keywords embedding for Local mode

4. **Token Budgeting**
   - max_context_tokens: 4000 (default)
   - Graph context priority over naive chunks
   - Smart truncation to fit LLM window

5. **Reranking** (optional)
   - BM25 or cross-encoder reranking
   - min_rerank_score: 0.1
   - rerank_top_k: 10

#### Configuration

```rust
SOTAQueryConfig {
    default_mode: QueryMode::Hybrid,
    max_entities: 20,
    max_relationships: 20,
    max_chunks: 10,
    max_context_tokens: 4000,
    graph_depth: 2,
    min_score: 0.1,
    use_keyword_extraction: true,
    use_adaptive_mode: true,
    enable_rerank: true,
}
```

#### Business Rules Discovered

- **BR0101**: Token budget enforcement (configurable, default 4000)
- **BR0102**: Graph context priority over naive chunks
- **BR0103**: Query mode must be valid enum value
- **BR0104**: Conversation history in context
- **BR0106**: Keyword cache TTL 24 hours default

### Key Differentiators

1. **LightRAG algorithm implementation**: Multi-level keyword extraction
2. **Adaptive mode selection**: Query intent determines retrieval strategy
3. **Graph-first priority**: Knowledge graph context over raw chunks
4. **Token budgeting**: Never exceed LLM context window
5. **Reranking**: Optional BM25/cross-encoder for precision
