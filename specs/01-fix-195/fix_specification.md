# Fix Specification — Issue #195
## Migration checksum mismatch after upgrade to 0.11.0

**Branch:** `fix/issue-195-migration-checksum`  
**Type:** Bug fix (no new features, no schema changes)  
**Risk:** Low — restores exact prior behaviour  

---

## Change

### File: `edgequake/migrations/001_init_database.sql`

Restore the file to its **byte-exact pre-v0.11.0 content**, undoing the mutation
introduced in commit `e91108df`.

**Revert:**

```diff
 -- CRITICAL: Set search_path to public FIRST to ensure all tables are created
--- in the public schema, not in the user's schema (edgequake).
--- WHY LOCAL: SET LOCAL scopes this change to the current transaction only.
--- Session-level SET would pollute sqlx-cli's connection state and cause it
--- to write migration tracking records to the wrong _sqlx_migrations table
--- on subsequent runs (edgequake schema vs public schema mismatch).
-SET LOCAL search_path = public;
+-- in the public schema, not in the user's schema (edgequake)
+SET search_path = public;
```

After this change the on-disk SHA-384 will match the value stored in every
production database that ran 0.10.12 migrations:

```
bb40c61f7d5cbeafa7827f2e8878588464e814ed118179bdb3c16e92880c79b2a3db4c92240e632bc8355363a768daf2
```

---

## Why `SET search_path` (session-level) is safe here

The `SET LOCAL` change was motivated by a concern that session-level `SET search_path`
inside a migration might pollute `sqlx-cli`'s connection state on subsequent runs.

This concern is **already addressed** at the `DATABASE_URL` level:

```
DATABASE_URL=postgres://…?options=-c%20search_path%3Dpublic
```

This ensures every connection — including `sqlx-cli` — starts with `search_path=public`
before any migration SQL is executed.  The `SET search_path = public;` inside
migration 001 therefore acts as a belt-and-suspenders guard, not the primary
mechanism.

Additionally, the compiled application binary uses an `after_connect` hook in
`edgequake/crates/edgequake-api/src/state/postgres.rs` that sets `search_path = public`
for every pool connection.

---

## No new migration required

This fix does **not** add a new migration file.  Adding a new migration would:

1. Not solve the checksum mismatch for migration 001
2. Add unnecessary schema-change noise to production upgrade logs
3. Create a permanent record of an accidental change

The correct fix is to restore the immutable file to its original state.

---

## Verification steps

```bash
# 1. Confirm restored SHA-384 matches expected
sha384sum edgequake/migrations/001_init_database.sql
# Expected: bb40c61f7d5cbeafa7827f2e8878588464e814ed118179bdb3c16e92880c79b2a3db4c92240e632bc8355363a768daf2

# 2. Build the binary (no compile errors)
cargo build -p edgequake 2>&1 | tail -5

# 3. On a test DB that ran 0.10.12 migrations, run migrations with the fix
sqlx migrate run --database-url "$DATABASE_URL"
# Expected: "Applied 0 migrations" (all checksums match, nothing to do)
```

---

## Affected users

All users who deployed 0.10.12 and are trying to upgrade to 0.11.0 with a
**persistent PostgreSQL database** (ECS, Kubernetes, Docker Compose with external DB).

Users who performed a fresh install with 0.11.0 are not affected.

---

## Release notes entry (for CHANGELOG)

```markdown
### Fixed
- **CRITICAL**: Restore migration 001 checksum compatibility — `001_init_database.sql`
  was inadvertently mutated in v0.11.0 causing a SHA-384 mismatch that prevented
  application startup for any database upgraded from v0.10.12.
  Fixes [#195](https://github.com/raphaelmansuy/edgequake/issues/195).
```
