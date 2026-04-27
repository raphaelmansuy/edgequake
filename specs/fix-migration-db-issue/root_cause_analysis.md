# Root Cause Analysis — Migration 019 Checksum Mismatch

> Cross-ref: [README.md](README.md) | [reproduction_guide.md](reproduction_guide.md) | [prevention_playbook.md](prevention_playbook.md)

---

## 1. First-Principles: How SQLx Migration Integrity Works

SQLx uses a content-addressed migration system. Every migration file is
fingerprinted with SHA-384 when first applied and the hash is stored in
`_sqlx_migrations`. On every subsequent startup the hash is re-computed
from the file on disk and compared to the stored value.

```
  Migration file                        _sqlx_migrations table
  +-------------------------------+     +-----------------------------------+
  | 019_add_tenant_...tasks.sql   |     | version | checksum (48 bytes)    |
  |                               |     +---------+-------------------+-----+
  | [raw UTF-8 bytes]             |     |      19 | SHA-384(file v0.10.1) |
  +-------------------------------+     +-----------------------------------+
          |                                       |
          | sha384(raw bytes)                     |
          v                                       |
      SHA-384 hash   <---  COMPARE  ----------->
          |
          +-- MATCH   -> proceed normally
          +-- MISMATCH -> ABORT with:
                  "migration 19 was previously applied
                   but has been modified"
```

SQLx algorithm (Rust library source, sqlx-core/src/migrate/migrate.rs):

```
1. Read file bytes
2. checksum = sha384(bytes)
3. SELECT checksum FROM _sqlx_migrations WHERE version = N
4. IF stored != computed:
     return Err(MigrateError::VersionMismatch(version))
```

---

## 2. Timeline of the Bug

```
Date         Version  Event
-----------  -------  -------------------------------------------------------
2026-01-28   v0.10.1  Migration 019 created (GOOD content, hash A stored)
                      SHA-384 = 1f538faa...54eb
                      Content: STEP 3 comment is "Add constraints"
                               NO ALTER TABLE SET DEFAULT lines

2026-04-19   v0.10.6  Commit 6f3d0204 modifies migration 019 IN-PLACE:
                      - Comment changed to "Add safe defaults and constraints"
                      - 7 new lines added (ALTER TABLE ... SET DEFAULT)
                      SHA-384 = 7b544306...37a
                      Migration 035 was also created with the same DEFAULT logic

                      *** BUG INTRODUCED ***
                      File on disk: hash B (7b544306...)
                      DB for users who ran v0.10.1: hash A (1f538faa...)
                      Hash A != Hash B -> STARTUP ERROR

2026-04-27   fix      Migration 019 restored to byte-for-byte v0.10.1 content
                      SHA-384 = 1f538faa...54eb (SAME AS v0.10.1)
                      *** BUG FIXED ***
```

---

## 3. The Exact Mutation

Commit `6f3d0204` changed the following lines in migration 019:

```
BROKEN (v0.10.6+, lines 37-46):              GOOD (v0.10.1, lines 37-38):
-------------------------------------         --------------------------------
-- STEP 3: Add safe defaults and             -- STEP 3: Add constraints
--         constraints                       -- ===========================
-- ===========================
                                             [no ALTER TABLE SET DEFAULT]
-- WHY: A small number of older
-- maintenance/test paths may still
-- omit these columns on insert.
-- Route those rows into a deterministic
-- sentinel tenant and workspace instead
-- of failing with NULL violations or
-- leaking across tenants.
ALTER TABLE tasks
ALTER COLUMN tenant_id SET DEFAULT
  '00000000-...-0000'::UUID,
ALTER COLUMN workspace_id SET DEFAULT
  '00000000-...-0000'::UUID;
```

---

## 4. Checksums

| Version | File content | SHA-384 |
|---------|-------------|---------|
| v0.10.1 | Original (GOOD) | `1f538faa36762ad72045e0056d2783179b6f9e33c093fbe48c2222d5a1dba00364c7de38a5e7ec0449db4927f51a54eb` |
| v0.10.6+ | Mutated (BROKEN) | `7b544306c5da16b05ec0607aa81b356055e82f4247705f98b5b036c26ab2cf1c5910d2a062309c6fedbd44e0eb54437a` |
| Fixed | Restored to v0.10.1 | `1f538faa36762ad72045e0056d2783179b6f9e33c093fbe48c2222d5a1dba00364c7de38a5e7ec0449db4927f51a54eb` |

Verification command:
```bash
sha384sum edgequake/migrations/019_add_tenant_workspace_to_tasks.sql
# Must output: 1f538faa...54eb
```

---

## 5. Why Migration 019 Was Mutated

The `ALTER TABLE tasks ALTER COLUMN tenant_id SET DEFAULT` logic was correct
and necessary. However, it was added to the wrong place:

```
WRONG: edit migration 019 (already deployed)
  +-- Changes stored checksum in new binaries
  +-- Does not match stored checksum in existing databases
  +-- Causes startup failure for all upgrade users

RIGHT: add the logic to a NEW migration (035)
  +-- New migration = new version number
  +-- New checksum slot in _sqlx_migrations
  +-- No conflict with existing checksums
  +-- Existing databases: migration 035 runs once and succeeds
```

Migration 035 (`035_harden_task_compatibility_defaults.sql`) already
contains the correct `SET DEFAULT` logic. The mutation to 019 was redundant
AND harmful.

---

## 6. Affected Users

Any installation that:
1. Ran edgequake **v0.10.1** (or any version before the mutation)
2. AND upgrades to any version from **v0.10.2 through v0.10.12**

is affected.

Fresh installs on v0.10.2+ are NOT affected (they store hash B from the
start, and the broken file and DB are consistent with each other).

```
Affected path:
  v0.10.1 install ---upgrade---> v0.10.2..v0.10.12 = FAILS

Not affected:
  Fresh v0.10.2+ install = OK (hash B stored, hash B on disk: match)
  Fresh install with fix  = OK (hash A stored, hash A on disk: match)
```

---

## 7. Why This Is a GOLDEN RULE Violation

**Golden Rule: Never modify a migration file after it has been deployed.**

SQLx enforces this rule by design. The error message is explicit:

> "migration N was previously applied but has been modified"

This rule exists because a migration file represents a specific point-in-time
schema change. Once applied, the database schema reflects the migration's
content. Changing the file afterward does not retroactively change the schema,
but it does break the integrity check that guarantees all deployments have
the same history.

See [prevention_playbook.md](prevention_playbook.md) for how to make this
class of bug impossible.

---

## Cross-references

- Reproduction steps: [reproduction_guide.md](reproduction_guide.md)
- Prevention: [prevention_playbook.md](prevention_playbook.md)
- Proof script: [sqlx_checksum_proof.py](sqlx_checksum_proof.py)
