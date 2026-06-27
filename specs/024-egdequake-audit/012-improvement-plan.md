# 012 — Consolidated Improvement Plan

**Cross-ref:** All lenses · [README finding IDs](./README.md#cross-reference-matrix)

---

## Executive Summary

EdgeQuake has **finished the hard part of persist unification** (P-G2) and **started SOTA retrieval features** (FTS, RRF, community labels). The next phase is **path unification**, **scale invariants**, and **retrieval defaults** — not more features.

```text
  NOW                          NEXT                         LATER
  ───                          ────                         ─────
  4 ingest paths               1 async path                 GraphRAG summaries
  Louvain per doc              debounced index              agentic retrieval
  Hybrid default               Mix+RRF default              HyDE / decomposition
  BM25 rerank only             cross-encoder                learned fusion
  dead config fields           wire or delete               incremental Louvain
```

---

## Phase 1 — Stop the Bleeding (P0, 2–4 weeks)

**Goal:** Eliminate correctness and scale cliffs without algorithm changes.

| # | Action | Finds | Files | Acceptance |
|---|--------|-------|-------|------------|
| 1.1 | Route `file_upload` + `batch_upload` through `TaskRuntime` (202 + task) | F-01 | `file_upload.rs`, `batch_upload.rs`, `text_upload.rs` pattern | E2E: upload returns 202; same checkpoint/resilience as text |
| 1.2 | Replace injection `tokio::spawn` with queued task type | F-01 | `injection.rs`, `edgequake-tasks` | Injection visible in tasks table; retriable |
| 1.3 | Debounce `refresh_community_index` (e.g. 5min coalesce per workspace) | F-03 | `community_persist.rs`, new `CommunityIndexScheduler` | Ingest 100 docs → ≤1 Louvain run |
| 1.4 | Workspace-scoped query cache invalidation | F-09 | `query_result_cache.rs`, `ingestion_persist.rs` | Ingest in ws-A does not bust ws-B cache |
| 1.5 | Enforce `strict_workspace_mode=true` in production bootstrap | F-01 | `processor/mod.rs`, `state/postgres.rs` | Misconfigured workspace → task fail, not silent fallback |
| 1.6 | Injection list/delete: prefix index or Postgres table | F-10 | `injection.rs`, migration | O(injections) not O(all keys) |

### Phase 1 ASCII — Target Ingest Topology

```text
  ALL HTTP UPLOADS                    WORKER (single path)
  ───────────────                     ────────────────────

  text ──────┐
  file ──────┼──> enqueue Task ──> text_insert.rs
  batch ─────┤         │
  pdf ───────┤         v
  injection ─┘    process_with_resilience_cancellable
                       │
                       v
                  IngestionPersister
                       │
                       v
                  debounced community index
```

---

## Phase 2 — Retrieval & Storage Hardening (P1, 4–6 weeks)

| # | Action | Finds | Files | Acceptance |
|---|--------|-------|-------|------------|
| 2.1 | Default query mode → `Mix`; default fusion → RRF | F-05 | `engine_impl/mod.rs`, API handler default | Hybrid still available; Mix is default |
| 2.2 | Rename or document Hybrid as `Interleave` | F-05 | `modes.rs`, API | No semantic confusion |
| 2.3 | Extend BM25/FTS fusion to local + global chunk stages | F-06 | `vector_queries.rs`, `sparse_retrieval.rs` | Contract tests for local/global sparse |
| 2.4 | Implement cross-encoder reranker | F-11 | `bootstrap.rs`, `reranking.rs` | `EDGEQUAKE_RERANKER=cross_encoder` works |
| 2.5 | Deduplicate chunk storage: store content once, reference by chunk_id in vector metadata | F-08 | persister, KV builder, migration | 50%+ metadata size reduction on sample doc |
| 2.6 | Wire `max_results` to `max_chunks` or remove from API | F-07 | `types.rs`, `query_execute.rs` | API param has effect or 400 if deprecated |
| 2.7 | Delete or implement `graph_depth` multi-hop traversal | F-07 | `vector_queries.rs` | Either BFS to depth N or field removed |
| 2.8 | Split `vector_queries.rs` into per-mode modules | F-12 | new `engine_impl/modes/*` | Each mode <200 LOC |

---

## Phase 3 — GraphRAG / SOTA Optional Track (P2–P3, 8+ weeks)

Only if product requires GraphRAG parity:

| # | Action | Finds |
|---|--------|-------|
| 3.1 | Community summary generation at index time (LLM per community) | F-04 |
| 3.2 | New `vector_type=community_summary` + global query map-reduce path | F-04 |
| 3.3 | Indexed `community_id` lookup (Cypher or relational) replacing popular scan | F-04 |
| 3.4 | Gold benchmark corpus (50–200 Q&A) in CI with recall@k | 006 |
| 3.5 | Optional HyDE (`EDGEQUAKE_QUERY_HYDE=true`) | 006 |

---

## Phase 4 — Operational Excellence (P2, ongoing)

| # | Action |
|---|--------|
| 4.1 | Ingest LLM cost estimator in admission API response |
| 4.2 | Task queue depth metric + alert threshold |
| 4.3 | KV/Postgres outbox for document metadata consistency |
| 4.4 | Remove dead `strategies/` module or bench-only gate |
| 4.5 | Shrink `migration_bootstrap.rs` into focused modules |

---

## Risk Register

| Risk if deferred | Impact | Phase |
|------------------|--------|-------|
| Louvain per ingest at 100k nodes | Ingest timeout cascade, worker circuit breaker trips | 1.3 |
| Sync upload on large PDF | Gateway timeout, orphan KV | 1.1 |
| Hybrid default in production | Suboptimal answers vs Mix+RRF | 2.1 |
| Injection KV scan | Admin API timeout | 1.6 |
| No cross-encoder | Plateau on semantic retrieval quality | 2.4 |

---

## Success Metrics (code-verifiable)

| Metric | Baseline (now) | Target (Phase 2) |
|--------|----------------|------------------|
| Ingest execution paths | 4 | 1 (+ library) |
| Louvain runs per 100 ingests | ~100 | ≤2 (debounced) |
| Query cache hit rate under ingest | ~0% (global bust) | >50% cross-workspace |
| Default fusion | round-robin | RRF |
| Dead config fields | 2 | 0 |
| `vector_queries.rs` LOC | ~801 | <200 per module |
| Cross-encoder rerank | stub | implemented |

---

## What NOT to Do

1. **Do not** market Global mode as GraphRAG without Phase 3.
2. **Do not** add new query modes before splitting `vector_queries.rs`.
3. **Do not** add more ingestion entry points — redirect to queue.
4. **Do not** run full Louvain synchronously on persist hook at scale.

---

## Lens Grades Summary

| Lens | Grade |
|------|:-----:|
| First principles architecture | C+ |
| Ingestion pipeline | B- (worker) / D (uniformity) |
| Query retrieval | B- |
| LightRAG expert | B- |
| GraphRAG expert | D+ |
| SOTA RAG (Jun 2026) | C+ |
| Postgres/AGE/pgvector | B+ |
| System engineering | B- |
| O(n) complexity | C |
| Rust/SOLID/DRY | B- |
| AI engineering | B- |

**Overall: B- engineering, C+ algorithmic retrieval defaults, not production-ready at 100k+ node scale without Phase 1.**
