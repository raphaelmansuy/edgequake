# OODA Iteration 01 - Observe

## Mission Re-Read ✅

**Mission**: Write 15+ promotional articles for EdgeQuake (Medium, LinkedIn, X.com)
**Location**: `./articles/` with numbered subfolders
**Spec File**: `./specs/006-write-articles.md`

---

## 🔭 Territory Mapping

### EdgeQuake Product Overview

EdgeQuake is a **high-performance Graph-RAG framework in Rust** that combines:

- Knowledge graphs (PostgreSQL + Apache AGE)
- Vector similarity search (pgvector)
- LLM-based entity extraction
- 5 query modes (naive, local, global, hybrid, mix)

### Key Differentiators (from codebase analysis)

| Feature              | EdgeQuake      | Traditional RAG | Business Impact     |
| -------------------- | -------------- | --------------- | ------------------- |
| Entity extraction    | ✅ LLM-based   | ❌ None         | 2-3x more context   |
| Relationship mapping | ✅ Graph edges | ❌ None         | Multi-hop reasoning |
| Query latency        | <200ms hybrid  | ~1000ms         | 5x faster           |
| Deduplication        | 40% reduction  | None            | Cleaner knowledge   |
| Cost per document    | $0.0014        | N/A             | Cost-effective      |

### Performance Metrics (verified from README.md)

```
┌────────────────────────────────────────────────────────────────┐
│                    PERFORMANCE BENCHMARKS                       │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Entity Extraction:     ~2-3x more entities vs traditional     │
│  Query Latency:         <200ms (hybrid) vs ~1000ms             │
│  Document Processing:   25s (10k tokens) vs ~60s               │
│  Concurrent Users:      1000+ vs ~100                          │
│  Memory Usage:          2MB per doc vs ~8MB                    │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

### LightRAG Algorithm Foundation (from docs/deep-dives/lightrag-algorithm.md)

EdgeQuake implements LightRAG with enhancements:

1. **Graph-Enhanced Indexing**: Entities + Relationships + Embeddings
2. **Dual-Level Retrieval**: Low-level (entities) + High-level (themes)
3. **Incremental Updates**: No full reindex required
4. **Gleaning**: Multi-pass extraction for better coverage

### Query Modes (from codebase)

```
┌──────────┬─────────────────────────────────────────────────┬───────────┐
│ Mode     │ Description                                     │ Latency   │
├──────────┼─────────────────────────────────────────────────┼───────────┤
│ Naive    │ Simple vector similarity                        │ ~50ms     │
│ Local    │ Entity-centric with local graph context         │ ~150ms    │
│ Global   │ Community-based semantic search                 │ ~200ms    │
│ Hybrid   │ Combines local + global                         │ ~250ms    │
│ Mix      │ Weighted combination with configurable ratios   │ ~300ms    │
└──────────┴─────────────────────────────────────────────────┴───────────┘
```

### Target Audience Identified

1. **CTOs/VPs Engineering**: ROI, scalability, production-readiness
2. **ML Engineers**: Algorithm details, performance tuning
3. **Data Scientists**: Knowledge graph benefits, entity extraction
4. **DevOps Engineers**: Deployment, Docker, Kubernetes
5. **Technical Decision Makers**: Comparisons, cost-benefit

### Competitor Landscape

| Competitor         | Approach           | EdgeQuake Advantage              |
| ------------------ | ------------------ | -------------------------------- |
| Microsoft GraphRAG | Python, complex    | Rust performance, simpler deploy |
| LightRAG (Python)  | Original algorithm | Production-ready Rust port       |
| LangChain RAG      | Vector-only        | Knowledge graph benefits         |
| LlamaIndex         | Vector-only        | Entity relationships             |

### Article Topics Inventory (15 confirmed)

1. **001_why_classic_rag_fails** - Problem statement, hook article
2. **002_edgequake_approach** - Solution overview
3. **003_entity_extraction_deep_dive** - Technical depth
4. **004_graph_storage_architecture** - PostgreSQL AGE + pgvector
5. **005_rust_performance** - Why Rust matters
6. **006_llm_provider_abstraction** - Multi-provider support
7. **007_pipeline_architecture** - Document processing
8. **008_query_engine** - 5 query modes
9. **009_deduplication_normalization** - Entity cleanup
10. **010_real_world_use_cases** - Legal, Healthcare, Finance
11. **011_cost_optimization** - $0.0014 per document
12. **012_production_deployment** - Dev to scale
13. **013_comparison_lightrag** - vs LightRAG Python
14. **014_webui_experience** - React 19 frontend
15. **015_future_roadmap** - What's next

---

## 📁 Current State

- `./articles/` folder: **Empty** (ready for content)
- `./specs/006-write-articles.md`: **Created** (mission defined)
- OODA loop structure: **Started** (iteration 01)

---

## 🔍 Key Insights from Codebase

### Entity Extraction (from edgequake-pipeline/src/\*)

- Uses tuple-delimited format (not JSON) for reliability
- Normalization: UPPERCASE_UNDERSCORED names
- Gleaning: Multi-pass extraction for better coverage
- Example: 40+ raw entities → 12 unique nodes (67% deduplication)

### Storage Architecture

```
PostgreSQL (Production)
├── Apache AGE (Graph: nodes + edges)
├── pgvector (Embeddings)
└── Row-Level Security (Multi-tenant)

Memory (Development)
├── In-memory graph
└── In-memory vectors
```

### Cost Metrics (from examples/production_pipeline.rs)

- gpt-4o-mini: $0.0014 per document (~10k tokens)
- text-embedding-3-small: 1536 dimensions
- Batch processing: Parallel chunk extraction

---

## Next Steps (for Orient phase)

1. Analyze which article should be written first (highest impact)
2. Research current GraphRAG landscape for accurate comparisons
3. Define article structure template for consistency
4. Plan content creation order (dependencies between articles)
