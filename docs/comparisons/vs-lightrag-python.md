# EdgeQuake vs LightRAG (Python)

> **A Rust reimplementation with production-grade enhancements**

EdgeQuake is a Rust-native reimplementation of the LightRAG algorithm from HKU. This document compares the two implementations to help you choose the right tool for your needs.

---

## Quick Comparison

| Aspect            | EdgeQuake (Rust)                              | LightRAG (Python)      |
| ----------------- | --------------------------------------------- | ---------------------- |
| **Language**      | Rust                                          | Python                 |
| **Codebase**      | ~130K LOC                                     | ~50K LOC               |
| **Query Modes**   | 6 (naive, local, global, hybrid, mix, bypass) | 6 (same)               |
| **Streaming**     | ✅ Native SSE                                 | ✅ Via streaming       |
| **Multi-tenant**  | ✅ Built-in                                   | ⚠️ Workspace isolation |
| **Database**      | PostgreSQL + pgvector + AGE                   | Multiple options       |
| **Type Safety**   | ✅ Compile-time                               | Runtime only           |
| **Async**         | Tokio-based                                   | asyncio-based          |
| **Memory Safety** | ✅ Guaranteed                                 | ❌ GC-managed          |
| **Deployment**    | Single binary                                 | Python environment     |

---

## Algorithm Fidelity

EdgeQuake faithfully implements the core LightRAG algorithm (arxiv:2410.05779):

### ✅ Shared Features

| Feature                     | Implementation                         |
| --------------------------- | -------------------------------------- |
| **Entity Extraction**       | LLM-based with tuple format            |
| **Relationship Extraction** | Same prompt structure                  |
| **Graph Construction**      | Entity → Node, Relationship → Edge     |
| **Query Modes**             | All 6 modes identical semantics        |
| **Gleaning**                | Multi-pass extraction for completeness |
| **Entity Normalization**    | UPPERCASE_UNDERSCORE format            |
| **Reranking**               | Optional BGE-Reranker support          |

### 🆕 EdgeQuake Enhancements

EdgeQuake adds production features not in the original LightRAG:

| Enhancement                | Description                                               |
| -------------------------- | --------------------------------------------------------- |
| **Multi-tenant Isolation** | Full workspace/tenant isolation with header-based routing |
| **PostgreSQL Integration** | Unified storage with pgvector + Apache AGE                |
| **REST API**               | Production-ready Axum-based HTTP API                      |
| **Type-Safe Crate System** | 11 modular Rust crates for maintainability                |
| **Cost Tracking**          | Token usage and cost metrics per query                    |
| **Source Lineage**         | Full document → chunk → entity provenance                 |

---

## Performance Comparison

### Theoretical Advantages

| Metric                     | EdgeQuake    | LightRAG        | Notes                       |
| -------------------------- | ------------ | --------------- | --------------------------- |
| **Startup Time**           | ~50ms        | ~2-5s           | Python import overhead      |
| **Memory Usage**           | ~50MB base   | ~200MB+         | Python interpreter overhead |
| **Concurrent Connections** | 10,000+      | ~500-1,000      | Tokio async vs asyncio      |
| **CPU Utilization**        | Near-optimal | 30-50% overhead | No GIL, native code         |
| **Binary Size**            | ~30MB        | ~500MB+ deps    | Single static binary        |

### Real-World Performance

Both implementations are **I/O bound** for typical RAG workloads:

- LLM API latency dominates (100-2000ms per call)
- Vector search latency (10-50ms)
- Graph traversal (5-20ms)

**Conclusion**: For most use cases, performance difference is negligible. EdgeQuake advantages appear at scale (>1000 concurrent users).

---

## Query Modes Comparison

Both implementations support the same 6 query modes:

```
┌─────────────────────────────────────────────────────────────────┐
│                    QUERY MODES (IDENTICAL)                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  Mode     │ EdgeQuake            │ LightRAG (Python)            │
│  ─────────┼──────────────────────┼──────────────────────────────│
│  naive    │ ✅ Vector only       │ ✅ Vector only               │
│  local    │ ✅ Entity-centric    │ ✅ Entity-centric            │
│  global   │ ✅ Community-based   │ ✅ Community-based           │
│  hybrid   │ ✅ Local + Global    │ ✅ Local + Global            │
│  mix      │ ✅ Weighted blend    │ ✅ Weighted blend            │
│  bypass   │ ✅ Direct LLM        │ ✅ Direct LLM                │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Storage Backend Comparison

### LightRAG (Python) Storage Options

| Type               | Options                                                |
| ------------------ | ------------------------------------------------------ |
| **KV Storage**     | JsonFile, PostgreSQL, Redis, MongoDB                   |
| **Vector Storage** | NanoVector, PostgreSQL, Milvus, Faiss, Qdrant, MongoDB |
| **Graph Storage**  | NetworkX, Neo4J, PostgreSQL/AGE, Memgraph              |

### EdgeQuake Storage Options

| Type               | Options                            |
| ------------------ | ---------------------------------- |
| **KV Storage**     | PostgreSQL, In-Memory              |
| **Vector Storage** | PostgreSQL (pgvector), In-Memory   |
| **Graph Storage**  | PostgreSQL (Apache AGE), In-Memory |

**Key Difference**: EdgeQuake uses PostgreSQL as a unified backend, simplifying deployment. LightRAG offers more flexibility with multiple backend options.

---

## API Comparison

### LightRAG Python API

```python
from lightrag import LightRAG, QueryParam

rag = LightRAG(
    working_dir="./rag_storage",
    embedding_func=openai_embed,
    llm_model_func=gpt_4o_mini_complete,
)
await rag.initialize_storages()

# Insert
await rag.ainsert("Your document text")

# Query
result = await rag.aquery(
    "What is the main topic?",
    param=QueryParam(mode="hybrid")
)
```

### EdgeQuake REST API

```bash
# Insert document
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{"content": "Your document text", "title": "My Document"}'

# Query
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "What is the main topic?", "mode": "hybrid"}'
```

### EdgeQuake Rust API

```rust
use edgequake_core::EdgeQuake;

let edgequake = EdgeQuake::new(config).await?;

// Insert
edgequake.ingest_text("Your document text", None).await?;

// Query
let response = edgequake.query(
    "What is the main topic?",
    QueryMode::Hybrid,
).await?;
```

---

## When to Choose EdgeQuake

✅ **Choose EdgeQuake when:**

- You need production-ready deployment (single binary, no Python deps)
- Multi-tenant architecture is required
- High concurrency (>500 concurrent users)
- Type safety and compile-time guarantees matter
- You're already using Rust or have Rust expertise
- PostgreSQL is your preferred database
- Memory efficiency is critical
- You need predictable latency (no GC pauses)

❌ **Consider LightRAG Python when:**

- You need rapid prototyping
- Your team has Python expertise
- You need Neo4J or other specialized backends
- You want community plugins and integrations
- You're doing research/experimentation

---

## Migration Guide

### From LightRAG to EdgeQuake

1. **Export your data** using LightRAG's export functions
2. **Transform entity format** (LightRAG uses similar normalization)
3. **Import via EdgeQuake API**:

```bash
# Export from LightRAG (Python)
await rag.export_knowledge(output_path="./export.json")

# Import to EdgeQuake (via API)
curl -X POST http://localhost:8080/api/v1/documents/import \
  -F "file=@export.json"
```

### Data Compatibility

| Data Type     | Compatible | Notes                         |
| ------------- | :--------: | ----------------------------- |
| Entities      |     ✅     | Same normalization format     |
| Relationships |     ✅     | Same structure                |
| Embeddings    |     ⚠️     | Must use same embedding model |
| Query history |     ❌     | Not transferred               |

---

## Feature Matrix

| Feature                   | EdgeQuake | LightRAG |
| ------------------------- | :-------: | :------: |
| Entity Extraction         |    ✅     |    ✅    |
| Relationship Extraction   |    ✅     |    ✅    |
| 6 Query Modes             |    ✅     |    ✅    |
| Gleaning                  |    ✅     |    ✅    |
| Reranking                 |    ✅     |    ✅    |
| Streaming Responses       |    ✅     |    ✅    |
| Multi-tenant              |    ✅     |    ⚠️    |
| REST API                  |    ✅     |    ✅    |
| WebUI                     |    ✅     |    ✅    |
| Graph Visualization       |    ✅     |    ✅    |
| PostgreSQL                |    ✅     |    ✅    |
| Neo4J                     |    ❌     |    ✅    |
| MongoDB                   |    ❌     |    ✅    |
| Milvus                    |    ❌     |    ✅    |
| Docker Compose            |    ✅     |    ✅    |
| Kubernetes                |    ✅     |    ✅    |
| Cost Tracking             |    ✅     |    ⚠️    |
| Source Citations          |    ✅     |    ✅    |
| Document Deletion         |    ✅     |    ✅    |
| Entity Merging            |    ✅     |    ✅    |
| Multimodal (RAG-Anything) |    ❌     |    ✅    |
| Langfuse Tracing          |    ⚠️     |    ✅    |

---

## Community & Support

| Aspect            | EdgeQuake     | LightRAG                                |
| ----------------- | ------------- | --------------------------------------- |
| **GitHub Stars**  | Growing       | 27.7k+                                  |
| **Contributors**  | Active        | 216+                                    |
| **Discord**       | TBD           | [Active](https://discord.gg/yF2MmDJyGJ) |
| **Documentation** | Comprehensive | Comprehensive                           |
| **License**       | Apache-2.0    | MIT                                     |

---

## Summary

**EdgeQuake** is ideal for production deployments requiring:

- Type safety and performance guarantees
- Multi-tenant architecture
- PostgreSQL-centric infrastructure
- Single-binary deployment

**LightRAG (Python)** is ideal for:

- Rapid prototyping and research
- Python-centric teams
- Multi-backend flexibility
- Community integrations (RAG-Anything, Langfuse)

Both implement the same core algorithm, so query quality is equivalent. The choice depends on your deployment requirements and team expertise.

---

## See Also

- [LightRAG Algorithm Deep-Dive](../deep-dives/lightrag-algorithm.md)
- [vs GraphRAG](vs-graphrag.md) - Microsoft's approach
- [vs Traditional RAG](vs-traditional-rag.md) - Why graphs matter
