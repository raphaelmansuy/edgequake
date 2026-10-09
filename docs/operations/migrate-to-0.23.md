---
title: "Migrate to EdgeQuake v0.23.0+"
description: "Historical guide to migrate an older EdgeQuake database to v0.23.0 and later. See Upgrading for the current process."
---

# Migrate to EdgeQuake v0.23.0+

> **One rule:** the API server **never** changes the database schema. You run `edgequake migrate` (or a one-shot migrate container/Job) **before** starting new API replicas.

This page is the short, operator-facing guide for **v0.23.0** and **v0.24.0** (adds migration **142** SPEC-105 assert).  
Deep dive / production soak: [Upgrade from v0.22.0 (SPEC-091)](./spec091-upgrade-from-v0.22.0.md) · Boot gate design: [LD-15](../../specs/091-simplify-data-layer/17-boot-migration-gating.md).

---

## What changed

| | v0.22.0 | v0.23.0 | v0.24.0 |
| --- | --- | --- | --- |
| Schema | Migrations through **105** (KV-centric) | Migrations **106–141** (typed relational tables) | Through **142** (empty legacy assert) |
| Who applies schema | Could still auto-migrate at boot in some setups | **Only** `edgequake migrate` | same |
| If DB is behind | Server might apply migrations | Server **exits 78** and tells you to migrate | same |
| Dangerous steps | — | Drops of old KV/vector tables need `--confirm-drop` | same; **142** aborts if residue remains |

Irreversible drops (restore-from-backup only if you need to undo):

- **125** — drop legacy `eq_*_kv`
- **126** — drop legacy chunk vector tables
- **131** — drop remaining fleet `eq_*_vectors`
- **142** — SPEC-105 assert: DROP empty leftovers / abort if rows; deferred while residue remains

---

## Pick your path

### A) Fresh install (empty database)

No legacy data → no `--confirm-drop` needed.

```bash
# 1) Preview (optional)
edgequake migrate dry-run

# 2) Apply schema
edgequake migrate

# 3) Start the API (compose / make / K8s)
```

`make dev` / `make dev-bg` already run the migrate step for you before the server starts.

### B) Upgrade from v0.22.0 (existing data)

Treat this as a **planned maintenance** change. Backup first.

```text
1. Backup Postgres          (pg_dump -Fc or volume snapshot)
2. Roll ALL API replicas    to the v0.23.0 binary (no mixed 0.22 + 0.23 after drop)
3. Preview                  edgequake migrate dry-run
4. Apply safe schema        edgequake migrate
5. Apply irreversible drops edgequake migrate --confirm-drop
6. Start / keep API         boot only verifies schema (LD-15)
7. Smoke-check              /health, list docs, one query, wipe one workspace
```

```bash
export DATABASE_URL=postgres://edgequake:…@…/edgequake

edgequake migrate dry-run                 # preview only — no writes
edgequake migrate                         # expandable schema; may stop before drops
edgequake migrate --confirm-drop          # 125 / 126 / 131 when you are ready
# then start API replicas on v0.23.0
```

Docker one-shot (same idea):

```yaml
services:
  migrate:
    image: ghcr.io/raphaelmansuy/edgequake:0.23.0
    command: ["migrate"]   # later: ["migrate", "--confirm-drop"] after dry-run review
    environment:
      DATABASE_URL: postgres://edgequake:${POSTGRES_PASSWORD}@postgres:5432/edgequake
    restart: "no"
  api:
    image: ghcr.io/raphaelmansuy/edgequake:0.23.0
    depends_on:
      migrate: { condition: service_completed_successfully }
```

---

## Commands cheat sheet

| Command | What it does |
| --- | --- |
| `edgequake migrate dry-run` | Shows pending migrations; **writes nothing** |
| `edgequake migrate` | Applies expandable (“safe”) migrations; refuses irreversible drops until confirmed |
| `edgequake migrate --confirm-drop` | Applies irreversible drops (**125 / 126 / 131**) after you consent |
| `edgequake migrate console` | Live posture / next-step advisor |
| `edgequake migrate guard` | Drop-readiness check |

Binary via cargo (from repo):

```bash
cargo run -p edgequake --features postgres -- migrate dry-run
cargo run -p edgequake --features postgres -- migrate
cargo run -p edgequake --features postgres -- migrate --confirm-drop
```

---

## If the server won’t start

Exit code **78** usually means: schema and binary disagree.

1. Read the log line — it names pending count and the two commands.
2. Run `edgequake migrate dry-run`, then `edgequake migrate` (add `--confirm-drop` only when the dry-run shows irreversible drops and you have a backup).
3. Start the API again.

`GET /health` → `schema.pending_count` / `schema.migration_required` show drift after boot (e.g. one replica still old while the fleet migrated).

---

## API behavior note (SPEC-104, post-0.23.0 patch)

No new SQL migrations for SPEC-104 — code-only monitor / tenant fixes. Operators should know the **tenant create** HTTP contract changed vs 0.22:

| `POST /api/v1/tenants` | Status |
| --- | --- |
| New slug | **201 Created** |
| Same slug + same name | **200 OK** (idempotent) |
| Same slug + different name | **409 Conflict** (was often 400) |

Storage inspect (`GET /api/v1/admin/storage/inspect`) on a SPEC-104+ binary must not emit Postgres `42703` (`workspaces.id`) or `42P01` (`edgequake."Node"`). Details: [`specs/104-fix-datalayer/`](../../specs/104-fix-datalayer/).

### SPEC-105 (legacy cutover assert — migration **142**)

Upgrade from **≤ v0.22.0** is unchanged until irreversible drops finish:

1. Roll write-stop binary → `edgequake migrate` (expandable, including toward **142**)
2. When durable `eq_*_kv` / `eq_*_vectors` rows remain: `edgequake migrate --confirm-drop` (**125 / 126 / 131**)
3. Migration **142** then drops **empty** leftovers and sets `server_config.legacy_stores_forbidden`; it **aborts** if rows remain (never silent delete). While durable legacy rows remain, expandable `edgequake migrate` **defers** 142 (soft-exit OK) so mid-upgrade fleets are not blocked.

Unknown `EDGEQUAKE_VECTOR_BACKEND` now selects **typed_embeddings** (not legacy). Spec: [`specs/105-fix-legacy/`](../../specs/105-fix-legacy/).

---

## Rollback

| When | How |
| --- | --- |
| Before `--confirm-drop` finishes 125 | Redeploy previous API image; additive migrations may remain |
| After 125 / 126 / 131 applied | **Restore from backup only** — there is no flag flip |

---

## Related

| Doc | Role |
| --- | --- |
| [spec091-upgrade-from-v0.22.0.md](./spec091-upgrade-from-v0.22.0.md) | Full production runbook, flags, soak, Compose/K8s detail |
| [17-boot-migration-gating.md](../../specs/091-simplify-data-layer/17-boot-migration-gating.md) | Why boot never migrates (LD-15) |
| [release-and-cd.md](./release-and-cd.md) | Release gates and GHCR tags |
| `make spec091-upgrade-soak` / `make spec93-migration-assessment` | Automated upgrade proofs |
