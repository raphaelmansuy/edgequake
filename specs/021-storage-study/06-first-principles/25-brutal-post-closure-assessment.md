# 25 — Brutal Post-Closure Assessment (2026-06-26, rev.2)

> **Method:** Code is Law after plan-24 + continuation pass (SOLID cache port, cross-doc
> merge contract, spec021 green). This is the honest verdict **after** all shipped work.

## Executive summary

Plan-19 is **closed for production new-write correctness**. What remains is
**operational** (P-G1b legacy repair) and **scope** (full 8-step persister), not
silent-corruption bugs.

| Dimension | Grade | Brutal truth |
|-----------|-------|--------------|
| Correctness (new writes) | **A** | RC-6..11 structurally fixed; saga on merge failure |
| Correctness (legacy) | **C** | Pre-G1 graphs still need admin P-G1b |
| Performance | **A−** | Vector + graph batch in merger; LLM summarization still O(E) when enabled |
| SOLID / DRY | **A−** | `IngestionPersister` + `QueryResultCacheInvalidator` ports; `from_settings` SSOT; not full 8-step port |
| E2E honesty | **B+** | Worker upload, HTTP Bypass/Mix, cache invalidation, cross-doc merge contract; mock often `partial_failure` |
| GraphRAG maturity | **C+** | Flat LightRAG; ops ≠ intelligence |

## What we can claim (verified)

- **DRY:** One persister impl; batch merge paths; `DefaultIngestionPersister::from_settings` used in orchestrator, processor, and contract tests
- **SOLID:** Callers depend on `IngestionPersister` (DIP); processor depends on `QueryResultCacheInvalidator`, not concrete `QueryEngine`; merger SRP preserved
- **E2E:** `make test-spec021` — ingest resilience, worker persist, HTTP modes, cache, graph batch, cross-doc entity merge, processor DIP contract
- **P-G9 complete:** Embedding cache + result cache + invalidation via trait port on worker persist success

## What we cannot claim (be honest)

1. **Mix HTTP test is thin** — checks `mode: "mix"` returns 200, not weight-sensitive ordering (engine contract covers that).
2. **Worker E2E graph assert is conditional** — mock LLM often yields `partial_failure`; chunks always asserted, graph only on terminal success (`node_count > 0`).
3. **Orchestrator path** — covered by `sc2_sc5_ingestion` (core) + `contract_cross_document_entity_merge`, not duplicate worker E2E.
4. **Postgres UNWIND through worker upload** — storage adapter has postgres batch contracts; no dedicated worker+Postgres persist E2E in spec021.
5. **Result cache invalidation** — wired on **API worker** persist success via `QueryResultCacheInvalidator`; `edgequake-core::insert` library path does not invalidate (no query engine in core — accepted).
6. **P-G1b** — tool exists; operators must run it; we do not auto-heal legacy tenants.
7. **GraphRAG** — still no communities; do not sell this work as retrieval quality improvement.
8. **Dead merger singles** — `merge_entity` / `merge_relationship` remain for tests; batch path is production; clippy dead_code warnings are noise until removed.

## Four-lens verdict (final)

| Lens | Grade | One line |
|------|-------|----------|
| GraphRAG | C+ | Hygiene fixed; zero hierarchical graph intelligence |
| LightRAG | B+ | Canonical merge + batch graph; PDF task-scan scaling gap |
| AI Engineer | B+ | Engine + HTTP + cache contracts; mock worker E2E still weak |
| System Engineer | A− | RC-7 closed; persister scope intentionally partial |

## Recommendation

**Ship.** Track P-G1b as operator runbook only. Do **not** reopen plan-19 schedule unless product asks for full 8-step persister or GraphRAG communities.

See `24-plan19-gap-closure.md` for change list; plan-19 §13.10 for traceability.
