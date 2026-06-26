# 25 — Brutal Post-Closure Assessment (2026-06-26, rev.3)

> **Method:** Code is Law after commit `64fcd808` + rev.3 pass (batch-only merger,
> worker cache E2E, dead-code cleanup). Honest verdict **after** full spec021 green.

## Executive summary

Plan-19 is **closed for production new-write correctness**. Remaining work is
**operational** (P-G1b legacy repair) and **scope** (full 8-step persister), not
silent-corruption bugs.

| Dimension | Grade | Brutal truth |
|-----------|-------|--------------|
| Correctness (new writes) | **A** | RC-6..11 structurally fixed; saga on merge failure |
| Correctness (legacy) | **C** | Pre-G1 graphs still need admin P-G1b |
| Performance | **A−** | Vector + graph batch only in merger; LLM summarization still O(E) when enabled |
| SOLID / DRY | **A−** | Batch-only merge path; two DIP ports; `from_settings` SSOT; not full 8-step persister |
| E2E honesty | **A−** | Worker upload + **worker cache bust** + HTTP Bypass/Mix/stats + cross-doc merge contracts; mock graph assert still conditional |
| GraphRAG maturity | **C+** | Flat LightRAG; ops ≠ intelligence |

## What we can claim (verified)

- **DRY:** Single batch merge path (removed unused single-entity/relationship shims); one persister impl; `from_settings` everywhere
- **SOLID:** `IngestionPersister` + `QueryResultCacheInvalidator` ports; processor never holds concrete `QueryEngine`
- **E2E:** `make test-spec021` includes worker persist cache invalidation (`e2e_spec021_worker_cache_invalidation`), engine cache contract, HTTP modes, cross-doc merge, processor DIP source contract
- **P-G9 complete:** Invalidation proven on engine **and** worker persist success paths

## What we cannot claim (be honest)

1. **Mix HTTP weight ordering** — still not tested over HTTP; engine `contract_query_modes` covers weight sensitivity.
2. **Worker graph assert** — still conditional on terminal success; mock often `partial_failure`.
3. **Postgres UNWIND through worker upload** — adapter contracts elsewhere; no worker+Postgres persist E2E in spec021.
4. **Core `insert()` cache invalidation** — API worker only; accepted architectural boundary.
5. **P-G1b** — admin tool only; no auto-heal.
6. **GraphRAG** — no communities; this sprint is storage/query hygiene, not retrieval intelligence.
7. **Full 8-step persister** — KV/relational/lineage steps remain in processor by design.

## Four-lens verdict (final)

| Lens | Grade | One line |
|------|-------|----------|
| GraphRAG | C+ | Hygiene fixed; zero hierarchical graph intelligence |
| LightRAG | B+ | Canonical batch merge + persister; PDF scan scaling gap |
| AI Engineer | A− | Engine + HTTP + **worker** cache contracts; Mix HTTP still not weight-tested |
| System Engineer | A− | RC-7 closed; persister scope intentionally partial |

## Recommendation

**Ship.** P-G1b = operator runbook only. Do **not** reopen plan-19 unless product asks for full 8-step persister or GraphRAG communities.

See `24-plan19-gap-closure.md`; plan-19 §13.10.
