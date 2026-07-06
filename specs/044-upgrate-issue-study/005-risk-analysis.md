# SPEC-044 — Risk Analysis

**Method:** Impact × likelihood with mitigations tied to [008-implementation-plan.md](./008-implementation-plan.md) phases.

---

## Risk matrix

| ID | Risk | Impact | Likelihood | Mitigation | Phase |
| -- | ---- | ------ | ---------- | ---------- | ----- |
| R-044-01 | Compensation `delete_node` always fails on merge error | **High** — orphan graph + failed docs | **High** (any merge error on v0.14.0+) | P0: fix `cypher_*_bound` | P0 |
| R-044-02 | `has_node`/`get_node` broken for tooling | **Medium** — community index, admin | Medium | Same P0 fix | P0 |
| R-044-03 | Orphan nodes accumulate in prod (pre-fix) | **Medium** — graph pollution, query noise | Medium | Reconciliation SQL + P0 deploy | P0 + Ops |
| R-044-04 | Over-broad compensation deletes good nodes | **Medium** — data loss on partial error | Medium | P2: phase-scoped artifacts | P2 |
| R-044-05 | Transient merge fail during M043 upgrade | **Medium** — first-boot ingest fail | Low–Medium | Retry + maintenance window | Ops |
| R-044-06 | False root-cause: schema migration | **Low** — wasted ops time | Low | `edgequakeSchema.sql` + health SQL | Doc |
| R-044-07 | False root-cause: SPEC-039 labels | **Low** — wrong fix path | Low | Error text discrimination | Doc |
| R-044-08 | sqlx bind regression on future bump | **High** — recurrence | Low | spec022 + BT-044 gates in CI | P3 |
| R-044-09 | PG16/PG18 bind behaviour drift | **Medium** — tier-specific outage | Low | Dual-matrix CI (SPEC-042) | P3 |
| R-044-10 | `raw_sql` reintroduced for bound path | **High** | Low | Grep gate + code review | P3 |
| R-044-11 | Inline literal "works in mock tests" | **High** — false green | Medium | Require `DATABASE_URL` postgres test in CI | P3 |
| R-044-12 | Operator ignores quarantine logs | **Low** — silent orphans | Medium | Alert on `quarantine:` rate in Loki | Ops |
| R-044-13 | Document delete leaves graph nodes | **High** — stale entities | **High** | Same P0a fix; D-3 E2E | P0d |
| R-044-14 | Entity API 404 (`get_node` broken) | **High** — broken UX | **High** | P0a; D-5 E2E | P0d |
| R-044-15 | CI `continue-on-error` masks C-1 | **High** — repeat incident | **High** | P0c remove mask | P0c |
| R-044-16 | spec022 static test locks `::agtype` | **Medium** — blocks fix | Medium | P0b invert assertion | P0b |
| R-044-17 | `raw_sql` prevents `$1` bind | **High** — fix appears done but isn't | High | P0a use `sqlx::query` | P0a |
| R-044-18 | Entity reconcile incomplete | **Medium** — admin tool fail | Medium | D-4 postgres test | P1 |
| R-044-19 | Cypher fix validated on PG18 only | **High** — pg16 prod outage | **High** | `spec044-battle-test-all` all tiers | P0c |

---

## Incident severity assessment

| Dimension | Rating | Notes |
| --------- | ------ | ----- |
| **Availability** | Medium | Ingest fails when merge errors; retry often succeeds |
| **Integrity** | Medium | Orphan nodes/vectors when compensation fails |
| **Security** | Low | Not an auth bypass; injection mitigated in fix |
| **Blast radius** | All PG deployments on v0.14.0+ | Compensation path universal |

---

## Upgrade path matrix

| From | To | Ingest risk | Required action |
| ---- | -- | ----------- | ----------------- |
| v0.13.x + PG16 volume | v0.14.1 app | **Latent** Cypher bug; merge usually OK | Deploy P0 patch; run health SQL |
| v0.14.0 / v0.14.1 | v0.14.2+ (P0 fix) | **Resolved** compensation | Standard rolling deploy |
| Fresh v0.14.x install | — | Same latent bug | P0 before prod traffic |
| App downgrade to v0.13.x | — | Schema forward-only OK | Cypher path differed pre-P-H7 bound |

---

## Rollback strategy

1. **Application:** Revert to v0.13.3 — merge/compensation paths differ; may avoid bound delete if not on hot path. **Not recommended** — forward fix preferred.
2. **Emergency mitigation:** Set `EDGEQUAKE_NATIVE_GRAPH_WRITES=1` — does not fix compensation delete.
3. **Data cleanup:** Orphan nodes from failed compensation — manual Cypher or `entity_reconcile` after P0.

---

## Monitoring signals

| Signal | Source | Alert if |
| ------ | ------ | -------- |
| `quarantine: failed to roll back orphan node` | Graylog / Loki | > 0 in 15 min post-deploy |
| `knowledge-graph merge error(s) during persist` | Worker logs | Sustained rate > baseline |
| `merge_entities_batch_global` WARN | Merger logs | Spike after upgrade |
| `Parameterized Cypher execute failed` | Storage errors | Any occurrence post P0 |
| `extversion` drift | `/health` operational | age or vector < pinned |
| Document status `Failed` | `documents` table | Spike post-deploy |

---

## Residual risk (post P0)

| Residual | Acceptance |
| -------- | ---------- |
| Primary merge transient failures | Retry + ops runbook; not eliminated by P0 |
| Over-broad compensation (P2 not shipped) | Document; fix in follow-up |
| Orphan residue pre-fix | One-time reconciliation script |
