# 00 — Executive Brutal Audit (2026-06-27)

> **Spec**: 022-edgequake-study  
> **Method**: Code Is Law — every claim re-verified against `edgequake/crates/` on 2026-06-27  
> **Lens**: LightRAG algorithm fidelity, GraphRAG maturity, production systems engineering

---

## One-paragraph truth

Plan-19 **closed the async ingestion path** (orchestrator + `text_insert` → `IngestionPersister` → batched merger + saga). That work is real and tested. But the audit is **not complete** until you account for **two synchronous HTTP upload handlers** that still perform **~400 lines of hand-rolled, N+1 graph/vector writes** with **no merger, no saga, and no cache invalidation**. In production, users uploading via `POST /documents/upload` hit the **worst** path, not the **best** one. Query retrieval is in good shape after P-G3/P-G6/P-G9 fixes; remaining gaps are **parity** (orchestrator lacks BM25) and **intelligence** (flat LightRAG, no communities).

---

## Grades (four lenses)

| Lens | Grade | Brutal one-liner |
|------|-------|------------------|
| **LightRAG ingestion fidelity** | **B−** | Async path matches LightRAG merge semantics; sync upload bypasses merger entirely |
| **LightRAG query fidelity** | **A−** | Six modes, batch graph reads, SQL-layer vector type filter — solid |
| **GraphRAG / SOTA** | **C+** | Hygiene fixed; zero hierarchical communities or Leiden-style structure |
| **Production correctness** | **B** | New async writes: A. Sync upload writes: **F** (split-brain persistence) |
| **SOLID / DRY** | **B+** | Persister trait is excellent; upload handlers are a DRY catastrophe |
| **O(n) performance** | **B** | Batched where it matters (merger, global query); upload sync path is O(E) round-trips |
| **Postgres / AGE / pgvector** | **B** | Good DDL and batch UNNEST; pgvector 0.7.4 leaves filtered ANN recall on the floor |

---

## What we can claim (verified green)

| Claim | Evidence |
|-------|----------|
| Single persister for orchestrator + processor | `ingestion_persister.rs` — `DefaultIngestionPersister` |
| Saga compensation on merge failure | `compensate_merge_failure` in persister + orchestrator ordering comment |
| Entity identity SSOT for merger path | `EntityId` in `edgequake-storage/src/entity_id.rs` |
| Global query no N+1 | `node_degrees_batch` — `contract_global_no_nplus1.rs` |
| Query mode contracts | `contract_query_modes.rs` (engine-level) |
| Vector batch upsert (atomic tx) | `PgVectorStorage::upsert` — UNNEST + single commit |
| SQL metadata pre-filter for local/naive | `query_filtered` + `metadata_filter_sql.rs` |
| API query engine: BM25 + caches | `query_bootstrap.rs:35-51` |

---

## What we cannot claim (be honest)

| Gap | Impact |
|-----|--------|
| **`file_upload.rs` bypasses `IngestionPersister`** | Duplicate entities on re-upload; no description merge; no saga |
| **`batch_upload.rs` same bypass + global vectors** | Wrong tenant/workspace isolation |
| **No E2E: worker upload → Postgres UNNEST** | Production batch path untested end-to-end |
| **Orchestrator engine lacks BM25 reranker** | SDK/direct `EdgeQuake::query()` ≠ HTTP quality |
| **pgvector 0.7.4 in Docker** | `hnsw.iterative_scan` code exists but **disabled** at runtime |
| **GraphRAG communities** | Not implemented — flat entity graph only |
| **Mix mode HTTP weight ordering** | Engine tested; HTTP layer checks mode + stats only |
| **Legacy pre-G1 graphs** | Admin P-G1b only — no auto-heal |

---

## Severity-ranked new findings (RC-022)

| ID | Sev | Finding | File anchor |
|----|-----|---------|-------------|
| RC-022-1 | **CRITICAL** | Sync `upload_file` reimplements persistence — no merger, N+1, no saga | `file_upload.rs:289-503` |
| RC-022-2 | **HIGH** | `batch_upload` same anti-pattern + uses global `vector_storage` | `batch_upload.rs:192-211` |
| RC-022-3 | **HIGH** | Three ingestion persistence paths (should be 1) | See `01-ingestion` |
| RC-022-4 | **MEDIUM** | Orchestrator query engine missing BM25 | `orchestrator/mod.rs:519-528` vs `query_bootstrap.rs:45` |
| RC-022-5 | **MEDIUM** | pgvector 0.7.4 — iterative scan gated off | `Dockerfile.postgres:18`, `search_tuning.rs:79-91` |
| RC-022-6 | **MEDIUM** | AGE Cypher built via string interpolation | `nodes_ops.rs:13-24`, `cypher_exec.rs:33-37` |
| RC-022-7 | **LOW** | LLM summarization still O(E) LLM calls when enabled | `ingestion_persister.rs:262-267` |
| RC-022-8 | **LOW** | GraphRAG flat — no community detection | architectural |

Full traceability: [05-cross-reference-index.md](./05-cross-reference-index.md).

---

## Ship recommendation

```
┌─────────────────────────────────────────────────────────────┐
│  CONDITIONAL SHIP                                           │
│                                                             │
│  ✅ Ship async ingestion (text/markdown/PDF task queue)     │
│  ✅ Ship query API with current engine                      │
│  ❌ Do NOT treat sync file upload as production-grade       │
│     until P-H1 (route through IngestionPersister) lands     │
│                                                             │
│  Operator runbook: run P-G1b reconcile on legacy graphs   │
└─────────────────────────────────────────────────────────────┘
```

**Do not reopen SPEC-021 plan-19 wholesale.** Close RC-022-1/2 surgically via [06-improvement-plan.md](./06-improvement-plan.md) P-H1–P-H2.

---

## ASCII: the split-brain problem

```
                    User uploads PDF via WebUI
                              │
                              ▼
              ┌───────────────────────────────┐
              │  Which HTTP route?            │
              └───────────────┬───────────────┘
                              │
           ┌──────────────────┼──────────────────┐
           │                  │                  │
           ▼                  ▼                  ▼
    text/markdown       multipart           batch API
    (async 202)         upload_file         upload_files_batch
           │                  │                  │
           ▼                  ▼                  ▼
    text_insert         INLINE ~400 LOC     INLINE ~60 LOC
    IngestionPersister  upsert_node loop    global vectors
    KnowledgeGraphMerger per-chunk upsert    no workspace
    saga compensation   NO merger            NO merger
           │                  │                  │
           ▼                  ▼                  ▼
         ✅ A              ❌ F               ❌ D
```
