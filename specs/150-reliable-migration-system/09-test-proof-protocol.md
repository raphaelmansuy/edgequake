# 09 — Test and proof protocol

Parent: [README](README.md) · Matrix: [07](07-upgrade-path-matrix.md) · WPs: [08](08-implementation-plan.md)

## Principle (LAW-150-7)

A proof is valid only if the database **shape** that historically broke us is present. SPEC-93 GREEN on 600 synthetic docs did not predict M118 `21000`.

## Fixture families

| ID | How produced | Covers |
|----|--------------|--------|
| F-empty | `sqlx migrate run` on blank PG + AGE + vector | Fresh install |
| F-epoch-{tag} | Run **that tag's binary** `migrate` (build from git if no GHCR) then seed | Real ledger checksums |
| F-fossil-001 | F-epoch v0.11.0 without later migrate | #195 |
| F-fossil-019 | F-epoch v0.10.6 | 019 drift |
| F-age-node | At least one `ag_catalog.create_graph` + `"Node"` rows | #273 `->>>` |
| F-dim-2001 | embedding column / row with dim > 2000 | #275 |
| F-multi-ws | same document id in two `workspace_id` | SPEC-110 M118 |
| F-mid-cutover | applied through 124, leftover KV rows | SPEC-137/139 |
| F-dirty | `_sqlx_migrations.success = false` | Dirty recovery |
| F-newer | ledger max > binary max | downgrade refuse |

GHCR: `ghcr.io/raphaelmansuy/edgequake:X.Y.Z` exists from **v0.23.0**. Older: `git checkout vX.Y.Z && cargo build -p edgequake --release` in CI cache.

## Always-on PR tests (fast)

| Test | Assert |
|------|--------|
| checksum lock | current files = lock (existing `check_migration_checksums.sh`) |
| manifest parse | WP-1 |
| authoring lint | WP-6 |
| `contract_spec091_boot_gate` | fail-mode 78 preserved |
| wait-mode | WP-5 AC |
| fossil 001/019 unit | UPDATE checksum when hash known |
| wiring | no duplicate version arrays |

## Nightly / release (slow)

For PG **16, 17, 18** (SPEC-93 already) **with AGE**:

1. Restore F-epoch for key tags: v0.11.0, v0.10.6, v0.13.2, v0.22.0, v0.23.0, v0.25.0, v0.26.10.
2. Seed F-age-node + F-multi-ws + (on 0.13) F-dim-2001.
3. `edgequake migrate` HEAD **without** `EDGEQUAKE_DEV_MODE`.
4. Assert: `max(version)=HEAD max`; canary `SELECT` on `documents`, `chunks`, one AGE graph; `guard` not silently GREEN if uncovered > 0.
5. `edgequake` serve wait-mode: `/live` 200; after migrate `/ready` 200.
6. Artifact: `migration_run_step` CSV (WP-3) for [10](10-performance-budget.md).

## Chaos

| Case | Expect |
|------|--------|
| `kill -9` mid Phase E file | Dirty or not applied; rerun applies or prints Dirty recovery |
| `kill -9` mid Phase D batch | Cursor not past failed batch; rerun continues |
| two migrate | second 75 or waits until deadline then 75 |
| `lock_timeout` via blocker txn | retry then fail with 55P03, not hang |
| Helm `--wait` dry-run / kubeconform | Job is pre-upgrade |

## Seeds that must exist (minimum SQL)

```sql
-- F-multi-ws: same logical doc, two workspaces (M118)
-- (adapt to the KV or typed shape of the epoch fixture)

-- F-age-node
SELECT * FROM ag_catalog.create_graph('eq_ws_seed');
-- insert at least one Node with properties JSON (not jsonb) if testing 078 path
```

## CI jobs to change

| Job | Change |
|-----|--------|
| `postgres-integration.yml` apply loop | `psql -v ON_ERROR_STOP=1`; **delete** `\|\| true` |
| `migration-guard.yml` | AGE image or extra `CREATE EXTENSION age` |
| new `migration-epoch.yml` | nightly matrix; not a PR gate until runtime < 20 min subset |

## Honest non-coverage (until measured)

- 100k+ vector HNSW rebuild wall clock (SPEC-93 non-goal).
- Partner production dump (SPEC-110 honesty). WP-8 uses synthetic **shape**, not their data.
- Concurrent ingest **during** Phase C DROP (must be operator: drain ingest).
