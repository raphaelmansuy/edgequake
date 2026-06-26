# 25 — Brutal Post-Closure Assessment (2026-06-26, rev.4)

> **Method:** Code is Law after rev.3 + rev.4 (deterministic worker mock, orchestrator
> cache invalidation, `build_test_state` DRY). Full `make test-spec021` green.

## Executive summary

Plan-19 is **closed for production new-write correctness**. Remaining gaps are
**operational** (P-G1b) or **explicitly deferred scope** (full 8-step persister, GraphRAG).

| Dimension | Grade | Brutal truth |
|-----------|-------|--------------|
| Correctness (new writes) | **A** | RC-6..11 fixed; saga on merge failure |
| Correctness (legacy) | **C** | Pre-G1 graphs need admin P-G1b |
| Performance | **A−** | Batch merge; LLM summarization still O(E) when enabled |
| SOLID / DRY | **A** | Batch-only merger; two DIP ports; `from_settings` + `build_test_state`; orchestrator `with_query_engine` |
| E2E honesty | **A** | Worker upload asserts `completed` + `SARAH_CHEN` node; worker + orchestrator cache bust; cross-doc contract |
| GraphRAG maturity | **C+** | Flat LightRAG; ops ≠ intelligence |

## What we can claim (verified)

- **DRY:** Single batch merge path; shared `build_test_state`; ingestion pipeline built via `build_ingestion_pipeline` in tests (not `default_pipeline()`)
- **SOLID:** `IngestionPersister` + `QueryResultCacheInvalidator`; orchestrator `with_query_engine` + `invalidate_result_cache()` after persist
- **E2E:** Worker graph assert is **unconditional** (seeded mock via test provider override); orchestrator cache test in `spec021_orchestrator_cache_invalidation`
- **P-G9:** Cache invalidation on API worker **and** core orchestrator when query engine wired

## What we cannot claim (be honest)

1. **Mix HTTP weight ordering** — engine `contract_query_modes` only; HTTP checks mode + stats.
2. **Postgres UNWIND through worker upload** — adapter contracts elsewhere; no worker+Postgres spec021 E2E.
3. **Orchestrator cache without `with_query_engine`** — default `initialize()` engine has no result cache; library callers must pre-wire.
4. **P-G1b** — admin tool only; no auto-heal.
5. **GraphRAG** — no communities; not a retrieval-quality sprint.
6. **Full 8-step persister** — KV/relational/lineage remain in processor by design.
7. **Test provider override** — production hook in `safety_limits` is test-only usage today; must not be set in prod (Mutex guard, cleared on `WorkerAppGuard` drop).

## Four-lens verdict (final)

| Lens | Grade | One line |
|------|-------|----------|
| GraphRAG | C+ | Hygiene fixed; zero hierarchical intelligence |
| LightRAG | A− | Batch merge + persister + deterministic worker E2E |
| AI Engineer | A− | Engine + HTTP + worker + orchestrator cache contracts |
| System Engineer | A− | RC-7 closed; persister scope partial by design |

## Recommendation

**Ship.** P-G1b = operator runbook. Do **not** reopen plan-19 unless product asks for full 8-step persister or GraphRAG communities.

See `24-plan19-gap-closure.md`; plan-19 §13.10 (rev.4).
