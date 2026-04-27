# Prevention Playbook: Migration Immutability

> **Cross-refs:** See [README.md](README.md), [root_cause_analysis.md](root_cause_analysis.md)
> **Purpose:** Make this class of bug impossible to ship again

---

## Golden Rule

```
  +-----------------------------------------------------------+
  |  GOLDEN RULE: A migration file that has been deployed    |
  |  to ANY environment is PERMANENTLY IMMUTABLE.            |
  |                                                           |
  |  The file bytes — including whitespace, comments, order  |
  |  of statements — must never change after first deploy.   |
  |                                                           |
  |  If logic must change: create a NEW migration.           |
  +-----------------------------------------------------------+
```

SQLx (and all other migration runners: Flyway, Liquibase, Alembic, goose) detect
mutations via checksum comparison. There is no override flag. The only recovery
is a manual DB intervention. The only prevention is discipline.

---

## What We Fixed

| File | Change | Why |
|------|--------|-----|
| `migrations/019_add_tenant_workspace_to_tasks.sql` | Reverted to v0.10.1 byte-for-byte content | File was mutated in commit `6f3d0204`; old checksum stored in existing DBs |
| `migrations/001_init_database.sql` line 23 | `SET search_path = public` → `SET LOCAL search_path = public` | Session-level SET caused sqlx-cli to use wrong `_sqlx_migrations` table on restart |
| `Makefile` `DEFAULT_DATABASE_URL` | Added `?options=-c%20search_path%3Dpublic` | Forces sqlx-cli to use `public` schema for tracking table, preventing duplicate-key on restart |
| `.env` `DATABASE_URL` | Added `?options=-c%20search_path%3Dpublic` | Same reason — affects developer `sqlx migrate run` invocations |
| `.env.example` comment | Updated to show the correct DATABASE_URL format | Documentation |

---

## Rules for Writing New Migrations

### Rule 1 — Use SET LOCAL, never SET

```sql
-- WRONG: session-level, pollutes connection state used by sqlx tracking
SET search_path = public;

-- CORRECT: transaction-scoped, reverts after the migration transaction commits
SET LOCAL search_path = public;
```

**Why:** sqlx-cli runs each migration in a transaction. A session-level `SET` persists
after the transaction commits and changes which schema the tracking table resolves to
on subsequent migrations or runs.

### Rule 2 — Never modify a file after deploy

```
  Timeline:
  
  commit A: create migrations/NNN_my_feature.sql   <- OK to edit (not yet deployed)
  deploy to staging                                 <- CHECKPOINT: file is now immutable
  deploy to prod                                    <- DOUBLE CHECKPOINT
  
  commit B: edit migrations/NNN_my_feature.sql     <- FORBIDDEN after staging deploy
```

**If you need to change logic:** Create `NNN+1_fix_my_feature.sql` with the corrective SQL.
Use `ALTER TABLE`, `CREATE INDEX CONCURRENTLY`, `DROP COLUMN`, etc. — not file edits.

### Rule 3 — Checksums are your contract with production DBs

Each deployed migration file writes a SHA-384 fingerprint into `public._sqlx_migrations`.
Every server start re-computes the fingerprint from disk and compares. Mutation = crash.

```
  public._sqlx_migrations:
  +--------+----------------------------+-----------------------------+
  | version| description                | checksum (SHA-384, 48 bytes)|
  +--------+----------------------------+-----------------------------+
  | 19     | add tenant workspace ...   | 1f538faa36762ad7...         |
  +--------+----------------------------+-----------------------------+
          ^                                        ^
          |                                        |
    permanent record                  must match file bytes forever
```

### Rule 4 — Test idempotency: run migrations twice on CI

Every CI pipeline must:
1. Start a fresh DB
2. Run `sqlx migrate run` (all migrations applied)
3. Run `sqlx migrate run` a second time (should exit 0 silently)

Step 3 catches:
- Missing `IF NOT EXISTS` guards
- Duplicate key bugs
- Schema ambiguity (wrong `_sqlx_migrations` table used on second run)

---

## CI Checklist for New Migrations

Before merging any PR that adds or modifies a migration:

```
- [ ] New file: numbered higher than all existing files
- [ ] No existing file was edited (check: git diff --name-only migrations/)
- [ ] File uses SET LOCAL search_path (not session-level SET)
- [ ] SQL is idempotent where possible (CREATE TABLE IF NOT EXISTS, etc.)
- [ ] Two-run test passes:
        sqlx migrate run --source migrations   # exit 0
        sqlx migrate run --source migrations   # exit 0, no output
- [ ] SHA-384 of new file noted in PR description for traceability
```

---

## Automated SHA-384 Verification Script

Use `specs/fix-migration-db-issue/sqlx_checksum_proof.py` in CI to validate
that deployed migration files have not drifted:

```bash
# Run the proof script (requires Python 3.8+, no dependencies)
python3 specs/fix-migration-db-issue/sqlx_checksum_proof.py

# Expected output when files are correct:
# CHECK 1 PASS - ...
# ...
# ALL CHECKS PASSED
```

Add to your CI pipeline after checkout:

```yaml
- name: Verify migration checksums
  run: python3 specs/fix-migration-db-issue/sqlx_checksum_proof.py
```

---

## Emergency Recovery (Production DB Already Broken)

If a user hits the checksum mismatch error in production:

```
  SCENARIO: migration 19 was previously applied but has been modified
  
  DB state: public._sqlx_migrations has OLD checksum for migration 019
  Disk:     migrations/019_...sql has NEW checksum
  
  Option 1 (recommended): Deploy the FIXED binary (with v0.10.1 file content)
  
  Option 2 (manual hotfix — use only if Option 1 unavailable):
```

```sql
-- Connect to the production DB
-- Re-compute the NEW checksum and update the tracking record

-- Step 1: verify the stored checksum
SELECT version, description, encode(checksum, 'hex') as checksum_hex
FROM public._sqlx_migrations
WHERE version = 19;

-- Step 2: update to match the CURRENT file bytes (only if you intend to keep the mutated file)
-- WARNING: Only do this if you fully understand the SQL difference and accept it.
-- PREFERRED: deploy the reverted binary instead.
UPDATE public._sqlx_migrations
SET checksum = decode('NEW_SHA384_HEX_HERE', 'hex')
WHERE version = 19;
```

**Always prefer** deploying a binary with the correct file over manual DB surgery.

---

## Design Patterns That Prevent Checksum Bugs

### Pattern A — Additive migrations only

```sql
-- In new migration NNN+1:
-- Instead of modifying NNN's intended effect, add to it.
ALTER TABLE tasks
  ALTER COLUMN tenant_id SET DEFAULT '00000000-0000-0000-0000-000000000000'::UUID;
```

### Pattern B — New migration for schema corrections

```
migrations/
  019_add_tenant_workspace_to_tasks.sql       <- IMMUTABLE (v0.10.1 content)
  035_harden_task_compatibility_defaults.sql  <- Additive fix in new file
```

The actual fix for the original intent (adding DEFAULT values) was correctly placed
in migration 035. The checksum bug happened because 019 was edited to include that
same logic redundantly.

### Pattern C — Use DATABASE_URL connection options, not session SETs

```
  Application (after_connect hook):
  - Sets: SET search_path TO public
  - Scope: all application pool connections

  sqlx-cli (DATABASE_URL option):
  - Sets: ?options=-c%20search_path%3Dpublic
  - Scope: all sqlx-cli connections

  Migration SQL (SET LOCAL):
  - Sets: SET LOCAL search_path = public
  - Scope: current transaction only
```

Three layers, all pointing at `public`, no ambiguity.

---

## Monitoring & Alerting

For production deployments, capture the startup error and alert immediately:

```rust
// In server startup code, catch MigrateError::VersionMismatch
match sqlx::migrate!("./migrations").run(&pool).await {
    Err(sqlx::migrate::MigrateError::VersionMismatch(v)) => {
        tracing::error!(
            version = v,
            "CRITICAL: migration {} checksum mismatch — file was modified after deploy. \
             Deploy the correct binary or restore the migration file.",
            v
        );
        std::process::exit(1);
    }
    Err(e) => { tracing::error!("Migration error: {}", e); std::process::exit(1); }
    Ok(_) => tracing::info!("Migrations applied successfully"),
}
```

The error message should include the version number so on-call engineers know
exactly which file to investigate.
