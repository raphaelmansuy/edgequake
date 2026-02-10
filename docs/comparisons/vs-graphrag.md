# EdgeQuake vs Microsoft GraphRAG

> **Two Approaches to Graph-Enhanced RAG**

Both EdgeQuake and Microsoft GraphRAG use knowledge graphs to enhance retrieval quality. They share similar goals but differ significantly in implementation, architecture, and operational characteristics.

---

## Quick Comparison

| Aspect                  | Microsoft GraphRAG                   | EdgeQuake                                     |
| ----------------------- | ------------------------------------ | --------------------------------------------- |
| **Language**            | Python                               | Rust                                          |
| **GitHub Stars**        | 30.6k+                               | ~1k                                           |
| **License**             | MIT                                  | Apache-2.0                                    |
| **Algorithm Origin**    | Original research (arxiv:2404.16130) | LightRAG paper (arxiv:2410.05779)             |
| **Community Detection** | Leiden (hierarchical)                | Louvain (flat)                                |
| **Query Modes**         | 4 (Global, Local, DRIFT, Basic)      | 6 (naive, local, global, hybrid, mix, bypass) |
| **Multi-tenant**        | ❌                                   | ✅ Built-in                                   |
| **Async Runtime**       | asyncio                              | Tokio                                         |
| **Indexing Cost**       | Very high ($$$)                      | Moderate ($$)                                 |

---

## Architectural Philosophy

```
┌─────────────────────────────────────────────────────────────────┐
│                   GRAPHRAG ARCHITECTURE                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐                                                │
│  │   Python    │  Pandas DataFrames, asyncio                    │
│  │ Data Pipes  │  Pipeline-based data transformation            │
│  └──────┬──────┘                                                │
│         │                                                       │
│         ▼                                                       │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐          │
│  │   Parquet   │    │  LanceDB    │    │   CosmosDB  │          │
│  │   Files     │    │  (Vector)   │    │  (Optional) │          │
│  └─────────────┘    └─────────────┘    └─────────────┘          │
│                                                                 │
│  Focus: Research, Analysis, Batch Processing                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                   EDGEQUAKE ARCHITECTURE                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐                                                │
│  │    Rust     │  Tokio async, zero-copy, 11 crates             │
│  │   Engine    │  Multi-tenant, streaming-first                 │
│  └──────┬──────┘                                                │
│         │                                                       │
│         ▼                                                       │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              PostgreSQL (Unified Backend)                  │ │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────────────────────────┐ │ │
│  │  │pgvector │  │ Apache  │  │   Standard Tables           │ │ │
│  │  │(vectors)│  │  AGE    │  │   (docs, workspaces)        │ │ │
│  │  │         │  │ (graph) │  │                             │ │ │
│  │  └─────────┘  └─────────┘  └─────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  Focus: Production Services, Multi-tenant SaaS                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Algorithm Comparison

### Community Detection

| Aspect       | GraphRAG              | EdgeQuake              |
| ------------ | --------------------- | ---------------------- |
| Algorithm    | Leiden                | Louvain                |
| Hierarchy    | ✅ Multi-level        | ⚠️ Flat (single level) |
| Summaries    | Per-level reports     | Community summaries    |
| Use at Query | Level-based selection | All communities        |

**GraphRAG's Hierarchical Approach:**

```
                    Level 0
                 ┌────────────┐
                 │ High-level │
                 │  Summary   │
                 └─────┬──────┘
                       │
          ┌────────────┼────────────┐
          ▼            ▼            ▼
    ┌──────────┐ ┌──────────┐ ┌──────────┐   Level 1
    │ Cluster  │ │ Cluster  │ │ Cluster  │
    │ Summary  │ │ Summary  │ │ Summary  │
    └────┬─────┘ └────┬─────┘ └────┬─────┘
         │            │            │
    ┌────┴────┐  ┌────┴────┐  ┌────┴────┐   Level 2
    │ Nodes   │  │ Nodes   │  │ Nodes   │
    └─────────┘  └─────────┘  └─────────┘
```

GraphRAG generates summaries at each hierarchical level, allowing queries to target the appropriate level of detail.

**EdgeQuake's Flat Approach:**

```
    ┌──────────┐ ┌──────────┐ ┌──────────┐
    │ Community│ │ Community│ │ Community│
    │    1     │ │    2     │ │    3     │
    │ Summary  │ │ Summary  │ │ Summary  │
    └────┬─────┘ └────┬─────┘ └────┬─────┘
         │            │            │
    ┌────┴────┐  ┌────┴────┐  ┌────┴────┐
    │ Entities│  │ Entities│  │ Entities│
    └─────────┘  └─────────┘  └─────────┘
```

EdgeQuake uses flat communities, trading hierarchical flexibility for simpler implementation and faster indexing.

---

### Query Modes Mapping

| GraphRAG Mode | EdgeQuake Equivalent | Description                                |
| ------------- | -------------------- | ------------------------------------------ |
| Global Search | `global`             | Community summaries for holistic questions |
| Local Search  | `local`              | Entity-centered graph traversal            |
| DRIFT Search  | N/A                  | Local + community context                  |
| Basic Search  | `naive`              | Standard vector similarity                 |
| N/A           | `hybrid`             | Combined local + global (default)          |
| N/A           | `mix`                | Weighted combination with scores           |
| N/A           | `bypass`             | Direct LLM, no retrieval                   |

**Key Difference:** GraphRAG's DRIFT (Dynamic Reasoning Including Facts and Themes) mode combines local entity search with community context. EdgeQuake's `hybrid` mode combines local and global but without the adaptive weighting.

---

## Indexing Pipeline

### GraphRAG Pipeline

```
┌────────────┐    ┌────────────┐    ┌────────────┐
│   Load     │ ─▶ │   Chunk    │ ─▶ │  Extract   │
│ Documents  │    │ Documents  │    │   Graph    │
└────────────┘    └────────────┘    └─────┬──────┘
                                          │
                                          ▼
┌────────────┐    ┌────────────┐    ┌────────────┐
│   Embed    │ ◀─ │  Generate  │ ◀─ │  Detect    │
│  Reports   │    │  Reports   │    │Communities │
└─────┬──────┘    └────────────┘    └────────────┘
      │
      ▼
┌────────────┐    ┌────────────┐    ┌────────────┐
│   Embed    │ ─▶ │   Embed    │ ─▶ │  Extract   │
│  Chunks    │    │  Entities  │    │   Claims   │
└────────────┘    └────────────┘    └────────────┘
```

**GraphRAG extras:**

- Claims extraction (fact-like statements)
- Multi-level community reports
- Entity covariates (additional attributes)

### EdgeQuake Pipeline

```
┌────────────┐    ┌────────────┐    ┌────────────┐
│   Load     │ ─▶ │   Chunk    │ ─▶ │  Extract   │
│ Documents  │    │ Documents  │    │ Entities + │
└────────────┘    └────────────┘    │ Relations  │
                                    └─────┬──────┘
                                          │
                                          ▼
┌────────────┐    ┌────────────┐    ┌────────────┐
│   Store    │ ◀─ │  Community │ ◀─ │  Normalize │
│   Graph    │    │ Detection  │    │  & Merge   │
└────────────┘    └────────────┘    └────────────┘
```

**EdgeQuake optimizations:**

- Entity normalization (deduplication)
- Gleaning (multi-pass extraction)
- Source lineage tracking
- Concurrent chunk processing

---

## Performance Characteristics

### Indexing Cost

| Document Type  | GraphRAG   | EdgeQuake   | Notes            |
| -------------- | ---------- | ----------- | ---------------- |
| 10-page report | ~$5-15     | ~$0.50-2.00 | Per document     |
| 100-page book  | ~$50-150   | ~$5-20      | Highly variable  |
| 1000 documents | ~$500-5000 | ~$50-500    | Batch processing |

**Why GraphRAG costs more:**

1. Hierarchical community summaries at multiple levels
2. Claims extraction (additional LLM calls)
3. Entity covariates extraction
4. Coarser chunking requiring more context

**Why EdgeQuake costs less:**

1. Flat community structure
2. Optimized prompts from LightRAG research
3. Entity deduplication reduces redundancy
4. Smaller default chunk sizes

### Query Latency

| Query Type        | GraphRAG           | EdgeQuake   |
| ----------------- | ------------------ | ----------- |
| Simple lookup     | ~300-800ms         | ~200-500ms  |
| Global (themes)   | ~2-5s (map-reduce) | ~500ms-2s   |
| Complex reasoning | ~1-3s              | ~500ms-1.5s |

**GraphRAG's map-reduce:**
Global search uses map-reduce over community reports, which is thorough but slow. Each "map" step generates intermediate responses, then "reduce" aggregates them.

**EdgeQuake's parallel approach:**
Uses Tokio's concurrent task execution for parallel context retrieval, generally faster for production workloads.

---

## Feature Matrix

| Feature                 |    GraphRAG    |  EdgeQuake  |
| ----------------------- | :------------: | :---------: |
| Entity extraction       |       ✅       |     ✅      |
| Relationship extraction |       ✅       |     ✅      |
| Community detection     | ✅ Multi-level |   ✅ Flat   |
| Community summaries     |       ✅       |     ✅      |
| Claims extraction       |       ✅       |     ❌      |
| Entity covariates       |       ✅       |     ❌      |
| Gleaning (multi-pass)   |       ❌       |     ✅      |
| Entity normalization    |    ⚠️ Basic    | ✅ Advanced |
| Source lineage          |    ⚠️ Basic    |   ✅ Full   |
| Multi-tenant            |       ❌       |     ✅      |
| REST API                |       ❌       |     ✅      |
| Streaming responses     |       ⚠️       |   ✅ SSE    |
| OpenAI-compatible API   |       ❌       |     ✅      |
| Prompt tuning CLI       |       ✅       |     ❌      |
| DRIFT search            |       ✅       |     ❌      |
| LLM caching             |       ✅       |  ⚠️ Basic   |

---

## Storage Backends

### GraphRAG Options

| Backend         | Vector | Graph | Status    |
| --------------- | ------ | ----- | --------- |
| Parquet/Files   | ❌     | ✅    | Default   |
| LanceDB         | ✅     | ❌    | Default   |
| Azure AI Search | ✅     | ❌    | Supported |
| CosmosDB        | ✅     | ✅    | Supported |
| Neo4j           | ❌     | ✅    | Community |

### EdgeQuake Options

| Backend                     | Vector | Graph | Status   |
| --------------------------- | ------ | ----- | -------- |
| PostgreSQL + pgvector + AGE | ✅     | ✅    | Default  |
| In-Memory                   | ✅     | ✅    | Dev only |

**EdgeQuake's unified PostgreSQL:**

- Single database for all storage needs
- Transactional consistency
- Simpler deployment
- Enterprise-ready (backup, replication, etc.)

---

## Deployment Complexity

### GraphRAG

```yaml
# Typical GraphRAG deployment needs:
dependencies:
  - Python 3.10+
  - LLM API (OpenAI/Azure)
  - File storage (Parquet)
  - Vector store (LanceDB/Azure AI Search)
  - Optional: CosmosDB, Neo4j

deployment_model: CLI/Notebook-driven
production_ready: Limited (research focus)
multi_tenant: Manual implementation required
```

### EdgeQuake

```yaml
# EdgeQuake deployment needs:
dependencies:
  - Rust runtime (compiled binary)
  - PostgreSQL 15+ with extensions
  - LLM API (OpenAI/Ollama)

deployment_model: Docker/Container
production_ready: Yes
multi_tenant: Built-in via workspaces
```

---

## Use Case Recommendations

### Choose GraphRAG When:

- ✅ Deep research and analysis is the goal
- ✅ Hierarchical document understanding is critical
- ✅ You need claims/facts extraction
- ✅ You're working in a Python-centric environment
- ✅ Indexing cost is not a concern
- ✅ Batch processing is acceptable

### Choose EdgeQuake When:

- ✅ Building a production service
- ✅ Multi-tenant SaaS is required
- ✅ Real-time query latency matters
- ✅ Indexing cost optimization is important
- ✅ PostgreSQL is your preferred database
- ✅ REST API is needed
- ✅ Streaming responses are required

---

## Migration Considerations

### GraphRAG → EdgeQuake

1. **Data Export:** Export entities and relationships from GraphRAG's Parquet files
2. **Schema Mapping:** Map to EdgeQuake's PostgreSQL schema
3. **Community Re-detection:** EdgeQuake uses flat communities, re-run detection
4. **Query Mode Adjustment:** Map GraphRAG modes to EdgeQuake equivalents

### EdgeQuake → GraphRAG

1. **Data Export:** Query PostgreSQL for entities/relationships
2. **Format Conversion:** Convert to GraphRAG's expected input format
3. **Re-indexing:** Full re-index required for hierarchical communities
4. **API Replacement:** Replace REST API calls with GraphRAG library calls

---

## Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                    DECISION MATRIX                               
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Requirement                 │ GraphRAG  │ EdgeQuake            │
│  ─────────────────────────────────────────────────────────────  │
│  Research/Analysis           │    ⭐⭐⭐⭐       ⭐⭐⭐               
│  Production Service          │    ⭐⭐         ⭐⭐⭐⭐              
│  Multi-tenant SaaS           │    ⭐          ⭐⭐⭐⭐              
│  Indexing Cost Efficiency    │    ⭐⭐         ⭐⭐⭐⭐             
│  Query Latency               │    ⭐⭐         ⭐⭐⭐⭐              
│  Hierarchical Understanding  │    ⭐⭐⭐⭐       ⭐⭐⭐               
│  Python Ecosystem            │    ⭐⭐⭐⭐       ⭐⭐                
│  Claims Extraction           │    ⭐⭐⭐⭐       ❌                 
│  REST API                    │    ⭐          ⭐⭐⭐⭐              
│                                                                 
│  GraphRAG: Best for research, analysis, deep document study     │
│  EdgeQuake: Best for production services, SaaS, real-time apps  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## References

- [Microsoft GraphRAG Paper](https://arxiv.org/pdf/2404.16130) (arxiv:2404.16130)
- [LightRAG Paper](https://arxiv.org/abs/2410.05779) (arxiv:2410.05779)
- [GraphRAG Documentation](https://microsoft.github.io/graphrag/)
- [EdgeQuake Quick Start](../getting-started/quick-start.md)

---

## See Also

- [vs LightRAG Python](vs-lightrag-python.md) - Comparison with the Python reference implementation
- [vs Traditional RAG](vs-traditional-rag.md) - Why graphs matter
- [Query Modes](../deep-dives/query-modes.md) - EdgeQuake's 6 query strategies
- [LightRAG Algorithm](../deep-dives/lightrag-algorithm.md) - Algorithm deep-dive
