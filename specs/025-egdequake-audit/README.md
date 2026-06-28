# SPEC-025 — EdgeQuake Ingestion & Query Pipeline Audit (Post-SPEC-024)

**Date:** 2026-06-27  
**Method:** Code is law — findings cite source only. SPEC-024 claims verified against working tree.  
**Commit audited:** `2afb4844` — `feat(spec-024): unify ingestion, hybrid query modes, and operational hardening`  
**Prior audit:** [SPEC-024](../024-egdequake-audit/) (baseline + implementation tracker)

---

## Verdict (brutal, one paragraph)

SPEC-024 **delivered**. EdgeQuake is now a **production-credible LightRAG-on-Postgres** system: one worker ingestion path, debounced Louvain, workspace-scoped cache bust, chunk KV SSOT, Hybrid round-robin parity, Mix+RRF default, BM25 on all arms, cross-encoder reranker, graph multi-hop, and operator-grade `/health`. That is real engineering — not marketing.

What remains is **honesty about category**:

- **Not GraphRAG.** Global mode is relationship-vector search + index-time `community_id` hints. No community reports, no map-reduce, no hierarchical summarization.
- **Not SOTA June 2026.** No agentic loops, no query decomposition, no faithfulness gates, no conversation-aware retrieval. Default Mix runs **three full retrieval arms in parallel** — quality up, cost up.
- **Not one brain.** Library `EdgeQuake::insert()` still gets adaptive chunking + Gleaning; HTTP worker path does not. API accepts `conversation_history` and **ignores it** in the engine.
- **Not O(1) at scale.** Prefix scans, graph BFS N+1, task payload duplication, and triple-arm queries are documented debt — not surprises waiting in prod if you load-test first.

**Overall grade: A- as LightRAG infrastructure, C+ as GraphRAG, B as June-2026 SOTA RAG.**

**Phase 5 (2026-06-27):** Conversation history wired, ingestion pipeline SSOT (adaptive chunk + gleaning on worker), upload admission DRY, QueryMode serde default aligned. See [012-improvement-plan.md](./012-improvement-plan.md).

**Phase 6 Sprint 1 (2026-06-27):** Slim task payload (KV ref), batch graph incident edges, cheap intent routing.

**Phase 6 Sprint 2–3 (2026-06-27):** Community_id push-down filter, RAGAS skeleton (50 Q&A), injection pagination, `text_insert/` SRP split. See [012-improvement-plan.md](./012-improvement-plan.md).

---

## Document Index

| ID | Lens | File | Primary crates |
|----|------|------|----------------|
| 001 | First principles | [001-first-principles-architecture.md](./001-first-principles-architecture.md) | all |
| 002 | Ingestion pipeline | [002-ingestion-pipeline-audit.md](./002-ingestion-pipeline-audit.md) | api, pipeline, core, tasks |
| 003 | Query / retrieval | [003-query-retrieval-audit.md](./003-query-retrieval-audit.md) | query, api, storage |
| 004 | LightRAG expert | [004-lightrag-expert-lens.md](./004-lightrag-expert-lens.md) | query, pipeline |
| 005 | GraphRAG expert | [005-graphrag-expert-lens.md](./005-graphrag-expert-lens.md) | query, storage |
| 006 | SOTA RAG expert (Jun 2026) | [006-sota-rag-expert-lens.md](./006-sota-rag-expert-lens.md) | query, llm |
| 007 | Postgres / AGE / pgvector | [007-postgres-age-pgvector-lens.md](./007-postgres-age-pgvector-lens.md) | storage, api |
| 008 | System engineering | [008-system-engineering-lens.md](./008-system-engineering-lens.md) | api, tasks, observability |
| 009 | O(n) complexity | [009-complexity-on-lens.md](./009-complexity-on-lens.md) | pipeline, query, storage |
| 010 | Rust / DRY / SOLID | [010-rust-solid-dry-lens.md](./010-rust-solid-dry-lens.md) | all |
| 011 | AI engineering | [011-ai-engineering-lens.md](./011-ai-engineering-lens.md) | pipeline, query, llm |
| 012 | Improvement plan | [012-improvement-plan.md](./012-improvement-plan.md) | — |

---

## Cross-Reference Matrix

Finding IDs are stable across SPEC-025 documents.  
✅ = resolved since SPEC-024 baseline · ⚠ = partial · ✗ = open

| ID | Finding | Sev | 002 | 003 | 004 | 005 | 006 | 007 | 008 | 009 | 010 | 011 | Status |
|----|---------|:---:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:------:|
| R-01 | Worker queue SSOT for HTTP ingest | P0 | ✓ | | ✓ | | | | ✓ | | ✓ | | ✅ |
| R-02 | `IngestionPersister` cross-store sequence | P0 | ✓ | | ✓ | | | ✓ | ✓ | | ✓ | | ✅ |
| R-03 | Debounced community index | P1 | ✓ | ✓ | | ✓ | | ✓ | ✓ | ✓ | | | ✅ |
| R-04 | Global ≠ GraphRAG | P2 | | ✓ | ✓ | ✓ | ✓ | | | | | ✓ | ✗ by design |
| R-05 | Mix default + RRF fusion | P2 | | ✓ | ✓ | | ✓ | | | | ✓ | ✓ | ✅ |
| R-06 | BM25/FTS on local/global/naive | P2 | | ✓ | ✓ | | ✓ | ✓ | | | | ✓ | ✅ |
| R-07 | `graph_depth`, `max_results` wired | P2 | | ✓ | | | ✓ | | | | ✓ | | ✅ |
| R-08 | Chunk text SSOT (`content_ref` + KV) | P1 | ✓ | ✓ | ✓ | | | ✓ | | ✓ | | | ✅ |
| R-09 | Workspace-scoped cache invalidation | P1 | ✓ | ✓ | | | | | ✓ | ✓ | | | ✅ |
| R-10 | Hybrid LightRAG round-robin merge | P2 | | ✓ | ✓ | | ✓ | | | | ✓ | ✓ | ✅ |
| R-11 | Cross-encoder reranker production path | P2 | | ✓ | | | ✓ | | | | | ✓ | ✅ |
| R-12 | Query modes modular split | P3 | | ✓ | ✓ | | ✓ | | | ✓ | ✓ | | ✅ |
| N-01 | `conversation_history` accepted, unused | P1 | | ✓ | | | ✓ | | | | ✓ | ✓ | ✅ 5.1 |
| N-02 | Adaptive chunk + Gleaning: library only | P1 | ✓ | | ✓ | | ✓ | | | | ✓ | ✓ | ✅ 5.2/5.3 |
| N-03 | Task payload duplicates document text | P1 | ✓ | | | | | ✓ | ✓ | ✓ | | | ✅ 6.1 |
| N-04 | Default Mix = 3× retrieval cost | P1 | | ✓ | | | ✓ | ✓ | | ✓ | | ✓ | ⚠ 6.4 adaptive routing ✅ |
| N-05 | No agentic / iterative retrieval | P2 | | ✓ | | | ✓ | | ✓ | | | ✓ | ✗ |
| N-06 | Graph BFS N+1 (`get_node_edges`) | P1 | | ✓ | ✓ | ✓ | | ✓ | | ✓ | | | ✅ 6.2 |
| N-07 | Upload admission DRY violation | P2 | ✓ | | ✓ | | | | | | ✓ | | ✅ 5.4 |
| N-08 | `text_insert.rs` god-module (~950 LOC) | P2 | ✓ | | | | | | | | ✓ | | ✅ 6.6 |
| N-09 | Injection list O(prefix) + per-key GET | P2 | ✓ | | | | | ✓ | ✓ | ✓ | ✓ | | ⚠ 6.5 paginated ✅ |
| N-10 | No RAG eval harness (RAGAS gates) | P2 | | ✓ | | | ✓ | | ✓ | | | ✓ | ⚠ 8.1 skeleton ✅ |
| N-11 | `QueryMode` serde default ≠ runtime default | P3 | | ✓ | ✓ | | ✓ | | | | ✓ | | ✅ 5.5 |
| N-12 | Saga excludes KV at HTTP admission | P1 | ✓ | | | | | ✓ | ✓ | | ✓ | | ⚠ |
| N-13 | `community_global` popular-node scan | P2 | | ✓ | | ✓ | ✓ | ✓ | | ✓ | | ✓ | ✅ 6.3 |

---

## Severity Scale

| Level | Meaning |
|-------|---------|
| **P0** | Data loss, tenant leak, or outage at modest scale |
| **P1** | Silent quality regression, >3× unnecessary cost, or protocol lie |
| **P2** | Maintainability, SOTA gap, suboptimal defaults |
| **P3** | Naming/docs debt |

---

## Lens Grades (post SPEC-024, honest)

| Lens | Grade | One-line reason |
|------|:-----:|-----------------|
| First principles | **A-** | Clear SSOTs; saga + multi-store acknowledged |
| Ingestion | **A-** | Unified queue; library/API feature split remains |
| Query / retrieval | **A** | LightRAG-faithful; expensive defaults |
| LightRAG expert | **A** | Core algorithm parity achieved |
| GraphRAG expert | **D+** | Labels only; no reports |
| SOTA RAG (Jun 2026) | **B-** | Hybrid+RRF+rerank yes; agentic no |
| Postgres / AGE / pgvector | **A-** | Workspace registry, FTS join, batch upsert |
| System engineering | **A+** | Health, pressure, metrics, recovery |
| O(n) complexity | **B** | Known hotspots documented |
| Rust DRY/SOLID | **A-** | Modes split; admission + text_insert debt |
| AI engineering | **B+** | Good retrieval stack; dead API fields |

---

## Related Specs

- [024-egdequake-audit](../024-egdequake-audit/) — baseline audit + SPEC-024 implementation log
- [017-dry-and-solid-audit](../017-dry-and-solid-audit/) — prior modularity pass
- [018-observability](../018-observability/) — OTEL full stack still deferred
