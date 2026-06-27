# 003 — Query & Retrieval Pipeline Audit

**Cross-ref:** [F-04,F-05,F-06,F-07,F-09,F-11,F-12](./README.md#cross-reference-matrix) · [006 SOTA](./006-sota-rag-expert-lens.md) · [005 GraphRAG](./005-graphrag-expert-lens.md)

---

## HTTP → Engine Path

```text
  POST /api/v1/query
       │
       v
  query_execute.rs
       │  validate_query, parse QueryMode (default: Hybrid)
       │  resolve_query_workspace, document_filter
       v
  query_execution.rs → execute_sota_query_with_auth_fallback
       │
       v
  QueryEngine::query_with_full_config  (engine_impl/)
       │
       ├── pipeline_prepare   (keywords, embeddings, mode)
       ├── pipeline_retrieve  (vector_queries.rs)
       └── pipeline_finalize (rerank, truncate, LLM)
```

Naming note: `execute_sota_query` is **routing label only** — production SSOT is `engine_impl`, not a separate SOTA engine crate.

---

## Query Pipeline Phases

File: `edgequake-query/src/engine_impl/query_entry/query_pipeline.rs`

```text
  PREPARE                          RETRIEVE                    FINALIZE
  ───────                          ────────                    ────────
  parallel:                        by QueryMode:               document_id filter
    keyword LLM extract              Naive  ──> chunk ANN       BM25 rerank_chunks
    embed_one(query)                 Local  ──> entity ANN     sort entities by degree
  validate_keywords (graph labels)     Global ──> rel ANN       balance_context (30k tok)
  QueryEmbeddings (hi/lo/query)        Hybrid ──> 3-way parallel
  mode: explicit | adaptive | default Mix    ──> weighted/RRF  generate_answer
```

---

## Mode Semantics (code truth)

From `modes.rs` line 31: **Global is relationship-vector search; not GraphRAG community reports.**

| Mode | Vector target | Graph ops | Chunk source |
|------|---------------|-----------|--------------|
| Naive | `type=chunk` ANN | none | direct ANN hits |
| Local | low-level → entity vectors | batch nodes/edges | provenance IDs → re-rank |
| Global | high-level → rel vectors | batch + community expand | provenance IDs |
| Hybrid | all three parallel | via local+global | **round-robin interleave** |
| Mix | all three parallel | via local+global | min-max weights or RRF |
| Bypass | skip | skip | direct LLM |

**Finding F-05 (P1):** Default mode is **Hybrid** (round-robin), not **Mix** (score fusion). Round-robin is not retrieval fusion — it is **deterministic interleaving without score normalization**.

---

## Dense + Sparse (F-06)

Only **naive mode** fuses BM25/FTS:

`sparse_retrieval.rs`:
1. ANN fetches `max_chunks × bm25_candidate_multiplier` candidates
2. Postgres: `text_search_filtered` (`fts.rs` — `ts_rank_cd` on GIN `content_tsv`)
3. Fallback: in-memory BM25 reranker over ANN candidates
4. Fusion: RRF if `EDGEQUAKE_MIX_FUSION=rrf`, else **sparse order wins entirely** (lines 99–104)

Local and global modes get **dense-only** retrieval. Misses standard hybrid search at entity/relationship stage.

---

## Community "Global" Expansion (F-04)

`community_global.rs`:
```text
  seed entities from rel ANN
       │
       v
  read community_id from node properties (index-time Louvain)
       │
       v
  get_popular_nodes_with_degree(max_entities * 2)
       │
       v
  scan popular nodes for matching community_id  ──> O(popular sample)
```

Comment in file: "no Louvain at query time" — correct. Expansion is **linear scan of popular nodes**, not indexed community lookup.

---

## Reranking (F-11)

`bootstrap.rs`:
- Default: `BM25Reranker::new_enhanced()`
- `EDGEQUAKE_RERANKER=cross_encoder` → **warns and falls back to BM25**

Same BM25 instance used for:
1. Post-retrieval rerank (`reranking.rs`)
2. Sparse rank generation (`sparse_retrieval.rs`)

Conflates **retrieval ranker** and **context reranker** roles.

---

## Caching (F-09)

| Cache | Scope | Key |
|-------|-------|-----|
| Query result | `context_only` requests only | query + mode + doc filter + mix weights |
| Embedding | LRU + TTL | model version + text |
| Keywords | TTL | query hash |

Full RAG answers **never cached** (LLM non-determinism). Ingestion invalidates **entire** result cache — destroys hit rate under load.

---

## Dead Config (F-07)

| Field | Defined | Used in retrieval |
|-------|---------|-------------------|
| `max_results` | `types.rs`, API DTO | **Never** — only `max_chunks` config applies |
| `graph_depth` | `QueryEngineConfig` default 2 | **Never read** in `vector_queries.rs` |

No multi-hop graph traversal despite config implying depth.

---

## Storage Interaction

**Vector ANN** (`storage_impl.rs`):
- Score: `1 - (embedding <=> query)`
- `SET LOCAL hnsw.ef_search`, `ivfflat.probes`, iterative scan for filters
- `MetadataFilter` pushed to SQL (tenant, workspace, vector_type, document_ids)

**Provenance chunk hydration** (local/global):
- Collect chunk IDs from entity/relationship metadata
- `query_filtered` with ID restriction — fixes entity-vector-dominated top-k bug

---

## Monolith Risk (F-12)

`vector_queries.rs` ~801 LOC — all modes, fallbacks, parallel arms, community hooks. High regression surface. SPEC-017 DRY cleaned pipeline orchestration but **not mode retrieval**.

---

## Query Grade

| Criterion | Grade |
|-----------|:-----:|
| LightRAG fidelity | B+ |
| Fusion quality | C (Hybrid default) |
| Graph traversal | D (1-hop only, dead graph_depth) |
| Sparse retrieval | B (naive only) |
| Config hygiene | D (dead fields) |
| Postgres tuning | A- |

**Bottom line:** Strong engineering on Postgres ANN + provenance chunks. **Weakest link:** default Hybrid merge and BM25-only rerank. Switch defaults to Mix+RRF; wire or delete dead config.
