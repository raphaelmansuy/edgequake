# SPEC-026 — EdgeQuake vs LightRAG Comparative Audit

**Date:** 2026-06-27  
**Method:** Code is law — every claim cites source in `/Users/raphaelmansuy/Github/03-working/LightRAG` (reference) or `edgequake/` (implementation).  
**Prior audits:** [SPEC-025](../025-egdequake-audit/) (internal EdgeQuake audit), [SPEC-024](../024-egdequake-audit/) (baseline)

---

## Executive Verdict (brutal, one paragraph)

EdgeQuake is a **credible Rust reimplementation of LightRAG's core graph-RAG algorithm**, not a clone. It achieves **algorithm parity** on dual-level keywords, three vector stores, provenance chunk retrieval, and hybrid/mix modes — then **extends** with BM25 fusion, RRF default, cross-encoder rerank, multi-hop graph, workspace tenancy, and operator-grade Postgres durability. LightRAG wins on **format breadth** (DOCX, MinerU, Docling, multimodal, 4 chunking strategies, 13 storage backends) and **ecosystem maturity** (224 test modules, battle-tested `operate.py`). EdgeQuake wins on **production substrate** (mandatory Postgres+AGE+pgvector, durable task queue, saga compensation, health/pressure metrics) and **retrieval stack depth** (FTS on all arms, intent routing, conversation history). Neither is GraphRAG. Neither is June-2026 SOTA agentic RAG. EdgeQuake's honest grade: **A as LightRAG infrastructure**, **B+ as reference implementation**, **C+ vs LightRAG feature surface area**.

---

## Reference Paths

| System | Path | Role |
|--------|------|------|
| LightRAG (reference) | `/Users/raphaelmansuy/Github/03-working/LightRAG` | Python canonical algorithm |
| EdgeQuake | `edgequake/crates/*` | Rust production port + extensions |

---

## Document Index

| ID | Topic | File |
|----|-------|------|
| 001 | First principles | [001-first-principles/001-first-principles.md](./001-first-principles/001-first-principles.md) |
| 002 | Algorithms | [002-algorithms/001-algorithm-comparison.md](./002-algorithms/001-algorithm-comparison.md) |
| 003 | Ingestion pipeline | [003-ingestion/001-ingestion-comparison.md](./003-ingestion/001-ingestion-comparison.md) |
| 004 | Query pipeline | [004-query/001-query-comparison.md](./004-query/001-query-comparison.md) |
| 005 | Features | [005-features/001-feature-matrix.md](./005-features/001-feature-matrix.md) |
| 006 | Robustness | [006-robustness/001-robustness-comparison.md](./006-robustness/001-robustness-comparison.md) |
| 007 | Evaluations | [007-evaluations/001-evaluation-comparison.md](./007-evaluations/001-evaluation-comparison.md) |
| 008 | Expert lenses | [008-expert-lenses/](./008-expert-lenses/) (8 lens documents) |
| 009 | Improvement plan | [009-improvement-plan/001-improvement-plan.md](./009-improvement-plan/001-improvement-plan.md) |
| 009b | Phase 2 ingestion parity | [009-improvement-plan/phase2/](./009-improvement-plan/phase2/) |

---

## Cross-Reference Matrix

Finding IDs stable across SPEC-026 documents.

| ID | Finding | Sev | Alg | Ing | Qry | Feat | Rob | Eval | Status |
|----|---------|:---:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| C-01 | Core LightRAG algorithm parity achieved | — | ✓ | ✓ | ✓ | | | | ✅ |
| C-02 | EdgeQuake extends retrieval (BM25, RRF, rerank) | — | ✓ | | ✓ | ✓ | | | ✅ by design |
| C-03 | LightRAG parser/chunking breadth >> EdgeQuake | P1 | | ✓ | | ✓ | | | ✗ gap |
| C-04 | LightRAG storage portability >> EdgeQuake | P2 | | | | ✓ | | | ✗ by design |
| C-05 | EdgeQuake Postgres durability >> LightRAG default | — | | ✓ | | | ✓ | | ✅ |
| C-06 | Neither has GraphRAG community reports | P2 | ✓ | | ✓ | ✓ | | | ✗ both |
| C-07 | Neither has agentic retrieval loops | P2 | | | ✓ | | | ✓ | ✗ both |
| C-08 | LightRAG test surface >> EdgeQuake | P2 | | | | | ✓ | ✓ | ⚠ |
| C-09 | EdgeQuake eval skeleton; LightRAG none | P3 | | | | | | ✓ | △ EQ ahead |
| C-10 | Default mode both Mix; merge semantics differ | P2 | ✓ | | ✓ | | | | △ |
| C-11 | Community index: EQ ingest-only; LR none | P2 | | ✓ | ✓ | ✓ | | | △ deviation |
| C-12 | Multimodal ingestion: LR yes, EQ PDF-only | P1 | | ✓ | | ✓ | | | ✗ gap |

---

## Lens Grades (EdgeQuake vs LightRAG reference)

| Lens | EdgeQuake | LightRAG | Winner | Doc |
|------|:---------:|:--------:|:------:|-----|
| LightRAG algorithm fidelity | **A** | ref | Tie | [002-lightrag-expert](./008-expert-lenses/002-lightrag-expert.md) |
| Feature breadth | **B-** | **A** | LightRAG | [005-features](./005-features/001-feature-matrix.md) |
| Production robustness | **A+** | **B** | EdgeQuake | [006-robustness](./006-robustness/001-robustness-comparison.md) |
| Parser / format ingestion | **C+** | **A** | LightRAG | [003-ingestion](./003-ingestion/001-ingestion-comparison.md) |
| Query retrieval quality stack | **A** | **B+** | EdgeQuake | [004-query](./004-query/001-query-comparison.md) |
| GraphRAG category | **D+** | **D** | Neither | [003-graphrag-expert](./008-expert-lenses/003-graphrag-expert.md) |
| SOTA RAG (Jun 2026) | **B-** | **C+** | EdgeQuake | [004-sota-rag](./008-expert-lenses/004-sota-rag-jun2026.md) |
| Postgres / AGE / pgvector | **A** | **B-** | EdgeQuake | [008-postgres](./008-expert-lenses/008-postgres-age-pgvector.md) |
| System engineering | **A+** | **B+** | EdgeQuake | [005-system-engineering](./008-expert-lenses/005-system-engineering.md) |
| O(n) / complexity | **B** | **C+** | EdgeQuake | [006-complexity](./008-expert-lenses/006-complexity-on.md) |
| Rust DRY/SOLID | **A-** | N/A | — | [007-rust-solid](./008-expert-lenses/007-rust-solid-dry.md) |
| AI engineering | **B+** | **B** | EdgeQuake | [001-ai-engineering](./008-expert-lenses/001-ai-engineering.md) |

---

## Severity Scale

| Level | Meaning |
|-------|---------|
| **P0** | Data loss, tenant leak, outage at modest scale |
| **P1** | Silent quality regression, >3× cost, missing parity on primary path |
| **P2** | Maintainability, category mislabel, suboptimal defaults |
| **P3** | Docs/naming debt |

---

## Related Specs

- [025-egdequake-audit](../025-egdequake-audit/) — internal pipeline audit (post SPEC-024)
- [017-dry-and-solid-audit](../017-dry-and-solid-audit/) — modularity pass
- [010-ingestion-reliability](../010-ingestion-reliability/) — reliability mission
