# 001 — First Principles Comparison

**Cross-ref:** [README](../README.md) · [002 Algorithms](../002-algorithms/001-algorithm-comparison.md) · [009 Plan](../009-improvement-plan/001-improvement-plan.md)

---

## 1. What Is LightRAG? (from code, not paper)

LightRAG is a **dual-level graph-augmented RAG** system:

```text
  INGEST                         QUERY
  ─────                          ─────

  Document                       User question
      │                              │
      ▼                              ▼
  Chunk (token-budget)           LLM keyword extract
      │                          hi-level + lo-level
      ▼                              │
  LLM entity/rel extract             ├── LOCAL  → entity VDB → graph → chunks
      │                              ├── GLOBAL → rel VDB   → graph → chunks
      ▼                              ├── NAIVE  → chunk VDB direct
  Merge into KG                    ├── HYBRID → round-robin local+global
  + 3 vector indices               └── MIX    → hybrid + naive vector arm
      │
      └── entities_vdb
      └── relationships_vdb
      └── chunks_vdb
      └── text_chunks KV
      └── graph storage
```

**Source:** `LightRAG/lightrag/operate.py` (`extract_entities`, `merge_nodes_and_edges`, `kg_query`, `naive_query`), `lightrag/base.py` (`QueryParam.mode`).

**First principle:** Retrieval is **provenance-based** in graph modes — chunks come from entity/relationship `source_id` links, not direct chunk embedding alone.

---

## 2. What Is EdgeQuake?

EdgeQuake is a **Rust port of the LightRAG retrieval model** with **Postgres-native persistence** and **production API surface**:

```text
  INGEST (HTTP)                  QUERY (HTTP/SDK)
  ─────────────                  ────────────────

  POST /documents                POST /query
      │                              │
      ▼                              ▼
  document_admission SSOT        query_pipeline SSOT
  KV pre-write + task enqueue        │
      │                              ├── prepare (keywords ∥ embed ∥ intent)
      ▼                              ├── retrieve (6 modes)
  WorkerPool                         └── finalize (rerank → truncate → LLM)
      │
      ▼
  build_ingestion_pipeline (adaptive chunk + gleaning)
      │
      ▼
  DefaultIngestionPersister
  KV → pgvector → AGE merge → debounced Louvain
```

**Source:** `edgequake-api/src/handlers/documents/upload/document_admission.rs`, `edgequake-pipeline/src/ingestion_pipeline.rs`, `edgequake-query/src/engine_impl/query_entry/query_pipeline.rs`, `edgequake-pipeline/src/persistence/ingestion_persister.rs`.

**First principle:** Same provenance retrieval model; **one SSOT per concern** enforced by crate boundaries.

---

## 3. Architectural Divergence (honest)

```text
                    LightRAG                    EdgeQuake
                    ────────                    ─────────
  Language          Python 3                   Rust
  Primary deploy    pip + optional API           Axum + mandatory Postgres
  Storage default   JSON KV + NetworkX           AGE + pgvector + KV tables
  Storage options   13 backends                  1 production stack
  Ingest entry      LightRAG.insert() / pipeline  HTTP worker queue (+ SDK)
  Parser layer      Rich (MD/DOCX/PDF/VLM)         PDF→MD + plain text
  Chunking          4 strategies (F/R/V/P)       Adaptive fixed (600/800/1200)
  Tenancy           Workspace namespace          Tenant + workspace UUID
  Community         None at query                Louvain @ ingest (debounced)
  Eval harness      None in repo                 50-case golden skeleton
```

---

## 4. Shared Invariants (both must hold)

| Invariant | LightRAG | EdgeQuake |
|-----------|:--------:|:---------:|
| Entity names normalized | ✓ UPPERCASE | ✓ UPPERCASE_UNDERSCORE |
| Three vector types at index | ✓ | ✓ |
| Dual-level keywords at query | ✓ | ✓ |
| Graph modes use provenance chunks | ✓ | ✓ |
| Merge deduplicates entities across docs | ✓ | ✓ |
| Token budget caps context | ~30K | 10K/10K/10K split |

**Violation of any invariant = not LightRAG-class.**

EdgeQuake passes all six (verified SPEC-025, reconfirmed SPEC-026).

---

## 5. First-Principles Scorecard

| Principle | LightRAG | EdgeQuake | Notes |
|-----------|:--------:|:---------:|-------|
| Single insert brain | △ pipeline mixin | ✅ worker SSOT | EQ fixed N-02 in SPEC-025 |
| Single persist brain | △ scattered in operate | ✅ IngestionPersister | EQ cleaner |
| Single query brain | ✅ kg_query + naive_query | ✅ run_query_pipeline | EQ more modular |
| Storage truth | △ pluggable = drift risk | ✅ one stack | EQ trades flexibility |
| API honesty | ✅ fields used | ✅ post N-01 fix | Both OK now |
| Measure quality | ✗ no eval | △ skeleton | EQ slight edge |

---

## 6. Brutal Summary

**EdgeQuake is not "LightRAG but Rust."** It is LightRAG's **retrieval kernel** hardened for Postgres multi-tenant production, with deliberate **feature subtraction** (parsers, storage backends) and **feature addition** (BM25, RRF, rerank, ops).

**LightRAG is not "research toy."** It is a **format-agnostic ingestion laboratory** with 13 storage adapters and 5995 lines of battle-tested merge/query logic in `operate.py`.

Choose LightRAG when you need **format breadth and storage choice**. Choose EdgeQuake when you need **one durable Postgres stack and richer retrieval defaults**.
