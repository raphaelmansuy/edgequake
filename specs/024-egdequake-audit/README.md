# SPEC-024 — EdgeQuake Ingestion & Query Pipeline Audit

**Date:** 2026-06-27  
**Method:** Code is law — findings cite source files only. Docs, ADRs, and marketing copy are ignored unless they contradict code (then code wins).  
**Commit audited:** `b868c76f` (BM25/FTS fusion, community labels, ingest hardening)

---

## Verdict (one paragraph)

EdgeQuake is a **serious LightRAG-on-Postgres implementation** with recent consolidation wins (unified `IngestionPersister`, batch graph/vector writes, FTS/RRF, Louvain labels). It is **not** GraphRAG, **not** SOTA neural-rerank RAG, and **not** a single ingestion path. Production quality is **bifurcated**: the async worker path (`text_insert.rs`) is mature; sync file upload, injection spawn, and library `insert` diverge in resilience, checkpointing, and failure semantics. Query default mode (`Hybrid`) uses round-robin merge — weaker than `Mix` with RRF. Several config fields (`graph_depth`, `max_results`) are dead code.

---

## Document Index

| ID | Lens | File | Primary crates |
|----|------|------|----------------|
| 001 | First principles | [001-first-principles-architecture.md](./001-first-principles-architecture.md) | all |
| 002 | Ingestion pipeline | [002-ingestion-pipeline-audit.md](./002-ingestion-pipeline-audit.md) | api, pipeline, core, tasks |
| 003 | Query / retrieval | [003-query-retrieval-audit.md](./003-query-retrieval-audit.md) | query, api, storage |
| 004 | LightRAG expert | [004-lightrag-expert-lens.md](./004-lightrag-expert-lens.md) | query, pipeline |
| 005 | GraphRAG expert | [005-graphrag-expert-lens.md](./005-graphrag-expert-lens.md) | query, storage |
| 006 | SOTA RAG expert (Jun 2026) | [006-sota-rag-expert-lens.md](./006-sota-rag-expert-lens.md) | query |
| 007 | Postgres / AGE / pgvector | [007-postgres-age-pgvector-lens.md](./007-postgres-age-pgvector-lens.md) | storage, api |
| 008 | System engineering | [008-system-engineering-lens.md](./008-system-engineering-lens.md) | api, tasks |
| 009 | O(n) complexity | [009-complexity-on-lens.md](./009-complexity-on-lens.md) | pipeline, query, storage |
| 010 | Rust / DRY / SOLID | [010-rust-solid-dry-lens.md](./010-rust-solid-dry-lens.md) | all |
| 011 | AI engineering | [011-ai-engineering-lens.md](./011-ai-engineering-lens.md) | pipeline, query, llm |
| 012 | Improvement plan | [012-improvement-plan.md](./012-improvement-plan.md) | — |

---

## Cross-Reference Matrix

Finding IDs are stable across documents.

| ID | Finding | 002 | 003 | 004 | 005 | 006 | 007 | 008 | 009 | 010 | 011 |
|----|---------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| F-01 | Four ingestion execution models | ✓ | | ✓ | | | | ✓ | ✓ | ✓ | ✓ |
| F-02 | `IngestionPersister` SSOT (P-G2) | ✓ | | ✓ | | | ✓ | ✓ | | ✓ | |
| F-03 | Community refresh on every ingest | ✓ | ✓ | | ✓ | | ✓ | ✓ | ✓ | | |
| F-04 | Global ≠ GraphRAG | | ✓ | ✓ | ✓ | ✓ | | | | | |
| F-05 | Hybrid round-robin default | | ✓ | ✓ | | ✓ | | | | ✓ | ✓ |
| F-06 | BM25/FTS only in naive mode | | ✓ | ✓ | | ✓ | ✓ | | | | ✓ |
| F-07 | Dead config: `graph_depth`, `max_results` | | ✓ | | | ✓ | | | | ✓ | |
| F-08 | Chunk content duplicated in vector metadata | ✓ | | ✓ | | | ✓ | | ✓ | | |
| F-09 | Global query cache invalidation | ✓ | ✓ | | | | | ✓ | ✓ | | |
| F-10 | Injection O(all KV keys) list/delete | ✓ | | | | | | ✓ | ✓ | ✓ | |
| F-11 | Cross-encoder reranker stub | | ✓ | | | ✓ | | | | | ✓ |
| F-12 | `vector_queries.rs` monolith (~800 LOC) | | ✓ | ✓ | | ✓ | | | ✓ | ✓ | |

---

## Severity Scale

| Level | Meaning |
|-------|---------|
| **P0** | Data loss, wrong tenant/workspace, or production outage at modest scale |
| **P1** | Correctness drift, silent quality regression, or >10× unnecessary cost |
| **P2** | Maintainability, dead config, suboptimal defaults |
| **P3** | Nice-to-have SOTA gap |

---

## Related Specs

- [017-dry-and-solid-audit](../017-dry-and-solid-audit/) — prior DRY/SOLID pass (partially addressed by SPEC-021/022/023)
- [018-observability](../018-observability/) — tracing gaps not re-audited here
