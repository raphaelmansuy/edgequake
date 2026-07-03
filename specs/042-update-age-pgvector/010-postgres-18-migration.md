# SPEC-042-B — PostgreSQL 18 Migration Runbook

**Spec:** `042-update-age-pgvector` Phase B  
**Date:** 2026-07-03  
**Status:** `PLANNED` → implement in release targeting PG18  
**Trigger:** [Issue #161](https://github.com/raphaelmansuy/edgequake/issues/161) — full closure requires **AGE 1.7.0**  
**Prerequisite:** SPEC-042 Phase A (PG16 pins + M042/M043 bootstrap) ✅

---

## Mission (reformulated — dual-track)

> Upgrade EdgeQuake's PostgreSQL stack to support **PG16 (supported)** and **PG18 (recommended)** concurrently, with optional major migration — not a forced cutover.

### Target stacks

| Tier | PostgreSQL | Extensions |
| ---- | ---------- | ---------- |
| Supported | 16 | pgvector 0.8.3 + AGE 1.6.0 |
| Recommended | 18 | pgvector 0.8.3 + AGE 1.7.0 |

See [012-dual-pg-major-compatibility.md](./012-dual-pg-major-compatibility.md) — **PG16 compatibility is retained.**

### Target stack (PG18 migration only — optional)

| Component | PG16 (Phase A — current default) | PG18 (Phase B — target) |
| --------- | -------------------------------- | ----------------------- |
| PostgreSQL | 16-bookworm | **18-bookworm** |
| pgvector | 0.8.3 | **0.8.3** |
| Apache AGE | 1.6.0 | **1.7.0** (`PG18/v1.7.0-rc0`) |
| Issue #161 | Partial (PG16 ceiling) | **Full closure** |

Official sources:

- [AGE PG18 download](https://age.apache.org/download/) — stable **1.7.0**
- [pgvector 0.8.3](https://pgxn.org/dist/vector)
- [PostgreSQL 18 docs](https://www.postgresql.org/docs/18/index.html)

---

## First principles

1. **PG major bump ≠ extension bump** — you cannot `ALTER EXTENSION` your way from PG16 data directory to PG18. Requires **logical dump** or **`pg_upgrade`**.
2. **AGE is PG-major-specific** — PG16 ships AGE 1.6.0; PG18 ships AGE 1.7.0. Graph data migrates via dump, not in-place extension upgrade across majors.
3. **sqlx migrations are PG-agnostic** — `_sqlx_migrations` rows restore with the dump; bootstrap reconciles indexes (M038/M046/M078) on first PG18 startup.
4. **Bootstrap is the safety net** — M042 (pgvector) + M043 (AGE) run on every startup; same code path on PG18.

---

## Migration procedure (operator runbook)

### Overview

```
┌─────────────┐    pg_dump -Fc     ┌─────────────┐    pg_restore      ┌─────────────┐
│  PG16 vol   │ ─────────────────► │  .dump file │ ────────────────► │  PG18 vol   │
│  AGE 1.6.0  │                    │  (backup)   │   + extensions   │  AGE 1.7.0  │
└─────────────┘                    └─────────────┘                    └─────────────┘
                                                                          │
                                                                          ▼
                                                                   backend bootstrap
                                                                   M042 + M043 + M078
```

### Automated script (recommended)

```bash
# Full procedure with preflight + postflight checks
./scripts/migrate_postgres_major.sh \
  --source-url "$PG16_DATABASE_URL" \
  --target-url "$PG18_DATABASE_URL" \
  --dump-file /tmp/edgequake-pg16-$(date +%Y%m%d).dump
```

E2E proof (dev):

```bash
./specs/042-update-age-pgvector/e2e/run_pg18_migration_procedure.sh
```

---

### Step 0 — Preflight checklist

- [ ] Maintenance window scheduled (estimate: 5–60 min depending on corpus size)
- [ ] `pg_dump` disk space ≥ database size × 1.2
- [ ] PG18 image built: `make postgres-image-build-pg18`
- [ ] Record baseline:

```bash
psql "$PG16_DATABASE_URL" -c "SELECT max(version) FROM _sqlx_migrations;"
psql "$PG16_DATABASE_URL" -c "SELECT extname, extversion FROM pg_extension WHERE extname IN ('vector','age');"
psql "$PG16_DATABASE_URL" -c "SELECT count(*) AS graphs FROM ag_catalog.ag_graph;" 2>/dev/null || true
curl -sf http://localhost:8080/health | jq '.operational.migration'
```

- [ ] Scale API to **1 replica** (avoid concurrent writers during cutover)

---

### Step 1 — Quiesce writers

```bash
make stop
# or kubectl scale deployment edgequake-api --replicas=0
```

Verify no active connections except admin:

```sql
SELECT pid, usename, application_name, state
FROM pg_stat_activity
WHERE datname = 'edgequake' AND pid <> pg_backend_pid();
```

---

### Step 2 — Backup PG16 (mandatory)

```bash
export PG16_DATABASE_URL="${DATABASE_URL}"
export DUMP_FILE="edgequake-pg16-$(date +%Y%m%d-%H%M).dump"

pg_dump -Fc -v -f "$DUMP_FILE" "$PG16_DATABASE_URL"
ls -lh "$DUMP_FILE"
```

**Rollback anchor:** keep this file until PG18 is verified in production for ≥ 7 days.

---

### Step 3 — Start PG18 instance (empty volume)

```bash
# Dev: dedicated port avoids clobbering PG16
export EQ_POSTGRES_PROFILE=pg18
export POSTGRES_PORT=5433
make postgres-image-build-pg18
make db-start-pg18
```

Verify extensions in **image** (before restore):

```bash
bash edgequake/docker/verify-postgres-extensions.sh edgequake-postgres:pg18
```

---

### Step 4 — Prepare target database

```bash
export PG18_DATABASE_URL="postgres://edgequake:edgequake_secret@localhost:5433/edgequake"

psql "$PG18_DATABASE_URL" -c "CREATE EXTENSION IF NOT EXISTS vector;"
psql "$PG18_DATABASE_URL" -c "CREATE EXTENSION IF NOT EXISTS pg_trgm;"
psql "$PG18_DATABASE_URL" -c "CREATE EXTENSION IF NOT EXISTS btree_gin;"
psql "$PG18_DATABASE_URL" -c 'CREATE EXTENSION IF NOT EXISTS "uuid-ossp";'
psql "$PG18_DATABASE_URL" -c "CREATE EXTENSION IF NOT EXISTS age;"
psql "$PG18_DATABASE_URL" -c "LOAD 'age';"
```

---

### Step 5 — Restore dump

```bash
pg_restore -v --no-owner --role=edgequake \
  -d "$PG18_DATABASE_URL" \
  "$DUMP_FILE" \
  2>&1 | tee /tmp/pg18-restore.log
```

**Expected warnings (non-fatal):**

- `extension "vector" already exists`
- `extension "age" already exists`
- Role/owner mismatches (handled by `--no-owner`)

**Fatal errors — stop and investigate:**

- Missing AGE catalog tables after restore
- `_sqlx_migrations` empty when source had rows

---

### Step 6 — Extension catalog alignment

```bash
psql "$PG18_DATABASE_URL" -f edgequake/migrations/support/042/apply.sql
psql "$PG18_DATABASE_URL" -f edgequake/migrations/support/043/apply.sql
```

Verify:

```sql
SELECT extname, extversion,
       (SELECT default_version FROM pg_available_extensions e2 WHERE e2.name = e.extname) AS shipped
FROM pg_extension e
WHERE extname IN ('vector', 'age');
-- Expected: vector 0.8.3, age 1.7.0
```

---

### Step 7 — Point EdgeQuake at PG18 + bootstrap

```bash
export DATABASE_URL="$PG18_DATABASE_URL"
make backend-bg
# Watch migration logs
grep -i 'migration\|042\|043\|078' /tmp/edgequake-backend.log | tail -40
```

Postflight:

```bash
curl -sf http://localhost:8080/ready
curl -sf http://localhost:8080/health | jq '.operational.migration'
```

Expected health fields:

```json
{
  "pgvector_extversion": "0.8.3",
  "age_extversion": "1.7.0",
  "pgvector_iterative_scan_capable": true,
  "ready_for_traffic": true
}
```

---

### Step 8 — Functional verification

```bash
# Graph + vector smoke
cargo test -p edgequake-storage --features postgres --test graph_sota_tests

# Bootstrap idempotency
cargo test -p edgequake-api migration_bootstrap_proof --features postgres

# Full stack (optional)
./specs/039-fix-docker/e2e/run_docker_fresh_install_proof.sh ollama
```

Manual checks:

- [ ] Document list loads
- [ ] Query returns answers
- [ ] Graph stats < 2s (M078 indexes present)
- [ ] Upload + ingest completes

---

### Step 9 — Cutover production

1. Update `DATABASE_URL` in secrets / compose to PG18 endpoint
2. Scale API replicas back up
3. Monitor `/ready` and error rates for 24h
4. Decommission PG16 volume **only after** backup retention policy satisfied

---

## Alternative: pg_upgrade (large DBs)

Use when dump/restore window exceeds SLA.

```bash
# Requires both PG16 and PG18 binaries in upgrade container — ops-managed
pg_upgrade \
  --old-datadir=/var/lib/postgresql/16/data \
  --new-datadir=/var/lib/postgresql/18/data \
  --old-bindir=/usr/lib/postgresql/16/bin \
  --new-bindir=/usr/lib/postgresql/18/bin \
  --check

# Then run extension CREATE + M042/M043 on upgraded cluster
```

**EdgeQuake note:** Test AGE graph integrity after `pg_upgrade` — prefer dump/restore for first production migration.

---

## Rollback procedure

| Situation | Action |
| --------- | ------ |
| Restore fails on PG18 | Fix errors; retry restore — PG16 untouched |
| Bootstrap fails on PG18 | Keep PG16 running; revert `DATABASE_URL` |
| Data corruption suspected | Stop PG18; restore PG16 from `$DUMP_FILE` via pg_restore to fresh PG16 instance |
| Partial cutover | DNS/secret rollback to PG16 URL |

**There is no PostgreSQL major downgrade.** Rollback = restore backup to PG16 cluster.

---

## Timeline estimates

| Corpus size | pg_dump | pg_restore | Bootstrap | Total |
| ----------- | ------- | ---------- | --------- | ----- |
| Dev (<1 GB) | <1 min | <1 min | <2 min | ~5 min |
| Small prod (1–10 GB) | 2–10 min | 5–15 min | 2–5 min | ~30 min |
| Large graph (>100k nodes) | 10+ min | 15+ min | 5–30 min (M078) | 1h+ |

---

## Implementation checklist (engineering)

- [ ] `Dockerfile.postgres.pg18` builds and verifies
- [ ] `extension-pins.sh` PG18 profile (`EQ_POSTGRES_PROFILE=pg18`)
- [ ] `make postgres-image-build-pg18` + `make db-start-pg18`
- [ ] `scripts/migrate_postgres_major.sh` procedural SSOT
- [ ] `e2e/run_pg18_migration_procedure.sh` green in CI (optional nightly)
- [ ] Default compose switch (`docker-compose.yml` → PG18) in release tag
- [ ] CHANGELOG + README update
- [ ] Close #161 with PG18 evidence

---

## Related

| Doc | Purpose |
| --- | ------- |
| [011-postgres-18-upgrade-path-matrix.md](./011-postgres-18-upgrade-path-matrix.md) | All upgrade paths |
| [007-risk-analysis.md](./007-risk-analysis.md) | R-09 large graph AGE 1.7 |
| [008-implementation-plan.md](./008-implementation-plan.md) | Phase B engineering steps |
