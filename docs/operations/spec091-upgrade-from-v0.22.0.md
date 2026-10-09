---
title: "SPEC-091 Upgrade from v0.22.0"
description: "Runbook for upgrading a v0.22.0 fleet through the SPEC-091 data-layer cutover."
---

# SPEC-091 — Upgrade from published v0.22.0

> **Start here if you want the short version:** [Migrate to v0.23.0](./migrate-to-0.23.md) (plain-language paths for fresh install vs upgrade).
> **Audience:** operators upgrading a live Postgres from GHCR **v0.22.0** (migrations ≤ **105**, KV SSOT) to **v0.23.0+** (migrations **106–142**: typed relational SSOT + irreversible KV/vector drops + RM0–RM5 outbox drain / citation / chunk FTS / AGE citation indexes + SPEC-098 spine/lifecycle + **SPEC-105** legacy cutover assert).
> **Spec:** [`specs/091-simplify-data-layer/`](../../specs/091-simplify-data-layer/) · risks R-21..R-29 in `09-risk-register.md` · RM program in [`22-ingestion-migration-system-assessment.md`](../../specs/091-simplify-data-layer/22-ingestion-migration-system-assessment.md).
> **Legacy cutover (SPEC-105):** after 125/126/131 `--confirm-drop`, migration **142** asserts empty leftovers (aborts if rows remain). Unknown `EDGEQUAKE_VECTOR_BACKEND` → typed. Pack: [`specs/105-fix-legacy/`](../../specs/105-fix-legacy/).
> **Automated proof:** `make spec93-migration-assessment` (PG16/17/18 realism) · `make spec091-upgrade-soak` (smoke) · `make spec091-gates`.
> **Formal pack:** [`specs/93-migration-assessment/`](../../specs/93-migration-assessment/).

## Risk summary

| Risk | Why it matters | Mitigation |
| --- | --- | --- |
| Migration **125** is irreversible | After `eq_*_kv` drop, rollback = **restore from backup** | `--confirm-drop` gate; durable-row guard aborts if typed SSOT incomplete |
| Replica skew (R-27) | Stale binary after drop treats missing KV as hard error → failed ingests | Roll **all** replicas to the write-stop build **before/with** the drop |
| Stale `kv`/`dual` flags | Post-drop flags hit `42P01` or wrong path | Keep `EDGEQUAKE_CHUNK_TEXT_AUTHORITY` + `EDGEQUAKE_KV_FAMILY_*=relational`; use `edgequake migrate console` |
| Serving fence (R-28) | Wrong JOIN zeros retrieval for every workspace | Fence OFF until query proof; JOIN must be `public.chunk_serving_state` |
| Multi-tenant shell drift (R-21/R-24) | Wipe/membership can miss shell docs | Schema-qualify `public.documents`; verify wipe on one WS leaves others intact |

**Verdict:** treat this upgrade as **high operational risk** until `make spec93-migration-assessment` is green on your class of data (or an equivalent restore of a production dump). Smoke: `make spec091-upgrade-soak`.

## Prerequisites

1. Verified backup / restore point (custom-format `pg_dump -Fc` recommended).
2. pgvector ≥ 0.8.2 (0.8.5 preferred), AGE at the tier pin for your Postgres major.
3. Every replica scheduled to run the **same** v0.23.0 (write-stop) binary — no mixed fleets across the drop.
4. Maintenance window sized for SQL backfills 117–124 + optional engine chunk-text job on large corpora.

## Flag matrix (upgrade-safe)

```bash
# LD-15: serving boot NEVER applies migrations — pending schema ⇒ exit-78
# refusal with a dry-run/migrate hint. There is no flag to set; `edgequake
# migrate` (below) is the only schema writer.
export EDGEQUAKE_MIGRATION_MODE=automatic      # or verify, then automatic
export EDGEQUAKE_CHUNK_TEXT_AUTHORITY=relational
export EDGEQUAKE_KV_FAMILY_DOC_HASH=relational
export EDGEQUAKE_KV_FAMILY_WSDOC=relational
export EDGEQUAKE_KV_FAMILY_CHECKPOINT=relational
export EDGEQUAKE_KV_FAMILY_ARTIFACT=relational
export EDGEQUAKE_KV_FAMILY_INJECTION=relational
export EDGEQUAKE_KV_FAMILY_METADATA=relational
# Fence defaults ON at v0.23.0 (LAW-IP1). Escape with off only during dual-write soak.
# export EDGEQUAKE_SERVING_FENCE=off
# Outbox drain defaults ON (RM0). Escape: EDGEQUAKE_OUTBOX_DRAIN=off
# Do NOT set EDGEQUAKE_MIGRATION_CONFIRM_DROP=1 in a shared env file casually.
```

`make dev` defaults assume a **post-drop** database. Do not copy those defaults onto a mid-upgrade v0.22.0 fleet without reading this runbook.

## Operator sequence

```ascii
 v0.22.0 (≤105)  →  backup  →  roll ALL replicas to v0.23.0 (write-stop)
                              →  edgequake migrate dry-run          # preview only (no writes)
                              →  edgequake migrate                  # refuses 125; prints guard
                              →  edgequake migrate --confirm-drop   # 106–131 (+ 132–137 SAFE)
                              →  start v0.23.0 API (boot migrate off; LD-15)
                              →  verify multi-WS query / wipe / assets / outbox drain
                              →  fence default on; confirm retrieval with query_ready
```

### Step-by-step

0. **Dry-run (preview only)** — after pointing `DATABASE_URL` at the target DB (and preferably after rolling write-stop replicas), inspect pending work **without applying anything**:

   ```bash
   cargo run -p edgequake --features postgres -- migrate dry-run
   ```

   Expected stdout shape (abridged):

   ```text
   EdgeQuake migrate v…
   database: postgresql://…:****@…
   MODE: DRY-RUN (no changes will be applied)

   preflight: N pending migration(s)
     pending 106 — …  [expandable]
     …
     pending 125 — …  [IRREVERSIBLE — KV drop]

   RISK: migration 125 is PENDING and IRREVERSIBLE …

   FAMILY / NEXT (runbook) / ACTIONS / GUARD drop-readiness …
   UPGRADE CHECKLIST …
   dry-run complete: no migrations applied (preview only).
   ```

   Exit **0** even when drop-readiness is RED (information). Non-zero only on connect/advisor hard errors. `_sqlx_migrations` max version must stay unchanged.

1. **Backup** the database (`pg_dump -Fc` or volume snapshot). Record restore-point id.
2. **Drain / stop writers** if your SLO requires a quiet window (recommended before confirm-drop).
3. **Roll every API replica** to the v0.23.0 binary that treats KV `42P01` as source-gone (write-stop). Do not leave a v0.22.0 API process attached after 125.
4. With `DATABASE_URL` pointing at the admin/maintenance URL:

   ```bash
   cargo run -p edgequake --features postgres -- migrate
   # Expect (expandable-first): applies any expandable migrations that precede
   # the irreversible drop (e.g. 128–130 before 131), then soft-exits 0 with
   # WARN when only irreversible drops remain — so `make_dev` can start.
   # Serving boot soft-allows irreversible-only pending (health still flags
   # migration_required). Hard refuse remains when an irreversible drop blocks
   # later expandables (sqlx cannot skip).
   # Expect (classic): refuses migration 125 without --confirm-drop when 125
   # is next and expandables sit behind it; prints readiness guard
   # and hints `edgequake migrate dry-run` for the full preview.

   cargo run -p edgequake --features postgres -- migrate console
   cargo run -p edgequake --features postgres -- migrate guard
   ```

5. When ready to contract (irreversible):

   ```bash
   cargo run -p edgequake --features postgres -- migrate --confirm-drop
   ```

   sqlx applies pending migrations **106–125** in order. Family SQL backfills (117–124) run first; migration **125** then runs a **verified purge** of presence-conservative KV keys (`staging:hash` / `doc:hash` / `wsdoc` / `injection`) that already exist in typed SSOT, then the durable-row guard. If un-migrated residue remains, **125 aborts** and the DB stays pre-drop for that apply — fix residue, restore if needed, retry. On success, stdout includes per-version `applied …` lines and `KV store dropped (migration 125). Rollback = restore from backup.`

6. **Start v0.23.0 API** with the relational flag matrix above. LD-15: the boot is fail-closed verify-only — since step 5 applied every pending migration, the gate passes; had any migration remained pending, the server would refuse with exit **78** and a `migrate dry-run` / `migrate` hint.
7. **Verify** per tenant/workspace:
   - `GET /health` healthy
   - document list non-empty where seeded
   - query returns grounded sources (not `sources: null` with populated vectors)
   - mm-asset / document asset paths are not 500 (`relation eq_*_kv does not exist`)
   - wipe one workspace → other workspaces and tenants intact
8. Optionally set `EDGEQUAKE_SERVING_FENCE=on` after a successful query proof.

## Boot migration gating (LD-15) — every environment

Server start **never** applies versioned schema. Boot reads `_sqlx_migrations` and:

- **schema behind on expandable migrations** ⇒ exits **78** (`EX_CONFIG`) with pending count + `edgequake migrate dry-run` / `edgequake migrate` + this runbook path;
- **only irreversible drops pending (125/126/131)** ⇒ soft-allows boot with WARN; health still reports `migration_required` until `--confirm-drop` (LD-07);
- **database newer than the binary** ⇒ exits 78 (downgrade protection);
- **up to date** ⇒ serves (reconcile hooks stay read-only probes).

`/health.schema.pending_count` + `migration_required` expose post-boot drift (e.g. a replica still up while the fleet migrated). Spec: [`specs/091-simplify-data-layer/17-boot-migration-gating.md`](../../specs/091-simplify-data-layer/17-boot-migration-gating.md).

### Docker Compose — one-shot migrate service

```yaml
services:
  migrate:
    image: ghcr.io/raphaelmansuy/edgequake:${EDGEQUAKE_VERSION}
    command: ["migrate"]              # add "dry-run" for a preview-only run
    environment:
      DATABASE_URL: postgres://edgequake:${POSTGRES_PASSWORD}@postgres:5432/edgequake
    depends_on:
      postgres: { condition: service_healthy }
    restart: "no"                     # one-shot; exits 0 when schema is current

  api:
    image: ghcr.io/raphaelmansuy/edgequake:${EDGEQUAKE_VERSION}
    depends_on:
      migrate: { condition: service_completed_successfully }
    # …unchanged; no migration-related env var exists anymore
```

Irreversible drops (125/126) on an **upgraded** database still require an explicit one-shot with `command: ["migrate", "--confirm-drop"]` after `dry-run` review — never bake that into a always-on compose service. Fresh installs (zero applied migrations) proceed without the flag: nothing legacy exists to lose.

### Kubernetes — migrate Job before the Deployment rolls

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
          args: ["migrate"]           # or ["migrate", "--confirm-drop"] after dry-run review
          env:
            - name: DATABASE_URL
              valueFrom: { secretKeyRef: { name: edgequake-db, key: url } }
```

Roll order: `kubectl apply` the Job → wait completion → roll the Deployment. New replicas CrashLoopBackOff (exit 78, message in logs) until the Job completes — visible, not silent. Optionally gate readiness on `/health.schema.migration_required == false` from an exec probe for defense in depth.

## Rollback

| Phase | Rollback |
| --- | --- |
| Before `--confirm-drop` completes 125 | Redeploy previous binary; schema additive migrations may remain (expand phase) |
| After 125 applied | **Restore from backup** only — there is no flag-flip rollback for dropped `eq_*_kv` |

## Automated soak (synthetic multi-tenant)

```bash
# Formal realism matrix (SPEC-93): 5 tenants × 3 workspaces × 40 docs × PG16/17/18
make spec93-migration-assessment

# Legacy smoke (tiny corpus, default postgres tag)
make spec091-upgrade-soak
```

**SPEC-93** is the binding proof pack: [`specs/93-migration-assessment/`](../../specs/93-migration-assessment/).  
It pulls `ghcr.io/raphaelmansuy/edgequake:0.22.0` + `edgequake-postgres:0.22.0-pg{16,17,18}`, seeds a realism corpus, dumps the DB, runs v0.23.0 `migrate dry-run` (asserts preview + no schema advance), then `migrate --confirm-drop` through migrations **106–141**, and asserts post-drop isolation / list / wipe / assets / fence-on retrieval. Reports: `specs/93-migration-assessment/reports/` (matrix summary + per-major `verdict.md`).

## Manual soak with a real dump

```bash
# 1) Restore a v0.22.0-era dump into an ephemeral Postgres (same major as source).
# 2) Point DATABASE_URL at it; follow Operator sequence above.
# 3) Run the same HTTP/SQL assertions as the soak script (multi-tenant isolation + wipe).
```

Prefer rehearsing on a staging restore before production confirm-drop.

## Related

- Release/CD: [release-and-cd.md](./release-and-cd.md)
- Cancel/fairness (live today, not SPEC-120): [ingestion-cancel-and-fairness.md](../ingestion-cancel-and-fairness.md)
- SPEC-120 status (orphaned WIP): [`specs/92-task-system/README.md`](../../specs/92-task-system/README.md)
