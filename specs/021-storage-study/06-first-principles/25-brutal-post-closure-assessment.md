# 25 — Brutal Post-Closure Assessment (2026-06-26, rev.5)

> **Method:** Code is Law after rev.4 + rev.5 (shared extraction fixture, orchestrator
> default result cache, env-gated test override, DIP trait on orchestrator). `make test-spec021` green.

## Executive summary

Plan-19 is **closed for production new-write correctness**. Remaining gaps are
**operational** (P-G1b) or **explicitly deferred scope** (full 8-step persister, GraphRAG).

| Dimension | Grade | Brutal truth |
|-----------|-------|--------------|
| Correctness (new writes) | **A** | RC-6..11 fixed; saga on merge failure |
| Correctness (legacy) | **C** | Pre-G1 graphs need admin P-G1b |
| Performance | **A−** | Batch merge; LLM summarization still O(E) when enabled |
| SOLID / DRY | **A** | Shared `SPEC021_SARAH_CHEN_EXTRACTION_JSON`; DIP ports everywhere; orchestrator default engine mirrors API caches |
| E2E honesty | **A** | Worker + orchestrator (default engine) cache bust; unconditional graph assert |
| GraphRAG maturity | **C+** | Flat LightRAG; ops ≠ intelligence |

## What we can claim (verified)

- **DRY:** `edgequake_pipeline::SPEC021_SARAH_CHEN_EXTRACTION_JSON` — single fixture for sc2, worker E2E, orchestrator tests
- **SOLID:** Orchestrator invalidates via `QueryResultCacheInvalidator`; default `initialize()` wires embedding + result cache (aligned with `build_production_query_engine`)
- **E2E:** `spec021_orchestrator_default_engine_invalidates_cache_on_insert` — no `with_query_engine` pre-wire required
- **Safety:** Test provider override requires `EDGEQUAKE_ALLOW_TEST_PROVIDER_OVERRIDE=1` (contract tested)

## What we cannot claim (be honest)

1. **Mix HTTP weight ordering** — engine `contract_query_modes` only; HTTP checks mode + stats.
2. **Postgres UNWIND through worker upload** — no worker+Postgres spec021 E2E.
3. **P-G1b** — admin tool only; no auto-heal.
4. **GraphRAG** — no communities.
5. **Full 8-step persister** — KV/relational/lineage remain in processor by design.
6. **Orchestrator vs API parity** — core default engine has caches but no BM25 reranker (API bootstrap adds reranker).

## Four-lens verdict (final)

| Lens | Grade | One line |
|------|-------|----------|
| GraphRAG | C+ | Hygiene fixed; zero hierarchical intelligence |
| LightRAG | A− | Batch merge + persister + deterministic worker E2E |
| AI Engineer | A | Engine + HTTP + worker + orchestrator (default) cache contracts |
| System Engineer | A− | RC-7 closed; persister scope partial by design |

## Recommendation

**Ship.** P-G1b = operator runbook. Do **not** reopen plan-19 unless product asks for full 8-step persister or GraphRAG communities.

See `24-plan19-gap-closure.md`; plan-19 §13.10 (rev.5).
