# Reproduction Guide: Migration Database Issues

> **Cross-refs:** See [README.md](README.md) for index, [root_cause_analysis.md](root_cause_analysis.md) for analysis
> **Status:** Both bugs fixed and reproduced per steps below

---

## Overview

This guide documents two independent bugs, both confirmed reproduced and fixed via Docker:

| Bug | Symptom | Root Cause | Fix |
|-----|---------|-----------|-----|
| **Bug 1** | `migration 19 was previously applied but has been modified` | Migration 019 mutated post-deploy in commit `6f3d0204` | Restored to v0.10.1 content |
| **Bug 2** | `duplicate key value violates unique constraint '_sqlx_migrations_pkey'` | sqlx-cli uses wrong schema for `_sqlx_migrations` on second run | Added `?options=-c%20search_path%3Dpublic` to DATABASE_URL |

---

## Prerequisites

```
  +------------------+      +------------------+      +------------------+
  |   Docker         |      |   sqlx-cli       |      |  Rust toolchain  |
  |  (OrbStack OK)   |      |  v0.8.x          |      |  (cargo build)   |
  +------------------+      +------------------+      +------------------+
```

```bash
# OrbStack users: set socket path
export DOCKER_HOST=unix:///Users/<you>/.orbstack/run/docker.sock

# Verify sqlx-cli
sqlx --version          # >= 0.8.0
```

---

## Bug 1: Migration 019 Checksum Mismatch

### Setup — Fresh DB

```bash
export DOCKER_HOST=unix:///Users/<you>/.orbstack/run/docker.sock

docker run -d --name mig-bug1 \
  -e POSTGRES_USER=edgequake \
  -e POSTGRES_PASSWORD=edgequake_secret \
  -e POSTGRES_DB=edgequake \
  -p 5499:5432 \
  pgvector/pgvector:pg16

sleep 5   # wait for PostgreSQL to be ready

export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5499/edgequake?options=-c%20search_path%3Dpublic"
cd edgequake   # top-level Rust workspace
```

### Step A — Reproduce "broken" scenario (v0.10.1 then v0.10.12)

```
  Upgrade timeline:
  +-------+          +----------+           +----------+
  | v0.10.1 install | | v0.10.6 mutates    | | v0.10.12 deploy |
  | mig 019 stored  | | 019 file changed   | | hash A != hash B |
  | hash A in DB    | | extra ALTER TABLE  | | STARTUP FAILS    |
  +-------+          +----------+           +----------+
```

```bash
# 1. Apply broken migration file (simulates v0.10.1 on-disk state)
git show 6f3d0204:edgequake/migrations/019_add_tenant_workspace_to_tasks.sql \
  > /tmp/019_broken.sql

# Save the current (fixed) file
cp edgequake/migrations/019_add_tenant_workspace_to_tasks.sql /tmp/019_fixed.sql

# 2. Swap in broken version (simulates v0.10.1 binary's file at first install)
cp /tmp/019_broken.sql edgequake/migrations/019_add_tenant_workspace_to_tasks.sql

# 3. Run migrations — simulates v0.10.1 writing hash A into DB
sqlx migrate run --source edgequake/migrations
echo "v0.10.1 migrations completed, hash stored in DB"

# 4. Restore mutated file (simulates upgrading to v0.10.12 binary)
cp /tmp/019_fixed.sql edgequake/migrations/019_add_tenant_workspace_to_tasks.sql
# NOTE: after our fix, the "fixed" file IS the v0.10.1 content, so no mismatch

# 5. Attempt second run (simulates v0.10.12 startup)
sqlx migrate run --source edgequake/migrations
echo "If this fails with 'modified': BUG REPRODUCED"
```

### Step B — Verify fix

```bash
# 1. Drop and recreate DB
docker rm -f mig-bug1
docker run -d --name mig-bug1 \
  -e POSTGRES_USER=edgequake \
  -e POSTGRES_PASSWORD=edgequake_secret \
  -e POSTGRES_DB=edgequake \
  -p 5499:5432 \
  pgvector/pgvector:pg16 && sleep 5

# 2. Run with RESTORED migration 019 (byte-for-byte v0.10.1 content)
sqlx migrate run --source edgequake/migrations
echo "Exit 0 = PASS"

# 3. Verify checksum matches v0.10.1
sha384sum edgequake/migrations/019_add_tenant_workspace_to_tasks.sql
# Must output: 1f538faa36762ad72045e0056d2783179b6f9e33c093fbe48c2222d5a1dba00364c7de38a5e7ec0449db4927f51a54eb
```

---

## Bug 2: sqlx-cli Schema Ambiguity (Second-Run Duplicate Key)

### The Race Condition

```
  SECOND RUN of sqlx migrate run (without the fix)
  ================================================

  1. sqlx-cli connects as user=edgequake
     search_path = "$user",public = edgequake,public  (edgequake schema exists!)

  2. sqlx looks for _sqlx_migrations:
     - Checks edgequake._sqlx_migrations → does NOT exist
     - Creates edgequake._sqlx_migrations (empty!)

  3. sqlx sees 0 applied migrations (wrong table!) → tries to re-apply all

  4. Migration 001 runs: "SET search_path = public" (SESSION level, old code)
     Now session search_path = public

  5. sqlx tries to write tracking record to _sqlx_migrations
     → resolves to public._sqlx_migrations (already has ALL records from run 1!)
     → DUPLICATE KEY ERROR

  Time: O(1) — fails immediately on restart
```

```
  WITH THE FIX: ?options=-c%20search_path%3Dpublic in DATABASE_URL
  ================================================================

  1. sqlx-cli connects with connection option: SET search_path = public
     search_path = public  (edgequake schema is bypassed entirely)

  2. sqlx looks for _sqlx_migrations:
     - Checks public._sqlx_migrations → EXISTS (from run 1)
     - Reads 34 records → all already applied

  3. sqlx sees 34/34 applied → NOTHING TO DO → exits 0

  No collision. No ambiguity. O(1) safe restart.
```

### Setup — Fresh DB

```bash
export DOCKER_HOST=unix:///Users/<you>/.orbstack/run/docker.sock

docker run -d --name mig-bug2 \
  -e POSTGRES_USER=edgequake \
  -e POSTGRES_PASSWORD=edgequake_secret \
  -e POSTGRES_DB=edgequake \
  -p 5499:5432 \
  pgvector/pgvector:pg16 && sleep 5
```

### Step A — Reproduce the bug (without fix)

```bash
# Use URL WITHOUT the options parameter
export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5499/edgequake"
cd edgequake

echo "=== RUN 1 (fresh DB) ==="
sqlx migrate run --source migrations
echo "Run 1 exit: $?"   # should be 0

echo ""
echo "=== RUN 2 (simulate restart — SHOULD FAIL without fix) ==="
sqlx migrate run --source migrations
echo "Run 2 exit: $?"   # was non-zero before fix: "duplicate key value violates unique constraint"
```

### Step B — Prove the fix

```bash
# Drop and recreate
docker rm -f mig-bug2
docker run -d --name mig-bug2 \
  -e POSTGRES_USER=edgequake \
  -e POSTGRES_PASSWORD=edgequake_secret \
  -e POSTGRES_DB=edgequake \
  -p 5499:5432 \
  pgvector/pgvector:pg16 && sleep 5

# Use URL WITH the search_path option
export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5499/edgequake?options=-c%20search_path%3Dpublic"
cd edgequake

echo "=== RUN 1 (fresh DB + fix) ==="
sqlx migrate run --source migrations
echo "Run 1 exit: $?"   # 0

echo ""
echo "=== RUN 2 (restart simulation + fix — MUST EXIT 0) ==="
sqlx migrate run --source migrations 2>&1
echo "Run 2 exit: $?"   # 0, no output = PASS

# Verify only public._sqlx_migrations exists (no ghost edgequake table)
PGPASSWORD=edgequake_secret psql \
  -h localhost -p 5499 -U edgequake -d edgequake \
  -c "SELECT schemaname, tablename FROM pg_tables WHERE tablename='_sqlx_migrations';"
# Expected: only 1 row: schemaname=public
```

### Expected Output

```
=== RUN 1 (fresh DB + fix) ===
Applied 1/migrate init database (...)
Applied 2/migrate add tasks table (...)
...
Applied 35/migrate harden task compatibility defaults (...)
Run 1 exit: 0

=== RUN 2 (restart simulation + fix — MUST EXIT 0) ===

Run 2 exit: 0

 schemaname | tablename
------------+------------------
 public     | _sqlx_migrations
(1 row)
```

---

## Combined Proof (Both Bugs)

```bash
export DOCKER_HOST=unix:///Users/<you>/.orbstack/run/docker.sock

docker run -d --name mig-proof \
  -e POSTGRES_USER=edgequake \
  -e POSTGRES_PASSWORD=edgequake_secret \
  -e POSTGRES_DB=edgequake \
  -p 5499:5432 \
  pgvector/pgvector:pg16 && sleep 5

export DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5499/edgequake?options=-c%20search_path%3Dpublic"
cd edgequake

# Run 1: all 35 migrations applied cleanly
sqlx migrate run --source migrations && echo "PROOF A: PASS"

# Run 2: no error = schema ambiguity is fixed
sqlx migrate run --source migrations 2>&1 | wc -c | xargs echo "PROOF B output bytes (0=clean):"

# SHA-384 check: migration 019 is back to v0.10.1 value
sha384sum migrations/019_add_tenant_workspace_to_tasks.sql | \
  grep "1f538faa36762ad72045e0056d2783179b6f9e33c093fbe48c2222d5a1dba00364c7de38a5e7ec0449db4927f51a54eb" \
  && echo "PROOF C: migration 019 checksum correct — PASS"

docker rm -f mig-proof
```

---

## Clean Up

```bash
export DOCKER_HOST=unix:///Users/<you>/.orbstack/run/docker.sock
docker rm -f mig-bug1 mig-bug2 mig-proof 2>/dev/null || true
```
