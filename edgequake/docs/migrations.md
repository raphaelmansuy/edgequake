# Database Migrations

EdgeQuake uses [SQLx](https://github.com/launchbadge/sqlx) embedded migrations.
**Schema is applied only by `edgequake migrate`** (SPEC-091 LD-15 / SPEC-150).
The API process never auto-applies numbered migrations on start.

**Operator guide (plain English):**
[`docs/operations/upgrading.md`](../../docs/operations/upgrading.md)

## Quick Reference

| Task | Command |
|------|---------|
| Fresh dev stack | `make dev` (runs `edgequake migrate` before backend start) |
| Preflight | `edgequake migrate check` |
| Apply schema | `edgequake migrate` / `edgequake migrate --confirm-drop` |
| Drain data jobs | `edgequake migrate drain` |
| Preview | `edgequake migrate dry-run` |
| Check immutability | `./scripts/check_migration_checksums.sh` |
| Epoch coverage | `./scripts/check_epoch_coverage.sh` |
| Manifest SSOT | `edgequake/migrations/manifest.toml` |

**Train:** HEAD ships through migration **207** (`207_age_lineage_discovery_rls.sql`). The published v0.35.0 tag stops at **206**
(`170_spec164_plain_graph.sql` expand, `171_spec164_drop_age.sql` confirm-drop,
`172`–`202` request-path indexes, `203` chunk legacy-key index,
`204` graph label search, `205` drops empty `ag_catalog` only after the
extension is gone (a no-op while Apache AGE stays installed),
`206` leakproof modality, stage, and node search,
`207` AGE document-lineage lookup under row security).
Apply 170 with `edgequake migrate`. When `age` is absent from
`pg_available_extensions`, the same run records 171. When the package is
installed, apply 171 only with `--drop-age` after
`PostgresAGEGraphStorage::run_spec164_cutover` marks `eq_graph_cutover` ready.
`--confirm-drop` does not drop Apache AGE.
Migrations 172 through 207 do not raise the serving floor.
The released product pin is **v0.35.1** (schema 207). Tag v0.35.0 stops at 206.
`release_for_schema` maps 170–206 to v0.35.0 and 207 to v0.35.1.
Preparation notes: [`docs/operations/upgrade-to-0.35.1.md`](../../docs/operations/upgrade-to-0.35.1.md).
Run `edgequake migrate` before expecting `/ready` 200 when upgrading from an older schema.

The ledger sqlx trusts is `public._sqlx_migrations`. Each step runs on one connection with `search_path` set to `public`, because `CREATE TABLE IF NOT EXISTS _sqlx_migrations` lands in the first schema of `search_path`. If an older run left copies in `ag_catalog` or `edgequake`, `edgequake migrate` copies the missing rows into `public` and drops the stray tables before it decides what is pending. Serving never does that write.

`edgequake migrate status` prints the database schema, the binary schema, and the pending count before it lists data-engine jobs. `edgequake migrate` prints version spans (`172–207`) and `latest=207` (or `latest=none`).

Migration-guard CI Layer 4 replays a real v0.34.0 database (schema 169, published checksum of migration 167) with 80,000 documents through 202. That is the release proof. Raw `sqlx migrate run` is not: it does not fold a split ledger, repair the published 167 checksum, or honor `--confirm-drop`.

## How Migrations Work

1. **Numbered SQL files** — `NNN_description.sql` in `edgequake/migrations/`
2. **Explicit CLI apply** — `edgequake migrate` (admin pool + reconcile)
3. **Per-step progress** — `[ i/N ] version description … applied in Xs`
4. **Manifest** — phases, fossils, irreversible drops, compat window (SPEC-150)
5. **Immutability lock** — `checksums.lock` is append-only; never edit shipped bodies
6. **Support scripts** — `migrations/support/` ops-only SQL (also locked)
7. **Epoch proof** — every published `vX.Y.Z` maps to `scripts/spec150/epochs.toml`

### Serving vs migrate

| Process | Writes schema? | Behavior when behind |
|---------|----------------|----------------------|
| `edgequake migrate` | Yes | Applies expandables; gates drops behind `--confirm-drop` |
| API (`edgequake`) | No | `EDGEQUAKE_SCHEMA_GATE=fail` → exit 78; `wait` → lite `/live` + `/ready` 503 |

**Env vars (SPEC-150):** see [upgrading.md](../../docs/operations/upgrading.md) §7.

## Golden path (upgrade)

```bash
edgequake migrate check
edgequake migrate dry-run
edgequake migrate
edgequake migrate drain --timeout 3600   # optional data cutover
edgequake migrate --confirm-drop         # only when guard GREEN
curl -sf localhost:8080/ready
```

Full runbook: [`docs/operations/upgrading.md`](../../docs/operations/upgrading.md).  
Design: [`specs/150-reliable-migration-system/`](../../specs/150-reliable-migration-system/).  
Epoch proof: `make spec150-matrix-quick`.
