---
title: "Upgrade to EdgeQuake v0.35.0"
description: "Upgrade from v0.34.0 to v0.35.0. Schema moves 169 to 206. Apache AGE is optional."
---

# Upgrade to EdgeQuake v0.35.0

> **From:** v0.34.0 (schema **169**) · **To:** v0.35.0 (schema **206**)

**demo.edgequake.com:** this cut installs the 0.35.0 API and WebUI. Non-interactive
migrate leaves Apache AGE installed (migration 171 stays pending). Password auth
remains the demo default.

Fresh install on Postgres without AGE:
[specs/164-pure-postgres-study/README.md](../../specs/164-pure-postgres-study/README.md).

The train is migrations **170** through **206**:

| Migration | What it does | Consent |
|-----------|--------------|---------|
| 170 | Plain graph tables. Does not raise the serving floor, so a v0.34 binary can still serve. | `edgequake migrate` |
| 171 | Drops Apache AGE after the cutover ledger is `ready`. Default is to leave AGE. | `edgequake migrate --drop-age` (or answer yes) |
| 172–202 | Request-path indexes. Do not raise the serving floor. | `edgequake migrate` (applied even when 171 is still pending) |
| 203 | Stored `chunks.legacy_chunk_key` so id lookup is an index condition under row security. | `edgequake migrate` |
| 204 | Label search runs as the table owner so `LIKE` can use the trigram index under row security. | `edgequake migrate` |
| 205 | Drops the empty `ag_catalog` schema left behind after Apache AGE is gone. Refuses if the cutover is not ready or a leftover table has rows. | `edgequake migrate` |
| 206 | Stored `modality` and `current_stage` columns, plus owner-side node search, so those predicates are index conditions under row security. | `edgequake migrate` |

`release_for_schema` maps 170–206 to v0.35.0. Migration **207** ships in [v0.35.1](upgrade-to-0.35.1.md). Schema 169 stays the published v0.33.0 / v0.34.0 point.

## What was proved

Migration-guard CI Layer 4 runs `e2e_spec164_v034_large_upgrade` on PostgreSQL. The test builds a database the way v0.34.0 left it: stop at migration **169**, then rewrite the stored checksum of migration **167** to the hash published in tag `v0.34.0`. It loads 80,000 documents, 40,000 chunks, 30,000 conversations, and 80,000 tasks, then upgrades with the product migrator.

Measured on that corpus ([v034_large_upgrade.json](../../specs/164-pure-postgres-study/measurements/v034_large_upgrade.json)):

| Check | Result |
|-------|--------|
| Empty database, migrations 1–169 | 2.008 s |
| Upgrade 170–202 | 3.003 s |
| Row counts | unchanged |
| Ledger | `public._sqlx_migrations` only, head 202 |
| Apache AGE | left installed unless migrate is answered yes (`--drop-age`) |
| `status=failed` page | 0.081 ms, no sequential scan |

The same test folds a stray ledger: if versions 15 and 16 were recorded outside `public`, the next `edgequake migrate` copies them back and drops the stray table.

## Operator steps (when the tag exists)

Until then, use these steps on a backup of a v0.34.0 database if you are rehearsing the train. Full command reference: [upgrading.md](upgrading.md).

```bash
pg_dump -Fc "$DATABASE_URL" > edgequake-$(date +%F).dump
edgequake migrate check
edgequake migrate dry-run
edgequake migrate
# 171 stays pending until the cutover ledger is ready and you consent:
edgequake migrate --confirm-drop
edgequake migrate status
```

`edgequake migrate status` prints the schema before any data-engine jobs:

```text
schema:
  database : 169 (≈ v0.33.0)
  binary   : 206 (≈ v0.35.0)
  pending  : 36
  drop old : 171 (drops Apache AGE after the SPEC-164 plain-graph ledger is ready)
```

After a successful apply the closing line is `migrate ok: pending_before=… latest=206 applied_this_run=…`. An empty database prints `latest=none`. Lists of versions are spans (`172–206`), and drop steps name what they delete.

## What this command will not do

- It will not apply schema from the API process. Only `edgequake migrate` writes numbered migrations.
- It will not drop Apache AGE unless you answer yes. `--confirm-drop` still gates the other DROP OLD steps. Without `--drop-age`, safe schema on both sides of 171 is applied and 171 stays pending.
- It will not repair an unknown checksum. The published v0.34.0 hash of migration 167 is a known fossil and is rewritten to the current file hash before sqlx runs. Any other mismatch stops.
- It will not write a second `_sqlx_migrations` table. Each step pins `search_path` to `public`.
