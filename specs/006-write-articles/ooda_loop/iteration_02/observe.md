# OODA Iteration 02 - Observe

## Mission Re-Read ✅

**Mission**: Write 15+ promotional articles for EdgeQuake
**Location**: `./articles/` with numbered subfolders
**Spec File**: `./specs/006-write-articles.md`
**Current Article**: 002_edgequake_approach - The EdgeQuake Approach: Graph-First RAG

---

## 🔭 Territory Mapping for Article 002

### Previous Article Context

Article 001 established:

- Classic RAG's 3 failures (lost relationships, no global view, no multi-hop)
- First principles explanation (embeddings lose structure)
- Knowledge graphs as the solution concept
- LightRAG paper validation

**Article 002 must answer**: "OK, so how does EdgeQuake actually solve this?"

### EdgeQuake Architecture (from codebase)

```
┌─────────────────────────────────────────────────────────────────┐
│                      EDGEQUAKE SYSTEM                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    RUST BACKEND (11 Crates)               │  │
│  ├───────────────────────────────────────────────────────────┤  │
│  │                                                            │  │
│  │  edgequake-core      │ Orchestration + EdgeQuake API      │  │
│  │  edgequake-llm       │ LLM providers (OpenAI, Mock)       │  │
│  │  edgequake-storage   │ Graph + Vector + KV storage        │  │
│  │  edgequake-pipeline  │ Chunking, Extraction, Merging      │  │
│  │  edgequake-query     │ 5 query modes                      │  │
│  │  edgequake-api       │ REST API (Axum)                    │  │
│  │  edgequake-pdf       │ PDF processing                     │  │
│  │                                                            │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│                              ▼                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    STORAGE LAYER                          │  │
│  │  PostgreSQL + Apache AGE (Graph) + pgvector (Embeddings)  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

### Key EdgeQuake Features (verified from README.md)

1. **High Performance**
   - Async-First (Tokio runtime)
   - Zero-Copy memory management
   - Parallel processing
   - Query latency <200ms

2. **Knowledge Graph**
   - Entity extraction (LLM-based)
   - Relationship mapping
   - Community detection
   - Graph visualization

3. **5 Query Modes**
   - Naive (~50ms)
   - Local (~150ms)
   - Global (~200ms)
   - Hybrid (~250ms)
   - Mix (~300ms)

4. **Production Ready**
   - REST API (OpenAPI 3.0)
   - SSE streaming
   - Multi-tenant
   - Docker/Kubernetes ready

### Pipeline Flow (from edgequake-pipeline/src/lib.rs)

```
Document → Chunks → LLM Extraction → Normalization → Merge → Storage
              │           │                │            │
              │           │                │            │
              ▼           ▼                ▼            ▼
         ChunkerConfig  ExtractedEntity  Normalized  GraphNode
         (1200 tokens,  (name, type,     (UPPERCASE_  + edges
          100 overlap)   description)    UNDERSCORED)
```

### Performance Benchmarks (from README.md)

| Metric                 | EdgeQuake        | Traditional RAG | Improvement |
| ---------------------- | ---------------- | --------------- | ----------- |
| Entity Extraction      | ~2-3x more       | Baseline        | 3x          |
| Query Latency (hybrid) | < 200ms          | ~1000ms         | 5x faster   |
| Document Processing    | 25s (10k tokens) | ~60s            | 2.4x faster |
| Concurrent Users       | 1000+            | ~100            | 10x         |
| Memory Usage           | 2MB per doc      | ~8MB            | 4x better   |

### Technology Stack

- **Language**: Rust (1.78+)
- **Runtime**: Tokio (async)
- **Web Framework**: Axum
- **Graph Database**: PostgreSQL + Apache AGE
- **Vector Storage**: pgvector
- **LLM Providers**: OpenAI, Ollama, Mock
- **Frontend**: React 19 + TypeScript

---

## Key Messages for Article 002

1. EdgeQuake is a **production-ready Rust implementation** of Graph-RAG
2. The **pipeline architecture** transforms documents into knowledge graphs
3. **5 query modes** handle different use cases (speed vs comprehensiveness)
4. Built for **scale**: multi-tenant, concurrent, memory-efficient
5. **Open source** and ready to deploy
