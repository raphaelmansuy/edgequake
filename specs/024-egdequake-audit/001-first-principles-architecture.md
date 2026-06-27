# 001 — First Principles Architecture

**Cross-ref:** [README](./README.md) · [002 Ingestion](./002-ingestion-pipeline-audit.md) · [003 Query](./003-query-retrieval-audit.md) · [012 Plan](./012-improvement-plan.md)

---

## 1. What EdgeQuake Actually Is (from code)

EdgeQuake is a **document → knowledge graph + multi-vector index → multi-mode RAG query** system implemented in Rust with **PostgreSQL as mandatory storage** (pgvector + Apache AGE + KV tables + task queue).

It is **not** a single pipeline. It is **four ingestion runtimes** converging on one persist trait, plus **one query engine** with six retrieval modes.

```text
                         INGESTION (4 paths)
    ┌──────────────────────────────────────────────────────────────┐
    │                                                              │
    │  [A] Async worker     text_upload / PDF → TaskQueue          │
    │  [B] Sync HTTP        file_upload / batch_upload             │
    │  [C] Detached spawn   injection (tokio::spawn)               │
    │  [D] Library          EdgeQuake::insert / insert_batch       │
    │                                                              │
    └──────────────────────────┬───────────────────────────────────┘
                               │
                               v
              ┌────────────────────────────────────┐
              │  Pipeline (chunk → LLM extract)    │
              │  process | process_with_resilience │
              └────────────────┬───────────────────┘
                               │
                               v
              ┌────────────────────────────────────┐
              │  IngestionPersister (SSOT)         │
              │  1. vector upsert (chunks)         │
              │  2. KnowledgeGraphMerger           │
              │  3. refresh_community_index        │
              │  4. compensate on merge failure    │
              └────────────────┬───────────────────┘
                               │
                               v
              ┌────────────────────────────────────┐
              │  invalidate_query_result_cache   │
              └────────────────────────────────────┘


                         QUERY (1 engine)
    ┌──────────────────────────────────────────────────────────────┐
    │  POST /api/v1/query → QueryEngine::query_with_full_config  │
    │                                                              │
    │  prepare: keywords + embeddings + mode selection           │
    │  retrieve: naive | local | global | hybrid | mix | bypass  │
    │  finalize: rerank (BM25) → truncate → LLM answer             │
    └──────────────────────────────────────────────────────────────┘
```

**First-principle invariant:** Retrieval quality is bounded by **what was indexed** (entity/rel/chunk vectors + graph edges + optional FTS column + community_id labels). Query modes are **views** over the same index — not separate indexes.

---

## 2. Single Sources of Truth (what code enforces)

| Concern | SSOT location | Violations |
|---------|---------------|------------|
| Persist sequence | `edgequake-pipeline/src/persistence/ingestion_persister.rs` | Pre-P-G2 paths removed; **entry execution still diverges** |
| Query modes enum | `edgequake-query/src/modes.rs` | API parses same strings; adaptive intent can override |
| Vector ANN | `edgequake-storage/.../vector/storage_impl.rs` | Workspace-scoped tables via registry |
| Graph merge | `edgequake-pipeline/src/merger/` | Batch upsert; relational sink still per-entity |
| Production query engine | `edgequake-query/src/engine_impl/` | `strategies/` appears unused on hot path |
| Community labels | `edgequake-storage/src/community_persist.rs` | Louvain at **ingest**, not query |

---

## 3. Trust Boundaries

```text
  Client ──HTTP──> edgequake-api (auth, limits, admission)
                        │
                        ├──> KV (document metadata, chunks, hashes)
                        ├──> Task queue (async ingest)
                        ├──> Pipeline + LLM provider
                        ├──> VectorStorage (pgvector per workspace)
                        ├──> GraphStorage (AGE Cypher)
                        └──> QueryEngine (read-only retrieval + LLM gen)
```

**Critical boundary failure:** Non-strict workspace mode can silently use global Ollama pipeline (`processor/mod.rs` docs). Injection can fall back to global vector storage on lookup failure (`handlers/injection.rs`). Both violate tenant isolation **when misconfigured**.

---

## 4. Saga vs Transaction

Code implements **compensating saga**, not 2PC:

1. Upsert chunk vectors (Postgres transaction per batch)
2. Merge graph (AGE batch Cypher)
3. On merge failure → `compensate_merge_failure` deletes vectors/edges

**Gap:** KV chunk records and content-hash mappings are **outside** the saga. Sync upload that fails at persist leaves orphan KV state (F-01, see 002).

---

## 5. First-Principles Quality Scorecard

| Dimension | Score (1–5) | Evidence |
|-----------|:-------------:|----------|
| Index correctness | 4 | Provenance-based chunk hydration; `vector_type=chunk` filter |
| Ingestion consistency | 2 | Four paths, different resilience |
| Query mode honesty | 4 | `modes.rs` explicitly says Global ≠ GraphRAG |
| Operational scale | 2 | Community refresh per doc; global cache invalidation |
| Code consolidation | 3 | P-G2 persist SSOT; `vector_queries.rs` still monolithic |
| SOTA retrieval | 2 | BM25 rerank only; RRF opt-in; no cross-encoder |

---

## 6. Non-Negotiable Fixes (first principles)

If you accept only three changes:

1. **One ingestion runtime** — all HTTP uploads enqueue worker tasks (same as text/PDF).
2. **Amortize community index** — debounce/batch Louvain; never O(graph) per document.
3. **Default to Mix+RRF** — Hybrid round-robin is not score-based fusion.

See [012-improvement-plan.md](./012-improvement-plan.md) for phased execution.
