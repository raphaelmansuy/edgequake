# WHY P0 hot-path status must stay honest about remaining DoD gaps

Status date: **2026-09-22**
Branch: `feat/149-data-access-improvements`
Against: [validation/definition-of-done.md](../validation/definition-of-done.md)

## Verdict

| Bar | Score |
|---|---|
| Scaffolding / compile / unit tests | Strong |
| **P0 crash/replay** (E2E04 B1–B3 on PostgreSQL) | **Closed** |
| **P0 single provider writer + wired e2e** | **Closed** |
| **P0 physical role-batch apply** | **Closed** |
| **P0 vector hydration ANY load** | **Closed** |
| **P0 scoped graph batch delete + retain read** | **Closed** |
| **P0 delete-claim hydration, vector deletes, cleanup** | **Closed** |
| **P0 lease renew + ack batch** | **Closed** |
| **P0 fact-revision UNNEST load** | **Closed** (proven with injected `object_kind=fact` events; product committer still emits `document_batch` / `document`) |
| **P0 HTTP E2E02–07 and E2E09–13** | **Closed** on PostgreSQL AppState (E2E08 / full E2E14–15 cutover stay open) |
| **P0 ANN recall@10 + p95 artifact** | **Closed** — [p0-ann-recall.json](p0-ann-recall.json) |
| **P0 production-correct hot path** | **Closed** for the bars above |
| Full R1 certification (all E2E01–15 + six-profile DoD) | **Open** (E2E08, SQLite E2E14/15, R2–R4) |
| Full six-profile DoD (P1–P4 + 7-day soak) | **Not met** |

Product serving remains **P0 only**. P1–P4 stay unavailable via `assert_product_serving_allowed`.

## P0 leftover apply loops (closed)

- **Vector hydration:** one `LEFT JOIN` with `event_id = ANY($1)` per `apply_batch` entry.
- **Scoped deletes:** `delete_nodes_scoped_batch` / `delete_edges_scoped_batch` once per scope.
- **Delete claim (n=5):** one graph fact hydration, one scoped delete, one cleanup `UNNEST`, three vector statements.
- **Lease renew + ack:** one fenced `UNNEST` renew and one fenced `UNNEST` ack+visibility per role claim.
- **Fact revisions:** `load_fact_revisions_many` one `UNNEST` join to `object_revisions` for all `object_kind=fact` events in a claim; `fact_revision_claim_loads_once_via_unnest` asserts delta = 1 for n=5.

## P0 HTTP certification (closed for the P0-provable subset)

`provider_access_e2e` with `postgres,provider-access-fault`:

- E2E01 health, P0 wiring, E2E02 isolation (relational document fence), E2E03 filters, E2E04/E2E05 SIGKILL at `b3`, E2E06 shared delete, E2E07 poison/pending, E2E09 bounded graph, E2E10 two runtimes + forged key, E2E11 model retention, E2E12 config/ready, E2E13 scratch migrate + rebuild digest.
- E2E08 (binding cutover), full E2E14 operational persistence, and E2E15 SQLite cutover remain open (fail-closed stubs only for P3 compile).

## P0 ANN artifact (closed)

[p0-ann-recall.json](p0-ann-recall.json): corpus 1000, dim 64, recall@10 ≥ 0.95, retained p50/p95. Not a 7-day soak and not a P1 comparison.

## Proof recorded

```bash
export DATABASE_URL=… EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1
cargo test -p edgequake-storage --features postgres \
  --test e2e_spec149_projection_replay -- --test-threads=1
# 11 passed, 0 failed, 0 ignored

cargo test -p edgequake-storage --features postgres,provider-access-fault \
  --test e2e_spec149_process_kill -- --test-threads=1
# 2 passed, 0 failed, 0 ignored

EDGEQUAKE_PROVIDER_ACCESS_E2E=1 cargo test -p edgequake-api \
  --features postgres,provider-access-fault --test provider_access_e2e -- --test-threads=1
# 15 passed, 0 failed, 0 ignored

cargo test -p edgequake-storage --features postgres \
  --test e2e_spec149_p0_ann_recall -- --nocapture
# 1 passed; writes evidence/p0-ann-recall.json
```

## Remaining work (open)

- E2E08 backfill/switch/rollback; full E2E14/E2E15 and R2–R4.
- P1–P4 adapter selection and live certification (`assert_product_serving_allowed` stays P0-only).
- 7-day soak.
- Product producer for `object_kind=fact` projection events (arm is batched; committer still uses `document_batch`).

## Migration risk (150–155)

- **Low** for expand-contract apply (155 is additive manifest membership).
- **High** if treated as alternate-provider cutover readiness (not certified).
