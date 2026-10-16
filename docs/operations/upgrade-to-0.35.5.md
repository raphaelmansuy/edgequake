---
title: "Upgrade to EdgeQuake v0.35.5"
description: "Upgrade from v0.35.4 to v0.35.5. Schema stays 207. Published Postgres images no longer include Apache AGE."
---

# Upgrade to EdgeQuake v0.35.5

> **From:** v0.35.4 (schema **207**) · **To:** v0.35.5 (schema **207**)

No new migration file. Schema stays **207**. `release_for_schema` still maps 207 to v0.35.1, the release that first shipped that schema.

The published `edgequake-postgres` images for v0.35.5 (pg16, pg17, and pg18) do not contain the Apache AGE library. pgvector 0.8.5, `pg_trgm`, `btree_gin`, and `uuid-ossp` stay. v0.35.4 is the last image that includes AGE.

A data directory created by the v0.35.4 Postgres image has already loaded `age`. The v0.35.5 image has no `age.so`, so Postgres will refuse to open that directory. Use a fresh volume. An existing database that still has the extension must run `edgequake migrate --drop-age` on the v0.35.4 image before the volume moves.

On a server where `age` is not in `pg_available_extensions`, `edgequake migrate` records migration 171 in the same run as the safe schema. Nothing is dropped: migration 170 already marks that database `ready` and `age_absent`. Afterward `/health` reports `pending_count` 0 and `migration_required` false.

On a server where the AGE package is still installed, the default is unchanged. Non-interactive migrate does not drop the extension unless you pass `--drop-age` or set `EDGEQUAKE_MIGRATE_DROP_AGE=yes`. Migration 171 stays pending until then.

**SPEC-001 Acc:** medical-mid n=200, archive [`medical-mid-20261016T162456Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20261016T162456Z/) (`valid: true`). EdgeQuake **0.8054**, LightRAG **0.7920**, delta **+0.0135**, 95% CI **[+0.0056, +0.1506]** (n_paired 40). The harness marks this as not a publishable beat because L2 is incomplete. EdgeQuake does not claim a win. Evidence recall is 0.921 versus 0.956, and context relevancy is 0.437 versus 0.508. The answer prompt is the same as v0.35.4.

## Operator steps

```bash
pg_dump -Fc "$DATABASE_URL" > edgequake-$(date +%F).dump
```

Fresh volume on the v0.35.5 Postgres image:

```bash
edgequake migrate
curl -sf localhost:8080/health
```

Expect `schema.latest_version` 207, `schema.pending_count` 0, `schema.migration_required` false, and `components.graph_available` true. `pg_available_extensions` has `vector` and does not have `age`.

Existing database that still has Apache AGE, still on the v0.35.4 image:

```bash
edgequake migrate --drop-age
```

Only after that drop succeeds, move the volume or restore the dump onto v0.35.5. Full command reference: [upgrading.md](upgrading.md).
