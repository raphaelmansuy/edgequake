# 24 — Plan-19 Gap Closure (2026-06-26, rev.5)

> **Supersedes:** plan-19 §6 and §13 closure record

## Closed (rev.4–5)

| Gap | Fix | Evidence |
|-----|-----|----------|
| **P-G9 orchestrator default** | `initialize()` default engine gets embedding + result cache | `orchestrator/mod.rs`, `spec021_orchestrator_default_engine_invalidates_cache_on_insert` |
| **P-G9 DIP orchestrator** | `QueryResultCacheInvalidator` on persist | `ingestion.rs` |
| **DRY fixture** | `SPEC021_SARAH_CHEN_EXTRACTION_JSON` in pipeline | `test_fixtures.rs`; sc2 + worker + orchestrator tests |
| **Test override safety** | Env gate `EDGEQUAKE_ALLOW_TEST_PROVIDER_OVERRIDE=1` | `safety_limits.rs`, `spec021_test_provider_override_contract.rs` |
| *(rev.4 items)* | Worker graph E2E, batch merger, persister trait, HTTP modes, worker cache | see git `4e01f78d`, `64fcd808` |

## Honest remaining (accepted)

| Item | Status |
|------|--------|
| P-G1b legacy backfill | Admin-only |
| Full 8-step persister | Deferred |
| Postgres worker E2E | Deferred |
| GraphRAG communities | Out of scope |
| Mix HTTP weight ordering | Engine-only |
| Core vs API query engine | No reranker in core default path |

## Verification

```bash
make test-spec021
```
