# Root Cause Analysis — Issue #195
## Migration checksum mismatch after upgrade to 0.11.0

**Classification:** CRITICAL / Production-blocking regression  
**Reporter:** @akashs-devops  
**Fixed in:** fix/issue-195-migration-checksum  
**Affected upgrades:** 0.10.12 → 0.11.0  

---

## 1. Symptom

```
migration 1 was previously applied but has been modified
```

Application refuses to start after upgrading the container image from
`ghcr.io/raphaelmansuy/edgequake:0.10.12` to `ghcr.io/raphaelmansuy/edgequake:0.11.0`.

---

## 2. First Principle Investigation

### How SQLx migration checksums work

SQLx computes a **SHA-384** hash of the raw bytes of each `.sql` migration file at
the time it is applied and stores it in the `_sqlx_migrations` table:

```sql
SELECT version, checksum, description FROM _sqlx_migrations ORDER BY version;
```

When the application starts, SQLx re-reads every migration file on disk, recomputes
the SHA-384 and compares it with the stored value.  If they differ, SQLx aborts with:

```
migration N was previously applied but has been modified
```

**This is by design** — it is a safety guard that prevents accidental data corruption
from silent schema drift.

### What changed in v0.11.0

Commit `e91108df` (squash merge of PR #193, v0.11.0 release) mutated
`edgequake/migrations/001_init_database.sql` in two ways:

| Line | Before (0.10.12) | After (0.11.0) |
|------|-----------------|----------------|
| 21 | `-- in the public schema, not in the user's schema (edgequake)` | `-- in the public schema, not in the user's schema (edgequake).` (trailing period added) |
| 22 | `SET search_path = public;` | *(4 comment lines inserted)* |
| 26 | *(blank)* | `SET LOCAL search_path = public;` |

Specifically, the diff was:

```diff
 -- CRITICAL: Set search_path to public FIRST to ensure all tables are created
--- in the public schema, not in the user's schema (edgequake)
-SET search_path = public;
+-- in the public schema, not in the user's schema (edgequake).
+-- WHY LOCAL: SET LOCAL scopes this change to the current transaction only.
+-- Session-level SET would pollute sqlx-cli's connection state and cause it
+-- to write migration tracking records to the wrong _sqlx_migrations table
+-- on subsequent runs (edgequake schema vs public schema mismatch).
+SET LOCAL search_path = public;
```

### Checksum delta

| Version | SHA-384 |
|---------|---------|
| 0.10.12 (expected by deployed DBs) | `bb40c61f7d5cbeafa7827f2e8878588464e814ed118179bdb3c16e92880c79b2a3db4c92240e632bc8355363a768daf2` |
| 0.11.0 (broken, on-disk) | `9e44513e1b22ab482a3703f394d1f0e35fe24625b77eca236789a3b702bbf6c1ceb9ed8beed4e13c9c4ca4b28feae925` |

Any PostgreSQL database that ran the 0.10.12 migrations has the first checksum stored.
The 0.11.0 binary computed the second checksum, found a mismatch, and refused to start.

---

## 3. Why the 0.11.0 change was unnecessary

The original intent of the `SET LOCAL` change (commit `53703612`, also in 0.11.0) was
to fix a **sqlx-cli schema ambiguity bug** where:

1. `001_init_database.sql` created schema `edgequake`
2. On the second run of `sqlx migrate run`, the PostgreSQL `$user` path resolved to
   the `edgequake` schema (because the connected user is named `edgequake`)
3. `sqlx-cli` found (or created) `_sqlx_migrations` in the `edgequake` schema
4. The session-level `SET search_path = public` inside migration 001 then redirected
   subsequent tracking writes to `public._sqlx_migrations` — causing duplicate-key
   errors

**However, the proper fix for this was already applied in the same commit:**

```
DATABASE_URL includes ?options=-c%20search_path%3Dpublic
```

This sets `search_path = public` at the **connection level** for every connection,
including sqlx-cli connections. The `SET LOCAL` change inside migration 001 became
redundant and its only effect was to break checksums for all deployed databases.

---

## 4. Impact

| Scope | Detail |
|-------|--------|
| Affected deployments | Any persistent PostgreSQL DB that ran 0.10.12 migrations |
| Deployment types | AWS ECS, Kubernetes, Docker Compose with external DB, bare-metal |
| Severity | Production-blocking — application cannot start |
| Workaround | Drop `_sqlx_migrations` table and re-apply all migrations (data loss risk) |
| Safe fix | Restore migration 001 to pre-v0.11.0 byte-exact content |

---

## 5. Fix

See [fix_specification.md](./fix_specification.md).

---

## 6. Prevention

See [prevention_playbook.md](./prevention_playbook.md).
