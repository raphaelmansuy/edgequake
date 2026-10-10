---
title: "Migrate to EdgeQuake v0.23.0+"
description: "Historical operator guide for moving an EdgeQuake database to v0.23.0 and later: fresh install versus upgrade from v0.22.0, migrate commands, exit 78 and rollback."
---

# Migrate to EdgeQuake v0.23.0+

> **One rule:** the API server **never** changes the database schema. Run `edgequake migrate` (or a one-shot migrate container or Job) **before** you start new API replicas.

This page is the short operator guide for **v0.23.0** and **v0.24.0** (v0.24.0 adds migration **142**, the SPEC-105 assert). For the deep dive and the production soak, read [Upgrade from v0.22.0 (SPEC-091)](./spec091-upgrade-from-v0.22.0.md). For the boot gate design, read [LD-15](../../specs/091-simplify-data-layer/17-boot-migration-gating.md).

## What changed

| | v0.22.0 | v0.23.0 | v0.24.0 |
| --- | --- | --- | --- |
| Schema | Migrations through **105** (KV-centric) | Migrations **106–141** (typed relational tables) | Through **142** (empty legacy assert) |
| Who applies schema | Could still auto-migrate at boot in some setups | **Only** `edgequake migrate` | Same |
| If the DB is behind | The server might apply migrations | The server **exits 78** and tells you to migrate | Same |
| Dangerous steps | None | Dropping old KV and vector tables needs `--confirm-drop` | Same; **142** aborts if residue remains |

Irreversible drops (restore from backup if you need to undo them):

- **125**: drops legacy `eq_*_kv`
- **126**: drops legacy chunk vector tables
- **131**: drops the remaining fleet `eq_*_vectors`
- **142**: SPEC-105 assert. Drops empty leftovers, and aborts if rows remain. It is deferred while residue exists.

## Which path are you on?

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TB
    A["Pick a path"] --> B{"Does the database hold legacy data?"}
    B -->|"no: fresh install"| C["edgequake migrate dry-run (optional)"]
    C --> D["edgequake migrate"]
    D --> E["Start the API"]
    B -->|"yes: upgrade from 0.22.0"| F["Back up Postgres"]
    F --> G["Roll ALL API replicas to the 0.23.0 binary"]
    G --> H["edgequake migrate dry-run"]
    H --> I["edgequake migrate (expandable steps)"]
    I --> J["edgequake migrate --confirm-drop (125, 126, 131)"]
    J --> K["Start the API (boot only verifies the schema)"]
%% eq-classes
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class B,F eqStore
```

A fresh install has nothing legacy to lose, so it needs no `--confirm-drop`.

## Fresh install

```bash
# 1) Preview (optional)
edgequake migrate dry-run

# 2) Apply the schema
edgequake migrate

# 3) Start the API (compose, make or Kubernetes)
```

`make dev` and `make dev-bg` run the migrate step before the server starts.

## Upgrade from v0.22.0 (existing data)

Treat this as planned maintenance, and take a backup first.

```text
1. Back up Postgres           (pg_dump -Fc or a volume snapshot)
2. Roll ALL API replicas      to the v0.23.0 binary (never mix 0.22 and 0.23 after a drop)
3. Preview                    edgequake migrate dry-run
4. Apply safe schema          edgequake migrate
5. Apply irreversible drops   edgequake migrate --confirm-drop   (125, 126, 131 when ready)
6. Start or keep the API      boot only verifies the schema (LD-15)
7. Smoke-check                /health, list documents, run one query, wipe one workspace
```

```bash
export DATABASE_URL=postgres://edgequake:…@…/edgequake

edgequake migrate dry-run      # preview only; writes nothing
edgequake migrate              # expandable schema; may stop before the drops
edgequake migrate --confirm-drop   # 125 / 126 / 131, once you are ready
# then start the API replicas on v0.23.0
```

Docker one-shot (same idea):

```yaml
services:
  migrate:
    image: ghcr.io/raphaelmansuy/edgequake:0.23.0
    command: ["migrate"]   # later: ["migrate", "--confirm-drop"] after a dry-run review
    environment:
      DATABASE_URL: postgres://edgequake:${POSTGRES_PASSWORD}@postgres:5432/edgequake
    restart: "no"
  api:
    image: ghcr.io/raphaelmansuy/edgequake:0.23.0
    depends_on:
      migrate: { condition: service_completed_successfully }
```

## Boot behaviour

Boot reads the migration ledger and either serves, soft-boots with a warning, or exits 78:

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
stateDiagram-v2
    [*] --> Boot
    Boot --> Serve: schema is current
    Boot --> SoftBoot: only irreversible drops pending
    Boot --> Exit78: expandable migrations pending or database newer than binary
    SoftBoot --> Serve: WARN logged; health reports migration_required
```

Exit code **78** usually means the schema and binary disagree. Read the log line, which names the pending count and the two commands to run.

## Commands cheat sheet

| Command | What it does |
| --- | --- |
| `edgequake migrate dry-run` | Shows pending migrations; **writes nothing** |
| `edgequake migrate` | Applies the expandable (safe) migrations; refuses irreversible drops until you confirm |
| `edgequake migrate --confirm-drop` | Applies the irreversible drops (**125 / 126 / 131**) after you consent |
| `edgequake migrate console` | Live posture and next-step advisor |
| `edgequake migrate guard` | Drop-readiness check |

From a source checkout, use cargo:

```bash
cargo run -p edgequake --features postgres -- migrate dry-run
cargo run -p edgequake --features postgres -- migrate
cargo run -p edgequake --features postgres -- migrate --confirm-drop
```

## If the server will not start

1. Read the log line. It names the pending count and the commands to run.
2. Run `edgequake migrate dry-run`, then `edgequake migrate`. Add `--confirm-drop` only when the dry-run shows irreversible drops and you have a backup.
3. Start the API again.

`GET /health` shows `schema.pending_count` and `schema.migration_required`. They reveal drift after boot, for example when one replica still runs the old binary while the fleet has migrated.

## API behaviour change (SPEC-104)

SPEC-104 added no SQL migrations, only code fixes for the monitor and tenant paths. The tenant create contract changed:

| `POST /api/v1/tenants` | Status |
| --- | --- |
| New slug | **201 Created** |
| Same slug and same name | **200 OK** (idempotent) |
| Same slug and different name | **409 Conflict** (was often 400 in 0.22) |

Storage inspect (`GET /api/v1/admin/storage/inspect`) on a SPEC-104 binary must not return Postgres `42703` (`workspaces.id`) or `42P01` (`edgequake."Node"`). Details: [`specs/104-fix-datalayer/`](../../specs/104-fix-datalayer/).

## SPEC-105 legacy cutover (migration 142)

From v0.22.0 or older, the upgrade is unchanged until the irreversible drops finish:

1. Roll the write-stop binary, then run `edgequake migrate`. This applies the expandable migrations, including those up to **142**.
2. While durable `eq_*_kv` or `eq_*_vectors` rows remain, run `edgequake migrate --confirm-drop` (**125 / 126 / 131**).
3. Migration **142** drops the **empty** leftovers and sets `server_config.legacy_stores_forbidden`. It aborts if rows remain and never deletes data silently. While durable legacy rows remain, a plain `edgequake migrate` defers 142, so a mid-upgrade fleet is not blocked.

An unknown `EDGEQUAKE_VECTOR_BACKEND` value now selects **typed_embeddings**, not the legacy backend. Spec: [`specs/105-fix-legacy/`](../../specs/105-fix-legacy/).

## Rollback

| When | How |
| --- | --- |
| Before `--confirm-drop` finishes 125 | Redeploy the previous API image. Additive migrations may remain. |
| After 125, 126 or 131 is applied | **Restore from backup only.** No flag can undo the drop. |

## Related

| Doc | Role |
| --- | --- |
| [spec091-upgrade-from-v0.22.0.md](./spec091-upgrade-from-v0.22.0.md) | Full production runbook, flags, soak, and Compose and Kubernetes detail |
| [17-boot-migration-gating.md](../../specs/091-simplify-data-layer/17-boot-migration-gating.md) | Why boot never migrates (LD-15) |
| [release-and-cd.md](./release-and-cd.md) | Release gates and GHCR tags |
| `make spec091-upgrade-soak` and `make spec93-migration-assessment` | Automated upgrade proofs |
