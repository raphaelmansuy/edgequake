# SPEC-011 — Storage Performance Audit & Optimization

> **Status**: In progress  
> **Scope**: `edgequake-storage` PostgreSQL adapters + API hot paths  
> **Trigger**: Production query `SELECT COUNT(*) FROM public.eq_eq_default_kv` taking **13.7s**

## Document Map

| Document | Purpose |
| -------- | ------- |
| [WHY.md](./WHY.md) | First-principles analysis — root cause, O(N) complexity, risk matrix |
| [BRUTAL_ASSESSMENT.md](./BRUTAL_ASSESSMENT.md) | Honest limits of phase 1; what still scales linearly |
| [PERFORMANCE_GUARANTEE.md](./PERFORMANCE_GUARANTEE.md) | G1/G2/G3 tiers, SLO table, CI enforcement |
| [COUNT_QUERY_FIX.md](./COUNT_QUERY_FIX.md) | **The 13s COUNT(*) incident — two-layer fix** |
| [QUERY_CATALOG.md](./QUERY_CATALOG.md) | Every storage SQL query with complexity class and risk rating |
| [CODE_ANALYSIS.md](./CODE_ANALYSIS.md) | Code-path cross-reference (callers → queries) |
| [IMPROVEMENT_PLAN.md](./IMPROVEMENT_PLAN.md) | Phased fix plan, edge cases, mitigations, non-regression gates |
| [IMPLEMENTATION_PROOF.md](./IMPLEMENTATION_PROOF.md) | Test results, before/after measurements, regression checklist |

## Summary

EdgeQuake stores documents, chunks, vectors, and graph data in PostgreSQL. Several code paths treat **full-table scans** as cheap operations:

1. **`/health`** calls `COUNT(*)` on KV, vector, and graph tables — O(N) per probe
2. **`keys()`** loads every key into memory — O(N) rows + O(N) network transfer
3. **`is_empty()`** delegates to `count()` — O(N) instead of O(1) EXISTS
4. **Row-by-row upserts** — O(N) round-trips per batch
5. **Triple connection pools** — KV + vector + graph each open `DATABASE_POOL_SIZE` connections

The fix preserves **exact semantics** for `count()` where callers need accuracy, but routes connectivity checks and emptiness tests through O(1) queries, adds prefix/LIKE key scans, batch upserts, and a shared pool.
