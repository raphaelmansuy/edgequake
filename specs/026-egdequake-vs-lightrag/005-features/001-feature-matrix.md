# 001 — Feature Matrix

**Cross-ref:** [003 Ingestion](../003-ingestion/001-ingestion-comparison.md) · [004 Query](../004-query/001-query-comparison.md) · [006 Robustness](../006-robustness/001-robustness-comparison.md)

**Findings:** C-03, C-04, C-12

---

## Legend

| Symbol | Meaning |
|:------:|---------|
| ✓ | Production-ready |
| △ | Partial / extension / deviation |
| ✗ | Missing |
| EQ+ | EdgeQuake extension beyond LightRAG |
| LR+ | LightRAG-only |

---

## Core RAG Features

| Feature | LightRAG | EdgeQuake | Notes |
|---------|:--------:|:---------:|-------|
| Entity extraction | ✓ | ✓ | Parity |
| Relationship extraction | ✓ | ✓ | Parity |
| Gleaning | ✓ | ✓ | Parity (post SPEC-025) |
| Knowledge graph | ✓ | ✓ | NetworkX vs AGE |
| Chunk vectors | ✓ | ✓ | |
| Entity vectors | ✓ | ✓ | |
| Relationship vectors | ✓ | ✓ | |
| Dual-level keywords | ✓ | ✓ | |
| Query modes (6) | ✓ | ✓ | |
| Streaming answers | ✓ | ✓ | |
| Document deletion cascade | ✓ | ✓ | |
| Entity merge/edit API | ✓ LR+ | △ | LR richer graph edit |

---

## Retrieval Extensions

| Feature | LightRAG | EdgeQuake | Notes |
|---------|:--------:|:---------:|-------|
| BM25 / FTS | ✗ | ✓ EQ+ | Postgres tsvector |
| RRF fusion | ✗ | ✓ EQ+ | Mix default |
| Cross-encoder rerank | △ external | ✓ EQ+ | env-configured |
| Graph multi-hop | △ 1-hop | ✓ EQ+ | `graph_depth` |
| Intent routing | ✗ | ✓ EQ+ | cost control |
| Conversation history | ✗ | ✓ EQ+ | multi-turn |
| Community index | ✗ | △ EQ+ | Louvain ingest |
| Hybrid round-robin | ✓ | ✓ | Parity |

---

## Ingestion Features

| Feature | LightRAG | EdgeQuake | Notes |
|---------|:--------:|:---------:|-------|
| Plain text insert | ✓ | ✓ | |
| File upload API | ✓ | ✓ | |
| Batch upload | ✓ | ✓ | |
| PDF parsing | △ multi-engine | ✓ embedded pdfium | Different approach |
| DOCX native | ✓ LR+ | ✗ | **C-12** |
| Markdown IR | ✓ LR+ | △ | |
| MinerU / Docling | ✓ LR+ | ✗ | **C-03** |
| Multimodal VLM | ✓ LR+ | ✗ | **C-12** |
| Semantic chunking | ✓ LR+ | ✗ | **C-03** |
| Knowledge injection | ✗ | ✓ EQ+ | EdgeQuake extension |
| Async worker queue | △ pipeline | ✓ durable PG | **C-05** |
| Upload dedup by hash | ✓ | ✓ | |
| Reingest policy | ✓ | ✓ | |

---

## Storage Features (C-04)

| Backend | LightRAG | EdgeQuake |
|---------|:--------:|:---------:|
| JSON file KV | ✓ | ✗ |
| NetworkX graph | ✓ | ✗ |
| PostgreSQL | ✓ | ✓ **required** |
| Apache AGE | ✗ | ✓ EQ+ |
| pgvector | △ via postgres | ✓ native |
| Neo4j | ✓ LR+ | ✗ |
| Milvus | ✓ LR+ | ✗ |
| Qdrant | ✓ LR+ | ✗ |
| MongoDB | ✓ LR+ | ✗ |
| Redis | ✓ LR+ | ✗ |
| FAISS | ✓ LR+ | ✗ |
| OpenSearch | ✓ LR+ | ✗ |

LightRAG: **13 storage implementations** (`lightrag/kg/*_impl.py`).  
EdgeQuake: **1 production stack** — intentional constraint.

---

## Platform Features

| Feature | LightRAG | EdgeQuake | Notes |
|---------|:--------:|:---------:|-------|
| REST API server | ✓ FastAPI | ✓ Axum | |
| Web UI | ✓ lightrag_webui | ✓ edgequake_webui | |
| Multi-workspace | ✓ namespace | ✓ UUID + tenant | |
| Multi-tenant auth | △ | ✓ | |
| Health endpoint | △ | ✓ rich | components breakdown |
| OTEL observability | △ | △ | both incomplete |
| Task recovery | △ | ✓ | orphan tasks/docs |
| Rate limiting | △ | ✓ | edgequake-rate-limiter |
| SDK (Python/TS) | △ | △ | both evolving |

---

## Feature Count Summary

```text
  Category              LightRAG-only    EdgeQuake-only    Shared
  ────────              ─────────────    ──────────────    ──────
  Parsers/formats            8                 1              3
  Storage backends          11                 1              1
  Retrieval extensions       0                 6              6
  Platform/ops               1                 4              5
  ─────────────────────────────────────────────────────────────
  Total unique advantages   ~20               ~12            ~15 core
```

**Brutal truth:** LightRAG is a **wider product**. EdgeQuake is a **deeper Postgres RAG engine** with fewer format adapters.

---

## Category Mislabeling Warning

| Claim | LightRAG | EdgeQuake |
|-------|:--------:|:---------:|
| "GraphRAG" | ✗ misleading | ✗ misleading |
| "LightRAG-compatible" | ref | ✓ core modes |
| "Production-ready" | △ depends on backend | ✓ Postgres path |
| "Multimodal RAG" | ✓ | ✗ |

Do not market EdgeQuake Global mode as GraphRAG. Do not market LightRAG JSON+NetworkX as production Postgres.
