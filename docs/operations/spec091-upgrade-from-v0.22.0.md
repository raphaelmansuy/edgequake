---
title: "SPEC-091 Upgrade from v0.22.0"
description: "Runbook for upgrading a live v0.22.0 PostgreSQL fleet through the SPEC-091 data-layer cutover to v0.23.0 and later, with risks, flags, rollback and automated soak tests."
---

# SPEC-091 — Upgrade from v0.22.0

This runbook is for operators who upgrade a live Postgres database from GHCR **v0.22.0** (migrations up to **105**, KV as the source of truth) to **v0.23.0 or later** (migrations **106–142**: the typed relational source of truth, irreversible KV and vector drops, the RM0–RM5 outbox drain, citation and chunk FTS, AGE citation indexes, SPEC-098 spine and lifecycle, and the **SPEC-105** legacy cutover assert).

For the short version, read [Migrate to v0.23.0](./migrate-to-0.23.md). The spec pack is in [`specs/091-simplify-data-layer/`](../../specs/091-simplify-data-layer/). Risks R-21 to R-29 are in `09-risk-register.md`.

> **Automated proof:** `make spec93-migration-assessment` (PG16, 17 and 18 realism), `make spec091-upgrade-soak` (smoke) and `make spec091-gates`. The formal pack is in [`specs/93-migration-assessment/`](../../specs/93-migration-assessment/).

## Risk summary

| Risk | Why it matters | Mitigation |
| --- | --- | --- |
| Migration **125** is irreversible | After the `eq_*_kv` drop, rollback means **restoring from backup** | The `--confirm-drop` gate; a durable-row guard aborts if the typed source of truth is incomplete |
| Replica skew (R-27) | A stale binary treats missing KV as a hard error and fails ingests | Roll **all** replicas to the write-stop build before or with the drop |
| Stale `kv` or `dual` flags | After the drop, these flags hit `42P01` or take the wrong path | Keep `EDGEQUAKE_CHUNK_TEXT_AUTHORITY` and `EDGEQUAKE_KV_FAMILY_*=relational`; check with `edgequake migrate console` |
| Serving fence (R-28) | A wrong JOIN zeroes retrieval for every workspace | Keep the fence on after a query proof; the JOIN must use `public.chunk_serving_state` |
| Multi-tenant shell drift (R-21, R-24) | A wipe or membership change can miss shell documents | Schema-qualify `public.documents`; check that a wipe in one workspace leaves the others intact |

**Verdict:** treat this upgrade as **high operational risk** until `make spec93-migration-assessment` is green for your class of data, or until an equivalent restore of a production dump passes.

## Prerequisites

1. A verified backup or restore point. A custom-format `pg_dump -Fc` is recommended.
2. pgvector 0.8.2 or later (0.8.5 preferred), and AGE at the tier pinned for your Postgres major.
3. Every replica runs the **same** v0.23.0 write-stop binary. Mixed fleets are not allowed across the drop.
4. A maintenance window long enough for the SQL backfills 117–124, plus the optional chunk-text job on large corpora.

## Flag matrix

Serving boot **never** applies migrations. A pending schema makes the server exit 78 with a dry-run hint. `edgequake migrate` is the only schema writer.

```bash
export EDGEQUAKE_MIGRATION_MODE=automatic      # or verify, then automatic
export EDGEQUAKE_CHUNK_TEXT_AUTHORITY=relational
export EDGEQUAKE_KV_FAMILY_DOC_HASH=relational
export EDGEQUAKE_KV_FAMILY_WSDOC=relational
export EDGEQUAKE_KV_FAMILY_CHECKPOINT=relational
export EDGEQUAKE_KV_FAMILY_ARTIFACT=relational
export EDGEQUAKE_KV_FAMILY_INJECTION=relational
export EDGEQUAKE_KV_FAMILY_METADATA=relational
# The serving fence is on by default from v0.23.0. Set it to off only during a dual-write soak.
# export EDGEQUAKE_SERVING_FENCE=off
# The outbox drain is on by default (RM0). Escape hatch: EDGEQUAKE_OUTBOX_DRAIN=off
# Do NOT set EDGEQUAKE_MIGRATION_CONFIRM_DROP=1 in a shared env file.
```

`make dev` defaults assume a **post-drop** database. Do not copy them onto a mid-upgrade v0.22.0 fleet without reading this runbook.

## Operator sequence

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TB
    A["Fleet on v0.22.0 (schema up to 105)"] --> B["Back up Postgres (pg_dump -Fc)"]
    B --> C["Roll ALL replicas to the 0.23.0 write-stop build"]
    C --> D["migrate dry-run (preview, no writes)"]
    D --> E["migrate (expandable steps first; stops before the drops)"]
    E --> F["migrate --confirm-drop (irreversible 125, 126, 131)"]
    F --> G["Start the 0.23.0 API (verify-only boot)"]
    G --> H["Verify tenants, wipe, assets, and query"]
    H --> I["Turn the serving fence on"]
%% eq-classes
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class B eqStore
```

Read the diagram top to bottom; the numbered steps below expand each box.

0. **Dry run** (preview only). Point `DATABASE_URL` at the target database and inspect the pending work. Nothing is applied.

   ```bash
   cargo run -p edgequake --features postgres -- migrate dry-run
   ```

   The run ends with `dry-run complete: no migrations applied (preview only).` It lists each pending migration and marks 125 as irreversible. Check the drop readiness it reports before you continue.

1. **Back up** the database (`pg_dump -Fc` or a volume snapshot) and record the restore point.
2. **Drain or stop writers** if your SLO needs a quiet window. This is recommended before `--confirm-drop`.
3. **Roll every API replica** to the v0.23.0 binary that treats KV `42P01` as "source gone" (write-stop). Do not leave a v0.22.0 API process attached after 125.
4. Run the expandable migrations, then check the guards:

   ```bash
   cargo run -p edgequake --features postgres -- migrate
   cargo run -p edgequake --features postgres -- migrate console
   cargo run -p edgequake --features postgres -- migrate guard
   ```

   The expandable steps run first. When only irreversible drops remain, the command exits 0 with a WARN. If an irreversible drop blocks a later expandable migration, the command refuses to run.

5. When you are ready to contract (irreversible):

   ```bash
   cargo run -p edgequake --features postgres -- migrate --confirm-drop
   ```

   sqlx applies the pending migrations 106–125 in order. The family SQL backfills (117–124) run first. Migration **125** then purges the KV keys that already exist in the typed source of truth, and runs the durable-row guard. If un-migrated residue remains, 125 aborts and the database stays pre-drop. Fix the residue, restore if needed, and retry. On success, stdout reports `KV store dropped (migration 125). Rollback = restore from backup.`

6. **Start the v0.23.0 API** with the relational flag matrix above. Boot only verifies the schema. If any migration were still pending, the server would exit 78.
7. **Verify** for each tenant and workspace:
   - `GET /health` reports healthy.
   - The document list is non-empty where documents were seeded.
   - Queries return grounded sources, not `sources: null` with populated vectors.
   - Asset paths do not return 500 with `relation eq_*_kv does not exist`.
   - A wipe in one workspace leaves the other workspaces and tenants intact.
8. Optionally set `EDGEQUAKE_SERVING_FENCE=on` after a successful query proof.

## Boot gate (LD-15)

Server start never applies versioned schema. Boot reads `_sqlx_migrations` and decides:

- **Expandable migrations pending:** exit **78** (`EX_CONFIG`), with the pending count and the `migrate dry-run` and `migrate` hints.
- **Only irreversible drops pending (125, 126, 131):** boot continues with a WARN. Health still reports `migration_required` until `--confirm-drop` runs (LD-07).
- **Database newer than the binary:** exit 78 (downgrade protection).
- **Up to date:** the server serves. Reconcile hooks stay read-only probes.

`/health.schema.pending_count` and `migration_required` show drift after boot, for example when a replica is still up while the fleet migrated. Spec: [`specs/091-simplify-data-layer/17-boot-migration-gating.md`](../../specs/091-simplify-data-layer/17-boot-migration-gating.md).

## Docker Compose: one-shot migrate service

```yaml
services:
  migrate:
    image: ghcr.io/raphaelmansuy/edgequake:${EDGEQUAKE_VERSION}
    command: ["migrate"]              # add "dry-run" for a preview-only run
    environment:
      DATABASE_URL: postgres://edgequake:${POSTGRES_PASSWORD}@postgres:5432/edgequake
    depends_on:
      postgres: { condition: service_healthy }
    restart: "no"                     # one-shot; exits 0 when the schema is current

  api:
    image: ghcr.io/raphaelmansuy/edgequake:${EDGEQUAKE_VERSION}
    depends_on:
      migrate: { condition: service_completed_successfully }
```

Migrations 125 and 126 on an **upgraded** database still need an explicit one-shot with `command: ["migrate", "--confirm-drop"]`, run after you review the `dry-run`. Never put that in an always-on service. A fresh install (no applied migrations) needs no flag.

## Kubernetes: migrate Job before the Deployment rolls

```yaml
apiVersion: batch/v1
kind: Job
metadata: { name: edgequake-migrate }
spec:
  backoffLimit: 1
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: migrate
          image: ghcr.io/raphaelmansuy/edgequake:VERSION
          args: ["migrate"]           # or ["migrate", "--confirm-drop"] after a dry-run review
          env:
            - name: DATABASE_URL
              valueFrom: { secretKeyRef: { name: edgequake-db, key: url } }
```

Apply the Job, wait for it to complete, then roll the Deployment. New replicas crash-loop with exit 78 until the Job completes, which makes the wait visible. For defence in depth, you can also gate readiness on `/health.schema.migration_required == false`.

## Rollback

| Phase | Rollback |
| --- | --- |
| Before `--confirm-drop` completes 125 | Redeploy the previous binary. Additive (expand-phase) migrations may remain. |
| After 125 is applied | **Restore from backup only.** There is no flag flip that brings back the dropped `eq_*_kv` tables. |

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TB
    A{"Has migration 125 been applied?"} -->|no| B["Redeploy the previous API image"]
    B --> C["Additive migrations may remain; that is safe"]
    A -->|yes| D["Restore the pre-upgrade backup"]
    D --> E["No flag can undo the KV drop"]
%% eq-classes
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class A,C,E eqStore
```

Notice that the rollback path depends on whether migration 125 has already dropped the KV tables.

## Automated soak

```bash
# Formal realism matrix (SPEC-93): 5 tenants x 3 workspaces x 40 documents x PG16/17/18
make spec93-migration-assessment

# Legacy smoke test (tiny corpus, default postgres tag)
make spec091-upgrade-soak
```

**SPEC-93** is the binding proof pack, in [`specs/93-migration-assessment/`](../../specs/93-migration-assessment/). It pulls `ghcr.io/raphaelmansuy/edgequake:0.22.0` and the matching `edgequake-postgres:0.22.0-pg{16,17,18}` images. It seeds a realism corpus, dumps the database, runs the v0.23.0 `migrate dry-run` (asserting a preview and no schema advance), then runs `migrate --confirm-drop` through migrations **106–141**. Finally it asserts isolation, listing, wipe, assets, and retrieval with the fence on. Reports are in `specs/93-migration-assessment/reports/`.

## Manual soak with a real dump

1. Restore a v0.22.0-era dump into an ephemeral Postgres of the same major version.
2. Point `DATABASE_URL` at it and follow the operator sequence above.
3. Run the same HTTP and SQL checks as the soak script (multi-tenant isolation and wipe).

Rehearse on a staging restore before you run `--confirm-drop` in production.

## Related

- Release and CD: [release-and-cd.md](./release-and-cd.md)
- Cancel and fairness (live today, not SPEC-120): [ingestion-cancel-and-fairness.md](../ingestion-cancel-and-fairness.md)
- SPEC-120 status (orphaned WIP): [`specs/92-task-system/README.md`](../../specs/92-task-system/README.md)
