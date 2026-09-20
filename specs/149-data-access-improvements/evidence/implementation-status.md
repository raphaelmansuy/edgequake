# WHY P0 hot-path status must stay honest about remaining DoD gaps

Status date: **2026-09-20**
Branch: `feat/149-data-access-improvements`
Against: [validation/definition-of-done.md](../validation/definition-of-done.md)

## Verdict

| Bar | Score |
|---|---|
| Scaffolding / compile / unit tests | Strong |
| **P0 production-correct hot path** (bindings, authority, fenced projection, exact-binding delete, managed worker) | **Closed** |
| Full R1 certification (PROVIDER-ACCESS-E2E01–07 / B1–B3 process-kill matrix) | **Open** |
| Full six-profile DoD (P0–P4 + E2E01–15 + soak) | **Not met** |

Product serving remains **P0 only**. P1–P4 stay unavailable via `assert_product_serving_allowed`.

## P0 production-correct hot path (closed)

- Immutable scoped `DataBindingDescriptor` / registry ports; migration **154** adds cross-scope delivery/visibility/cleanup triggers.
- `PgIngestionCommitter` auto-provisions P0 graph+vector bindings in the authority transaction, requires exactly those deliveries, and atomically advances document revision with chunks/facts/contributions/embeddings/events.
- Pipeline uses explicit `IngestionAuthority` (`DurableCommitter` on P0); no silent optional-committer fallback.
- Projection is revision-scoped, digest-verified, binding-specific, lease-renewed before apply, and visibility-published only on fenced ack. Scoped graph node IDs isolate same names across tenant/workspace.
- Deletion requires lifecycle tombstone; cleanup intents and vector targets come from recorded bindings only (no default-store fallback).
- `ProjectionWorkerRuntime` is owned by `AppState` (cancellable); production appliers are AGE + colocated pgvector (no success no-ops).

## Proof (non-skipped, real PostgreSQL)

Commands (require reachable `DATABASE_URL` / scratch `{db}_test` with migrations through 154):

```bash
export DATABASE_URL=… EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1
cargo test -p edgequake-storage --features postgres \
  --test e2e_spec149_ingestion_committer \
  --test e2e_spec149_projection_replay \
  -- --nocapture --test-threads=1
```

Recorded locally (2026-09-20): **5 + 4 passed**, 0 failed, 0 ignored — concurrent same-key (20), digest conflict, rollback, lease takeover/stale ack, cross-tenant graph isolation, exact-binding tombstone intents, end-to-end replay apply.

Cert runner (`scripts/provider-access/test-profile.sh` recovery suite) labels this **`SPEC149-P0-HOTPATH-REPLAY`**, not full **`PROVIDER-ACCESS-E2E04`**. That E2E ID stays reserved until B1/B2/B3 process-kill requirements pass.

## Remaining work (open)

- Full HTTP E2E01–15 matrix and PROVIDER-ACCESS-E2E04 crash barriers.
- P1–P4 adapter selection and live certification.
- Provider cutover / restore / seven-day soak.
- Completing operational SQLite ports in `required-ports.json`.

## Migration risk (150–154)

- **Low** for expand-contract apply (additive; 154 is trigger-only integrity).
- **High** if treated as alternate-provider cutover readiness (not certified).
